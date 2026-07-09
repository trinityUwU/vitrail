//! Binaire privilégié minimal invoqué via `pkexec` par `killswitch/nftables.rs` et
//! `attribution/daemon_config.rs`.
//!
//! Surface volontairement étroite (décision PLAN.md §6bis/6ter/6quinquies) : trois
//! sous-commandes fixes, aucune autre action, jamais d'interpolation shell — uniquement
//! `std::process::Command` avec un tableau d'arguments fixe passé directement à `nft`/
//! `systemctl`, jamais de shell intermédiaire.

use std::process::{Command, ExitCode};

/// Code de sortie dédié à l'état dégradé "config écrite mais restart échoué" (fix robustesse
/// EPIC 1) — distinct de `ExitCode::FAILURE` générique pour que `daemon_config.rs` puisse le
/// reconnaître et le logger comme une divergence fichier/runtime, pas un échec total.
const EXIT_CONFIG_WRITTEN_RESTART_FAILED: u8 = 2;

const NFT_BIN: &str = "nft";
const NFT_FAMILY: &str = "inet";
const NFT_TABLE: &str = "vitrail";
const NFT_CHAIN: &str = "VITRAIL_REDIRECT";
const OPENSNITCH_CONFIG_PATH: &str = "/etc/opensnitchd/default-config.json";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let subcommand = match args.next() {
        Some(value) => value,
        None => {
            eprintln!("usage: vitrail-helper <nft-apply|nft-flush|opensnitch-set-socket>");
            return ExitCode::FAILURE;
        }
    };

    match subcommand.as_str() {
        "nft-apply" => exit_code_from(nft_apply()),
        "nft-flush" => exit_code_from(nft_flush()),
        "opensnitch-set-socket" => match args.next() {
            Some(socket_address) => opensnitch_set_socket_exit_code(&socket_address),
            None => {
                eprintln!("usage: vitrail-helper opensnitch-set-socket <adresse-socket>");
                ExitCode::FAILURE
            }
        },
        other => {
            eprintln!(
                "sous-commande inconnue: {other} \
                 (attendu: nft-apply, nft-flush, opensnitch-set-socket)"
            );
            ExitCode::FAILURE
        }
    }
}

fn exit_code_from(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("vitrail-helper: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Traduit `opensnitch_set_socket` en code de sortie — distingue explicitement l'état dégradé
/// "config écrite mais restart échoué" (`ConfigWrittenRestartFailed`) d'un échec total, pour que
/// l'appelant Rust (`daemon_config.rs`) puisse logger l'incohérence de façon actionnable plutôt
/// que la confondre avec un échec générique.
fn opensnitch_set_socket_exit_code(socket_address: &str) -> ExitCode {
    match opensnitch_set_socket(socket_address) {
        Ok(()) => ExitCode::SUCCESS,
        Err(SetSocketError::Failed(message)) => {
            eprintln!("vitrail-helper: {message}");
            ExitCode::FAILURE
        }
        Err(SetSocketError::ConfigWrittenRestartFailed(message)) => {
            eprintln!(
                "vitrail-helper: INCOHÉRENCE config/runtime opensnitchd — configuration écrite \
                 mais `systemctl restart` a échoué: {message}"
            );
            ExitCode::from(EXIT_CONFIG_WRITTEN_RESTART_FAILED)
        }
    }
}

/// Crée la table `inet vitrail` et la chaîne `VITRAIL_REDIRECT` (vide, hook output) si elles
/// n'existent pas déjà. `nft add` est idempotent par nature (contrairement à `nft create`).
fn nft_apply() -> Result<(), String> {
    run_nft(&["add", "table", NFT_FAMILY, NFT_TABLE])?;
    run_nft(&[
        "add", "chain", NFT_FAMILY, NFT_TABLE, NFT_CHAIN, "{", "type", "filter", "hook", "output",
        "priority", "0", ";", "}",
    ])?;
    Ok(())
}

/// Détruit la table `inet vitrail` (et donc la chaîne qu'elle contient) si elle existe.
/// Idempotent : ne doit pas échouer si la table est déjà absente.
fn nft_flush() -> Result<(), String> {
    if !table_exists()? {
        return Ok(());
    }
    run_nft(&["delete", "table", NFT_FAMILY, NFT_TABLE])
}

fn table_exists() -> Result<bool, String> {
    let output = Command::new(NFT_BIN)
        .args(["list", "table", NFT_FAMILY, NFT_TABLE])
        .output()
        .map_err(|error| format!("échec d'exécution de `nft list table`: {error}"))?;
    Ok(output.status.success())
}

fn run_nft(args: &[&str]) -> Result<(), String> {
    let output = Command::new(NFT_BIN)
        .args(args)
        .output()
        .map_err(|error| format!("échec d'exécution de `nft {}`: {error}", args.join(" ")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "`nft {}` a échoué (code {}): {}",
        args.join(" "),
        output.status.code().unwrap_or(-1),
        stderr.trim()
    ))
}

/// État dégradé distinct d'un échec total (fix robustesse EPIC 1) : `ConfigWrittenRestartFailed`
/// signale que le fichier de config a déjà été écrit avec la nouvelle adresse mais que le
/// restart n'a pas suivi — fichier et runtime `opensnitchd` divergent jusqu'à un restart externe.
enum SetSocketError {
    Failed(String),
    ConfigWrittenRestartFailed(String),
}

/// Édite `Server.Address` dans `/etc/opensnitchd/default-config.json` puis redémarre
/// `opensnitchd` (PLAN.md §6quinquies). Validation stricte du chemin socket AVANT toute
/// écriture — jamais d'exécution shell arbitraire, jamais d'interpolation. Le restart doit
/// suivre l'écriture (il relit le fichier au démarrage) : à partir du moment où le fichier est
/// écrit, un échec de restart n'est PLUS un échec total mais un état dégradé signalé à part.
fn opensnitch_set_socket(socket_address: &str) -> Result<(), SetSocketError> {
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

/// Validation stricte AVANT toute écriture ou action privilégiée : seul le format
/// `unix:///chemin/absolu` est accepté (PLAN.md §6quinquies), aucun caractère shell, jamais
/// de `..`. Miroir exact de `attribution::daemon_config::validate_socket_address` côté app —
/// dupliqué volontairement (duplication accidentelle tolérée, code-standards.md DRY) : ce
/// binaire ne dépend d'aucun crate de `src-tauri`, c'est la garantie de surface étroite.
fn validate_socket_address(address: &str) -> Result<(), String> {
    let invalid = || format!("adresse socket invalide, refusée: {address}");
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
