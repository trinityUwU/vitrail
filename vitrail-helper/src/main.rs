//! Binaire privilégié minimal invoqué via `pkexec` par `killswitch/nftables.rs`,
//! `attribution/daemon_config.rs` et `decryption/` (EPIC 4).
//!
//! Surface volontairement étroite (PLAN.md §6bis/6ter/6quinquies/6nonies) : sous-commandes
//! fixes, aucune autre action, jamais d'interpolation shell — uniquement `std::process::Command`
//! avec un tableau d'arguments fixe, jamais de shell intermédiaire. Chaque sous-commande valide
//! strictement ses arguments (`validate.rs`) AVANT toute action privilégiée.

mod ca;
mod nft;
mod opensnitch;
mod validate;

use std::process::ExitCode;

use ca::EXIT_NO_TRUST_MECHANISM;
use opensnitch::SetSocketError;

/// Code de sortie dédié à l'état dégradé "config écrite mais restart échoué" (EPIC 1) —
/// distinct de `ExitCode::FAILURE` générique.
const EXIT_CONFIG_WRITTEN_RESTART_FAILED: u8 = 2;

const USAGE: &str = "usage: vitrail-helper <nft-apply|nft-flush|opensnitch-set-socket|\
install-ca|remove-ca|nft-redirect|nft-clear-redirect|nft-set-exclusions> [args]";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let subcommand = match args.next() {
        Some(value) => value,
        None => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };

    match subcommand.as_str() {
        "nft-apply" => exit_code_from(nft::nft_apply()),
        "nft-flush" => exit_code_from(nft::nft_flush()),
        "opensnitch-set-socket" => match args.next() {
            Some(socket_address) => opensnitch_set_socket_exit_code(&socket_address),
            None => usage_failure("opensnitch-set-socket <adresse-socket>"),
        },
        "install-ca" => match args.next() {
            Some(cert_path) => install_ca_exit_code(&cert_path),
            None => usage_failure("install-ca <chemin-cert>"),
        },
        "remove-ca" => match args.next() {
            Some(fingerprint) => remove_ca_exit_code(&fingerprint),
            None => usage_failure("remove-ca <fingerprint-sha256-exact>"),
        },
        "nft-redirect" => match args.next() {
            Some(port) => nft_redirect_exit_code(&port),
            None => usage_failure("nft-redirect <port-local>"),
        },
        "nft-clear-redirect" => exit_code_from(nft::nft_clear_redirect()),
        "nft-set-exclusions" => nft_set_exclusions_exit_code(args.next().as_deref().unwrap_or("")),
        other => {
            eprintln!("sous-commande inconnue: {other}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn usage_failure(expected: &str) -> ExitCode {
    eprintln!("usage: vitrail-helper {expected}");
    ExitCode::FAILURE
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

fn opensnitch_set_socket_exit_code(socket_address: &str) -> ExitCode {
    match opensnitch::opensnitch_set_socket(socket_address) {
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

fn install_ca_exit_code(cert_path: &str) -> ExitCode {
    if let Err(message) = validate::validate_file_path(cert_path) {
        eprintln!("vitrail-helper: {message}");
        return ExitCode::FAILURE;
    }
    match ca::install_ca(cert_path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(ca::InstallError::Failed(message)) => {
            eprintln!("vitrail-helper: {message}");
            ExitCode::FAILURE
        }
        Err(ca::InstallError::NoTrustMechanism(message)) => {
            eprintln!("vitrail-helper: {message}");
            ExitCode::from(EXIT_NO_TRUST_MECHANISM)
        }
    }
}

fn remove_ca_exit_code(fingerprint: &str) -> ExitCode {
    if let Err(message) = validate::validate_fingerprint(fingerprint) {
        eprintln!("vitrail-helper: {message}");
        return ExitCode::FAILURE;
    }
    exit_code_from(ca::remove_ca(fingerprint))
}

fn nft_redirect_exit_code(port: &str) -> ExitCode {
    match validate::validate_local_port(port) {
        Ok(parsed) => exit_code_from(nft::nft_redirect(parsed)),
        Err(message) => {
            eprintln!("vitrail-helper: {message}");
            ExitCode::FAILURE
        }
    }
}

fn nft_set_exclusions_exit_code(raw: &str) -> ExitCode {
    let ips = match validate::validate_ip_list(raw) {
        Ok(ips) => ips,
        Err(message) => {
            eprintln!("vitrail-helper: {message}");
            return ExitCode::FAILURE;
        }
    };
    let (v4, v6): (Vec<String>, Vec<String>) =
        ips.iter()
            .fold((Vec::new(), Vec::new()), |(mut v4, mut v6), ip| {
                match ip {
                    std::net::IpAddr::V4(_) => v4.push(ip.to_string()),
                    std::net::IpAddr::V6(_) => v6.push(ip.to_string()),
                }
                (v4, v6)
            });
    exit_code_from(nft::nft_set_exclusions(&v4, &v6))
}
