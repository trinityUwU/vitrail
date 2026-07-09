//! Persistance JSONL append-only des paquets capturés (`capture_events.jsonl`, 600) — même
//! pattern que `killswitch::snapshot::append_event` (résolution `$XDG_DATA_HOME`, ouverture
//! 600), transitoire avant EPIC 6/SQLite (PLAN.md §6quater). Ne réutilise pas de logique de
//! corrélation ou de storage — ce fichier est strictement local au domaine `capture/`.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::killswitch::KillSwitchError;

/// Miroir du JSON Lines émis par `vitrail-capture-helper` (stories 2.2/2.3/2.4) — mêmes noms
/// de champs en `snake_case`, aucun `rename` : le helper sérialise sans renommage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPacket {
    pub timestamp_unix_ms: u128,
    pub interface: String,
    pub protocol: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub bytes: usize,
    pub sni: Option<String>,
    pub detected_protocol: Option<String>,
}

fn events_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        });
    base.join("vitrail").join("capture_events.jsonl")
}

/// Ajoute un paquet capturé au journal JSONL append-only. Erreur loggée et remontée, jamais
/// de panic : une défaillance de persistance ne doit pas interrompre le thread de lecture
/// stdout du helper.
pub fn append_packet(packet: &CapturedPacket) -> Result<(), KillSwitchError> {
    let path = events_path();
    ensure_parent_dir(&path)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&path)
        .map_err(|error| {
            tracing::error!(error = %error, path = %path.display(), "ouverture de capture_events.jsonl échouée");
            KillSwitchError::Persistence(error.to_string())
        })?;

    let line = serde_json::to_string(packet).map_err(|error| {
        tracing::error!(error = %error, "sérialisation d'un paquet capturé échouée");
        KillSwitchError::Persistence(error.to_string())
    })?;

    writeln!(file, "{line}").map_err(|error| {
        tracing::error!(error = %error, "écriture dans capture_events.jsonl échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}

fn ensure_parent_dir(path: &std::path::Path) -> Result<(), KillSwitchError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|error| {
        tracing::error!(error = %error, path = %parent.display(), "création du dossier capture échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}
