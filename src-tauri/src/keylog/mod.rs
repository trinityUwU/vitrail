//! Pipeline SSLKEYLOGFILE (injection + tail + tshark) — n'intercepte jamais activement (pas de
//! MITM), ARCHITECTURE.md. EPIC 3 (PLAN.md §6octies) : délègue tout le déchiffrement/la
//! reconstruction HTTP à `tshark` en sous-processus (`-o tls.keylog_file:`, sortie `-T ek`),
//! aucun parsing TLS/HTTP maison. `KeylogSubsystem` implémente `Subsystem`
//! (killswitch/subsystem.rs), remplace le `StubSubsystem` "keylog".

mod app_injection;
mod detection;
mod keyfile;
mod parser;
mod subsystem;
mod tshark_process;

// `parse_ek_line` : réexporté pour `decryption/` (EPIC 4) — même précédent déjà établi que
// `attribution::normalize_ip`/`attribution::find_desktop_file` consommés directement par
// `keylog/` (import de fonction publique d'un autre domaine, pas de ses internes). PolarProxy
// (via `--pcapoverip`) et le pipeline SSLKEYLOGFILE produisent tous deux un flux `tshark -T ek`
// à interpréter de façon identique — DRY volontaire sur cette seule fonction de parsing,
// jamais sur la logique métier des deux domaines qui restent indépendants.
pub use parser::{parse_ek_line, DecryptedFragment};
pub use subsystem::KeylogSubsystem;

#[cfg(test)]
pub use subsystem::FakeKeylogSubsystem;

use std::path::PathBuf;

/// `$XDG_DATA_HOME` avec repli `~/.local/share` — même convention que
/// `storage::connection::default_db_path` (PLAN.md §6sexies), dupliquée ici plutôt que
/// partagée : `storage/` ne doit exposer aucun helper de chemin en dehors de son propre
/// fichier DB (frontière stricte, storage ignore les chemins des autres domaines).
pub(crate) fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        })
}

pub(crate) fn vitrail_data_dir() -> PathBuf {
    xdg_data_home().join("vitrail")
}

/// Restaure immédiatement l'injection d'une app avant son retrait de la liste ciblée — appelé
/// par `commands/settings.rs::remove_keylog_app`. Sans ce filet, retirer une app pendant que le
/// kill switch est actif laisserait sa surcharge `.desktop` orpheline : un futur `stop()` ne la
/// connaîtrait plus (la ligne storage aurait déjà été supprimée), cassant la garantie de
/// réversibilité (PLAN.md §6octies). Décision non explicite dans PLAN.md — tranchée ainsi,
/// signalée au rapport de livraison EPIC 3. No-op si l'app n'était pas injectée.
pub fn restore_app_injection(storage: &crate::storage::StorageHandle, binary_path: &str) {
    if let Ok(Some(row)) = crate::storage::keylog::get_app(storage, binary_path) {
        if row.desktop_path.is_some() {
            app_injection::restore_app(storage, &row);
        }
    }
}
