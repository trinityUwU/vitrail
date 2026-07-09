//! Détection + reconfiguration du daemon `opensnitchd` — stories 1.1/1.2/1.6.
//! Lecture de `/etc/opensnitchd/default-config.json` en LECTURE SEULE (best-effort, aucun
//! privilège requis pour lire un fichier de config lisible) ; la reconfiguration (écriture +
//! `systemctl restart`) passe exclusivement par `pkexec vitrail-helper opensnitch-set-socket
//! <adresse>` (PLAN.md §6quinquies) — jamais d'écriture directe du fichier depuis l'app.

use std::fs;
use std::process::Command;

use serde_json::Value;

use crate::killswitch::KillSwitchError;
use crate::storage::{self, StorageHandle};

const CONFIG_PATH: &str = "/etc/opensnitchd/default-config.json";
const DEFAULT_HELPER_PATH: &str = "/usr/local/bin/vitrail-helper";
/// Miroir de `vitrail-helper::EXIT_CONFIG_WRITTEN_RESTART_FAILED` — les deux binaires ne
/// partagent aucun crate commun (surface étroite volontaire, cf. doc de tête de
/// `vitrail-helper/src/main.rs`), ce code est donc dupliqué ici pour distinguer un échec total
/// d'un état dégradé "config écrite mais restart opensnitchd échoué" (fix robustesse EPIC 1).
const HELPER_EXIT_CONFIG_WRITTEN_RESTART_FAILED: i32 = 2;

#[derive(Debug, Clone)]
pub struct DaemonDetection {
    pub installed: bool,
    // Conservé dans `DaemonDetection` (story 1.1) même si aucun code ne le relit encore hors
    // tests : `AttributionSubsystem::last_detection()` l'expose pour un futur
    // `get_system_status` enrichi (EPIC 8), pas de commande IPC dédiée dans ce périmètre.
    #[allow(dead_code)]
    pub active: bool,
    pub current_address: Option<String>,
    /// true si l'adresse actuelle ne pointe pas déjà vers Vitrail (probable GUI officielle
    /// ou autre UI) — story 1.1, doit être signalé explicitement, jamais une surprise
    /// silencieuse.
    #[allow(dead_code)]
    pub points_elsewhere: bool,
}

pub trait DaemonConfigurator: Send + Sync {
    fn detect(&self, vitrail_socket: &str) -> DaemonDetection;
    fn set_socket(&self, socket_address: &str) -> Result<(), KillSwitchError>;
}

pub struct SystemDaemonConfigurator;

impl SystemDaemonConfigurator {
    fn helper_path() -> String {
        std::env::var("VITRAIL_HELPER_PATH").unwrap_or_else(|_| DEFAULT_HELPER_PATH.to_string())
    }

    fn is_active() -> bool {
        Command::new("systemctl")
            .args(["is-active", "--quiet", "opensnitchd"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn is_installed() -> bool {
        Command::new("systemctl")
            .args(["list-unit-files", "opensnitchd.service"])
            .output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout).contains("opensnitchd.service")
            })
            .unwrap_or(false)
    }

    fn read_current_address() -> Option<String> {
        let content = fs::read_to_string(CONFIG_PATH).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;
        json.get("Server")?
            .get("Address")?
            .as_str()
            .map(|s| s.to_string())
    }
}

impl DaemonConfigurator for SystemDaemonConfigurator {
    /// Détection best-effort, jamais de `pkexec` ici (lecture seule uniquement).
    fn detect(&self, vitrail_socket: &str) -> DaemonDetection {
        let installed = Self::is_installed();
        let active = installed && Self::is_active();
        let current_address = Self::read_current_address();
        let points_elsewhere = match &current_address {
            Some(addr) => addr != vitrail_socket,
            None => false,
        };
        if points_elsewhere {
            tracing::warn!(
                current = ?current_address,
                vitrail = vitrail_socket,
                "opensnitchd pointe déjà vers une autre UI (probable GUI officielle) — \
                 la reconfiguration Vitrail va la couper"
            );
        }
        if !installed {
            tracing::warn!("opensnitchd non détecté (unité systemd absente)");
        }
        DaemonDetection {
            installed,
            active,
            current_address,
            points_elsewhere,
        }
    }

    fn set_socket(&self, socket_address: &str) -> Result<(), KillSwitchError> {
        validate_socket_address(socket_address)?;
        let helper = Self::helper_path();
        let output = Command::new("pkexec")
            .arg(&helper)
            .arg("opensnitch-set-socket")
            .arg(socket_address)
            .output()
            .map_err(|error| {
                tracing::error!(
                    error = %error,
                    helper = %helper,
                    "échec d'invocation de pkexec (opensnitch-set-socket)"
                );
                exec_error(error)
            })?;

        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if output.status.code() == Some(HELPER_EXIT_CONFIG_WRITTEN_RESTART_FAILED) {
            tracing::error!(
                stderr = %stderr,
                socket_address = %socket_address,
                "INCOHÉRENCE opensnitchd : fichier de config écrit mais `systemctl restart` a \
                 échoué — le daemon en cours d'exécution tourne encore avec l'ancienne adresse, \
                 restart manuel requis pour resynchroniser fichier et runtime"
            );
        } else {
            tracing::error!(stderr = %stderr, "vitrail-helper opensnitch-set-socket a échoué");
        }
        Err(exec_error(stderr))
    }
}

