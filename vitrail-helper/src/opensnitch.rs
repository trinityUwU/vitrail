//! `opensnitch-set-socket` (PLAN.md §6quinquies, EPIC 1) — édite `Server.Address` dans
//! `/etc/opensnitchd/default-config.json` puis redémarre `opensnitchd`.

use std::process::Command;

use crate::validate;

const OPENSNITCH_CONFIG_PATH: &str = "/etc/opensnitchd/default-config.json";

/// État dégradé distinct d'un échec total : `ConfigWrittenRestartFailed` signale que le
/// fichier de config a déjà été écrit avec la nouvelle adresse mais que le restart n'a pas
/// suivi — fichier et runtime `opensnitchd` divergent jusqu'à un restart externe.
pub enum SetSocketError {
    Failed(String),
    ConfigWrittenRestartFailed(String),
}

pub fn opensnitch_set_socket(socket_address: &str) -> Result<(), SetSocketError> {
    validate_socket_address(socket_address).map_err(SetSocketError::Failed)?;

    let content = std::fs::read_to_string(OPENSNITCH_CONFIG_PATH).map_err(|error| {
        SetSocketError::Failed(format!(
            "lecture de {OPENSNITCH_CONFIG_PATH} échouée: {error}"
        ))
    })?;
    let mut json: serde_json::Value = serde_json::from_str(&content).map_err(|error| {
        SetSocketError::Failed(format!(
            "parsing JSON de {OPENSNITCH_CONFIG_PATH} échoué: {error}"
        ))
    })?;

    let server = json.get_mut("Server").ok_or_else(|| {
        SetSocketError::Failed("champ 'Server' absent de la config opensnitchd".to_string())
    })?;
    server["Address"] = serde_json::Value::String(socket_address.to_string());

    let serialized = serde_json::to_string_pretty(&json)
        .map_err(|error| SetSocketError::Failed(format!("sérialisation JSON échouée: {error}")))?;
    std::fs::write(OPENSNITCH_CONFIG_PATH, serialized).map_err(|error| {
        SetSocketError::Failed(format!(
            "écriture de {OPENSNITCH_CONFIG_PATH} échouée: {error}"
        ))
    })?;

    // Point de non-retour : le fichier reflète déjà la nouvelle adresse. Toute erreur à partir
    // d'ici est une divergence fichier/runtime, jamais un échec générique indistinct.
    run_systemctl(&["restart", "opensnitchd"]).map_err(SetSocketError::ConfigWrittenRestartFailed)
}

/// Validation stricte AVANT toute écriture : seul le format `unix:///chemin/absolu` est
/// accepté, aucun caractère shell, jamais de `..`. Miroir de
/// `attribution::daemon_config::validate_socket_address` côté app — dupliqué volontairement
/// (ce binaire ne dépend d'aucun crate de `src-tauri`, garantie de surface étroite).
fn validate_socket_address(address: &str) -> Result<(), String> {
    let invalid = || format!("adresse socket invalide, refusée: {address}");
    let path = address.strip_prefix("unix://").ok_or_else(invalid)?;
    validate::validate_file_path(path).map_err(|_| invalid())
}

fn run_systemctl(args: &[&str]) -> Result<(), String> {
    let output = Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|error| {
            format!(
                "échec d'exécution de `systemctl {}`: {error}",
                args.join(" ")
            )
        })?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "`systemctl {}` a échoué (code {}): {}",
        args.join(" "),
        output.status.code().unwrap_or(-1),
        stderr.trim()
    ))
}
