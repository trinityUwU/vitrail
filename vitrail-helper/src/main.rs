//! Binaire privilégié minimal invoqué via `pkexec` par `killswitch/nftables.rs`.
//!
//! Surface volontairement étroite (décision PLAN.md §6bis/6ter) : deux sous-commandes fixes,
//! aucune autre action, jamais d'interpolation shell — uniquement `std::process::Command`
//! avec un tableau d'arguments fixe passé directement à `nft`.

use std::process::{Command, ExitCode};

const NFT_BIN: &str = "nft";
const NFT_FAMILY: &str = "inet";
const NFT_TABLE: &str = "vitrail";
const NFT_CHAIN: &str = "VITRAIL_REDIRECT";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let subcommand = match args.next() {
        Some(value) => value,
        None => {
            eprintln!("usage: vitrail-helper <nft-apply|nft-flush>");
            return ExitCode::FAILURE;
        }
    };

    let result = match subcommand.as_str() {
        "nft-apply" => nft_apply(),
        "nft-flush" => nft_flush(),
        other => {
            eprintln!("sous-commande inconnue: {other} (attendu: nft-apply, nft-flush)");
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("vitrail-helper: {message}");
            ExitCode::FAILURE
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
