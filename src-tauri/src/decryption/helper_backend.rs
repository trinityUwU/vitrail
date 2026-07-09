//! Abstraction entre `decryption/` et l'exécution privilégiée réelle (`pkexec vitrail-helper`)
//! — même principe que `killswitch::nftables::NftablesBackend` (EPIC 7), mais couvre les 5
//! nouvelles sous-commandes EPIC 4 (`install-ca`/`remove-ca`/`nft-redirect`/
//! `nft-clear-redirect`/`nft-set-exclusions`). `SystemHelperBackend` invoque le même binaire
//! `vitrail-helper` déjà utilisé par `killswitch/nftables.rs` ; `FakeHelperBackend` reste en
//! mémoire pour les tests, jamais de process réel ni de prompt polkit déclenché par un test.

use std::process::Command;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
#[cfg(test)]
use std::sync::{Arc, Mutex};

const DEFAULT_HELPER_PATH: &str = "/usr/local/bin/vitrail-helper";

pub trait HelperBackend: Send + Sync {
    fn install_ca(&self, cert_path: &str) -> Result<(), String>;
    fn remove_ca(&self, fingerprint: &str) -> Result<(), String>;
    fn nft_redirect(&self, port: u16) -> Result<(), String>;
    fn nft_clear_redirect(&self) -> Result<(), String>;
    fn nft_set_exclusions(&self, ips: &[String]) -> Result<(), String>;
}

/// Port local non privilégié (`> 1024`) — même règle que `vitrail-helper::validate::
/// validate_local_port`, dupliquée côté app AVANT tout appel `pkexec` (défense en profondeur,
/// même pattern que `attribution::daemon_config::validate_socket_address` pour
/// `opensnitch-set-socket`, EPIC 1). Le helper revalide indépendamment côté privilégié — ceci
/// n'est jamais la seule ligne de défense, juste un rejet précoce côté app (audit EPIC 4, point
/// 4).
fn validate_redirect_port(port: u16) -> Result<(), String> {
    if port <= 1024 {
        return Err(format!("port privilégié refusé (attendu > 1024): {port}"));
    }
    Ok(())
}

pub struct SystemHelperBackend;

impl SystemHelperBackend {
    fn helper_path() -> String {
        std::env::var("VITRAIL_HELPER_PATH").unwrap_or_else(|_| DEFAULT_HELPER_PATH.to_string())
    }

    fn run_pkexec(args: &[&str]) -> Result<(), String> {
        let helper = Self::helper_path();
        let output = Command::new("pkexec")
            .arg(&helper)
            .args(args)
            .output()
            .map_err(|error| {
                tracing::error!(error = %error, helper = %helper, "échec d'invocation de pkexec");
                error.to_string()
            })?;

        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        tracing::error!(args = ?args, stderr = %stderr, "vitrail-helper a échoué");
        Err(stderr)
    }
}

impl HelperBackend for SystemHelperBackend {
    fn install_ca(&self, cert_path: &str) -> Result<(), String> {
        Self::run_pkexec(&["install-ca", cert_path])
    }

    fn remove_ca(&self, fingerprint: &str) -> Result<(), String> {
        Self::run_pkexec(&["remove-ca", fingerprint])
    }

    fn nft_redirect(&self, port: u16) -> Result<(), String> {
        validate_redirect_port(port)?;
        Self::run_pkexec(&["nft-redirect", &port.to_string()])
    }

    fn nft_clear_redirect(&self) -> Result<(), String> {
        Self::run_pkexec(&["nft-clear-redirect"])
    }

    fn nft_set_exclusions(&self, ips: &[String]) -> Result<(), String> {
        Self::run_pkexec(&["nft-set-exclusions", &ips.join(",")])
    }
}

/// Backend en mémoire pour les tests — jamais de process réel. `Clone` (via `Arc` interne) pour
/// que les tests puissent inspecter les compteurs d'appels après coup tout en passant une
/// possession `Box<dyn HelperBackend>` au sous-système sous test.
#[cfg(test)]
#[derive(Clone)]
pub struct FakeHelperBackend {
    install_calls: Arc<AtomicUsize>,
    remove_calls: Arc<AtomicUsize>,
    redirect_calls: Arc<Mutex<Vec<u16>>>,
    clear_redirect_calls: Arc<AtomicUsize>,
    exclusions_calls: Arc<Mutex<Vec<Vec<String>>>>,
    /// Nombre d'appels `nft_clear_redirect` restants à faire échouer avant de réussir — simule
    /// un `pkexec`/`vitrail-helper` indisponible (point 2, audit EPIC 4). `usize::MAX` simule un
    /// échec permanent (ex: binaire helper introuvable).
    clear_redirect_fail_remaining: Arc<AtomicUsize>,
}

#[cfg(test)]
impl FakeHelperBackend {
    pub fn new() -> Self {
        Self {
            install_calls: Arc::new(AtomicUsize::new(0)),
            remove_calls: Arc::new(AtomicUsize::new(0)),
            redirect_calls: Arc::new(Mutex::new(Vec::new())),
            clear_redirect_calls: Arc::new(AtomicUsize::new(0)),
            exclusions_calls: Arc::new(Mutex::new(Vec::new())),
            clear_redirect_fail_remaining: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Fait échouer les `n` prochains appels à `nft_clear_redirect` (puis réussir normalement).
    pub fn fail_clear_redirect_times(&self, n: usize) {
        self.clear_redirect_fail_remaining
            .store(n, Ordering::SeqCst);
    }

    pub fn install_calls(&self) -> usize {
        self.install_calls.load(Ordering::SeqCst)
    }

    pub fn remove_calls(&self) -> usize {
        self.remove_calls.load(Ordering::SeqCst)
    }

    pub fn redirect_calls(&self) -> Vec<u16> {
        self.redirect_calls.lock().unwrap().clone()
    }

    pub fn clear_redirect_calls(&self) -> usize {
        self.clear_redirect_calls.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
impl HelperBackend for FakeHelperBackend {
    fn install_ca(&self, _cert_path: &str) -> Result<(), String> {
        self.install_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn remove_ca(&self, _fingerprint: &str) -> Result<(), String> {
        self.remove_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn nft_redirect(&self, port: u16) -> Result<(), String> {
        self.redirect_calls.lock().unwrap().push(port);
        Ok(())
    }

    fn nft_clear_redirect(&self) -> Result<(), String> {
        self.clear_redirect_calls.fetch_add(1, Ordering::SeqCst);
        let remaining = self.clear_redirect_fail_remaining.load(Ordering::SeqCst);
        if remaining > 0 {
            self.clear_redirect_fail_remaining
                .store(remaining - 1, Ordering::SeqCst);
            return Err(
                "échec simulé de nft-clear-redirect (fake, pkexec indisponible)".to_string(),
            );
        }
        Ok(())
    }

    fn nft_set_exclusions(&self, ips: &[String]) -> Result<(), String> {
        self.exclusions_calls.lock().unwrap().push(ips.to_vec());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_redirect_port_refuse_les_ports_privilegies() {
        assert!(validate_redirect_port(80).is_err());
        assert!(validate_redirect_port(443).is_err());
        assert!(validate_redirect_port(1024).is_err());
        assert!(validate_redirect_port(1025).is_ok());
        assert!(validate_redirect_port(10443).is_ok());
    }
}
