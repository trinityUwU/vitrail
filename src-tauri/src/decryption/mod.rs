//! Orchestrer PolarProxy, produire du contenu déchiffré + signaux de pinning — ne fait pas
//! d'attribution processus (ARCHITECTURE.md). EPIC 4 (PLAN.md §6nonies) : domaine le plus
//! risqué du projet (CA système, redirection nftables de TOUT le trafic 80/443, MITM actif).
//! `CaSubsystem`/`PolarProxySubsystem` implémentent `Subsystem`, remplacent les deux stubs
//! `"ca"`/`"polarproxy"` de `killswitch/mod.rs`.

mod abnormal_exit_guard;
mod ca;
mod exclusions;
mod helper_backend;
mod output;
mod polarproxy_process;
mod subsystem;

pub use ca::{rotate_ca, CaSubsystem};
pub use exclusions::{add_exclusion, remove_exclusion};
pub use helper_backend::{HelperBackend, SystemHelperBackend};
pub use subsystem::PolarProxySubsystem;

#[cfg(test)]
pub use ca::FakeCaSubsystem;
#[cfg(test)]
pub use subsystem::FakePolarProxySubsystem;

use std::path::PathBuf;

/// Même convention que `keylog::xdg_data_home`/`storage::connection::default_db_path` —
/// dupliquée volontairement (frontière stricte, chaque domaine ignore les chemins des autres).
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