fn exec_error(reason: impl ToString) -> KillSwitchError {
    KillSwitchError::SubsystemExec {
        subsystem: "attribution".to_string(),
        reason: reason.to_string(),
    }
}

/// Validation stricte côté Rust AVANT tout appel privilégié (PLAN.md §6quinquies) : seul le
/// format `unix:///chemin/absolu` est accepté, aucun caractère shell, jamais de `..`.
/// Décision non explicitement tranchée dans PLAN.md : les adresses TCP (`ip:port`), possibles
/// côté opensnitchd, sont volontairement rejetées — hors périmètre 1.2/1.6, signalé en rapport.
pub fn validate_socket_address(address: &str) -> Result<(), KillSwitchError> {
    let invalid = || exec_error(format!("adresse socket invalide, refusée: {address}"));

    let path = address.strip_prefix("unix://").ok_or_else(invalid)?;
    if !path.starts_with('/') || path.contains("..") || path.is_empty() || path.len() > 4096 {
        return Err(invalid());
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.');
    if !path.chars().all(allowed) {
        return Err(invalid());
    }
    Ok(())
}

// --- Persistance de l'adresse d'origine (stories 1.1/1.6) ---
// EPIC 6 : remplace `attribution_state.jsonl` par `storage::attribution` (table
// `attribution_state`), même comportement observable (dernière ligne valide relue).

/// Sauvegarde l'adresse d'origine AVANT toute reconfiguration. Ne remplace jamais une entrée :
/// la restauration relit toujours la DERNIÈRE valeur insérée.
pub fn save_original_address(
    storage: &StorageHandle,
    original_address: &str,
) -> Result<(), KillSwitchError> {
    storage::attribution::save_origin_socket(storage, original_address).map_err(|error| {
        tracing::error!(error = %error, "sauvegarde de l'adresse d'origine opensnitchd (storage) échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}

/// Relit la dernière adresse d'origine sauvegardée.
pub fn read_last_original_address(storage: &StorageHandle) -> Option<String> {
    storage::attribution::read_origin_socket(storage)
        .inspect_err(|error| {
            tracing::error!(error = %error, "lecture de l'adresse d'origine opensnitchd (storage) échouée");
        })
        .ok()
        .flatten()
}

/// Variante testable — jamais de `pkexec`/`systemctl` ni d'accès à `/etc/opensnitchd/` réel
/// (même principe que `killswitch::nftables::FakeNftablesBackend`).
#[cfg(test)]
pub struct FakeDaemonConfigurator {
    pub set_calls: std::sync::Mutex<Vec<String>>,
    pub fail_set: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl FakeDaemonConfigurator {
    pub fn new() -> Self {
        Self {
            set_calls: std::sync::Mutex::new(Vec::new()),
            fail_set: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakeDaemonConfigurator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl DaemonConfigurator for FakeDaemonConfigurator {
    fn detect(&self, vitrail_socket: &str) -> DaemonDetection {
        let original = "unix:///tmp/osui.sock".to_string();
        DaemonDetection {
            installed: true,
            active: true,
            points_elsewhere: vitrail_socket != original,
            current_address: Some(original),
        }
    }

    fn set_socket(&self, socket_address: &str) -> Result<(), KillSwitchError> {
        if self.fail_set.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(exec_error("échec simulé"));
        }
        self.set_calls
            .lock()
            .expect("mutex fake configurator empoisonné")
            .push(socket_address.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valide_une_adresse_unix_absolue() {
        assert!(validate_socket_address("unix:///run/user/1000/vitrail/ui.sock").is_ok());
    }

    #[test]
    fn refuse_une_adresse_sans_prefixe_unix() {
        assert!(validate_socket_address("/tmp/osui.sock").is_err());
        assert!(validate_socket_address("127.0.0.1:50051").is_err());
    }

    #[test]
    fn refuse_un_chemin_relatif_ou_avec_traversal() {
        assert!(validate_socket_address("unix://relative/path").is_err());
        assert!(validate_socket_address("unix:///tmp/../etc/passwd").is_err());
    }

    #[test]
    fn refuse_les_caracteres_shell() {
        assert!(validate_socket_address("unix:///tmp/foo; rm -rf /").is_err());
        assert!(validate_socket_address("unix:///tmp/$(whoami)").is_err());
    }
}
