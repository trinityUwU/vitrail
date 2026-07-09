//! Abstraction entre l'orchestration killswitch et l'exécution réelle de nftables.
//! `SystemNftablesBackend` invoque `vitrail-helper` via `pkexec` (élévation polkit par
//! action, PLAN.md §6bis) ; `FakeNftablesBackend` reste en mémoire pour les tests (7.6),
//! jamais de process réel ni de prompt polkit déclenché par un test.

use std::process::Command;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use super::KillSwitchError;

const DEFAULT_HELPER_PATH: &str = "/usr/local/bin/vitrail-helper";

pub trait NftablesBackend: Send + Sync {
    fn apply(&self) -> Result<(), KillSwitchError>;
    fn flush(&self) -> Result<(), KillSwitchError>;
    fn is_applied(&self) -> bool;
}

pub struct SystemNftablesBackend;

impl SystemNftablesBackend {
    fn helper_path() -> String {
        std::env::var("VITRAIL_HELPER_PATH").unwrap_or_else(|_| DEFAULT_HELPER_PATH.to_string())
    }

    fn run_pkexec(subcommand: &str) -> Result<(), KillSwitchError> {
        let helper = Self::helper_path();
        let output = Command::new("pkexec")
            .arg(&helper)
            .arg(subcommand)
            .output()
            .map_err(|error| {
                tracing::error!(error = %error, helper = %helper, "échec d'invocation de pkexec");
                KillSwitchError::NftablesExec(error.to_string())
            })?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        tracing::error!(subcommand, stderr = %stderr, "vitrail-helper a échoué");
        Err(KillSwitchError::NftablesExec(stderr))
    }
}

impl NftablesBackend for SystemNftablesBackend {
    fn apply(&self) -> Result<(), KillSwitchError> {
        Self::run_pkexec("nft-apply")
    }

    fn flush(&self) -> Result<(), KillSwitchError> {
        Self::run_pkexec("nft-flush")
    }

    /// Lecture seule best-effort (pas de pkexec) : si la lecture échoue (permissions,
    /// nft absent), on considère prudemment la chaîne absente plutôt que de bloquer le
    /// snapshot sur une erreur de lecture.
    fn is_applied(&self) -> bool {
        match Command::new("nft")
            .args(["list", "table", "inet", "vitrail"])
            .output()
        {
            Ok(output) => output.status.success(),
            Err(error) => {
                tracing::warn!(error = %error, "lecture d'état nftables impossible (best-effort)");
                false
            }
        }
    }
}

/// Backend en mémoire pour les tests — jamais de process réel. Compilé uniquement en test
/// (seul `tests.rs` le consomme) pour ne pas déclencher de warning dead_code en build normal.
#[cfg(test)]
pub struct FakeNftablesBackend {
    applied: AtomicBool,
}

#[cfg(test)]
impl FakeNftablesBackend {
    pub fn new() -> Self {
        Self {
            applied: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakeNftablesBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl NftablesBackend for FakeNftablesBackend {
    fn apply(&self) -> Result<(), KillSwitchError> {
        self.applied.store(true, Ordering::SeqCst);
        tracing::info!("nftables (fake): chaîne VITRAIL_REDIRECT appliquée");
        Ok(())
    }

    fn flush(&self) -> Result<(), KillSwitchError> {
        self.applied.store(false, Ordering::SeqCst);
        tracing::info!("nftables (fake): chaîne VITRAIL_REDIRECT flush");
        Ok(())
    }

    fn is_applied(&self) -> bool {
        self.applied.load(Ordering::SeqCst)
    }
}
