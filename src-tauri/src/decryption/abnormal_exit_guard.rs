//! Garde-fou de dernier recours pour `PolarProxySubsystem` — même pattern Drop-based que
//! `attribution::server::AbnormalExitGuard`. Extrait de `subsystem.rs` (audit EPIC 4, point 6 —
//! limite de taille de fichier) : porte aussi le retry borné de `nft_clear_redirect` (point 2)
//! et la remise à `false` de l'état `active` (point 3) — un échec transitoire de `pkexec`/
//! `vitrail-helper` ne doit jamais laisser le trafic bloqué indéfiniment sans plusieurs
//! tentatives, ni laisser `is_active()` mentir sur l'état réel du process.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::helper_backend::HelperBackend;

/// Nombre maximal de tentatives de `nft_clear_redirect` depuis le garde-fou — même valeur que
/// `killswitch::sequence::STEP_MAX_ATTEMPTS` (EPIC 7).
const ABNORMAL_EXIT_CLEAR_MAX_ATTEMPTS: u32 = 3;
/// Backoff fixe entre deux tentatives — même pattern que `killswitch::sequence::
/// STEP_RETRY_BACKOFF`.
const ABNORMAL_EXIT_CLEAR_RETRY_BACKOFF: Duration = Duration::from_millis(200);

/// Filet de sécurité de dernier recours : si le thread de garde se termine SANS que
/// `clean_shutdown` ait été positionné par `stop()`, `Drop` retire la redirection nftables (avec
/// retry, point 2) avant que quoi que ce soit d'autre ne se passe, ET remet `active` à `false`
/// (point 3) — `is_active()` ne doit jamais continuer d'affirmer `true` après une mort anormale.
pub struct AbnormalExitGuard {
    pub clean_shutdown: Arc<AtomicBool>,
    pub active: Arc<AtomicBool>,
    pub redirect: Arc<dyn HelperBackend>,
}

impl Drop for AbnormalExitGuard {
    fn drop(&mut self) {
        if self.clean_shutdown.load(Ordering::SeqCst) {
            return;
        }
        tracing::error!(
            "decryption: PolarProxy terminé ANORMALEMENT — retrait immédiat de la redirection \
             nftables (garde-fou anti-blackhole, PLAN.md §6nonies 4.2)"
        );
        if let Err(error) = clear_redirect_with_retry(self.redirect.as_ref()) {
            tracing::error!(
                error = %error,
                attempts = ABNORMAL_EXIT_CLEAR_MAX_ATTEMPTS,
                "decryption: ÉCHEC DÉFINITIF du retrait de la redirection nftables par le \
                 garde-fou après plusieurs tentatives — LE TRAFIC WEB DE LA MACHINE PEUT ÊTRE \
                 BLOQUÉ. Intervention manuelle requise : `nft flush chain inet vitrail \
                 VITRAIL_REDIRECT` en root."
            );
        }
        // Que le retrait ait réussi ou non : PolarProxy est mort, `is_active()` ne doit jamais
        // continuer de mentir en affichant `true` (point 3, audit EPIC 4).
        self.active.store(false, Ordering::SeqCst);
    }
}

/// Retente `nft_clear_redirect()` depuis le garde-fou — même pattern retry/backoff que
/// `killswitch::sequence::stop_with_retry`. Un échec transitoire (`pkexec` indisponible, prompt
/// polkit qui timeout, `vitrail-helper` momentanément introuvable) ne doit pas suffire à
/// abandonner après une seule tentative.
fn clear_redirect_with_retry(redirect: &dyn HelperBackend) -> Result<(), String> {
    let mut last_error = String::new();
    for attempt in 1..=ABNORMAL_EXIT_CLEAR_MAX_ATTEMPTS {
        match redirect.nft_clear_redirect() {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = error;
                if attempt < ABNORMAL_EXIT_CLEAR_MAX_ATTEMPTS {
                    tracing::warn!(
                        attempt,
                        error = %last_error,
                        "decryption: garde-fou — tentative de retrait de la redirection \
                         nftables échouée, nouvelle tentative"
                    );
                    std::thread::sleep(ABNORMAL_EXIT_CLEAR_RETRY_BACKOFF);
                }
            }
        }
    }
    Err(last_error)
}
