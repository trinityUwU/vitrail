//! Snapshot horodaté de l'état système observable (chaîne nftables + sous-systèmes) et
//! persistance JSONL append-only pour l'audit (`system_events`, transitoire avant EPIC 6
//! — cf. PLAN.md §6ter, migration prévue vers SQLite sans réécriture de cette logique).

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::nftables::NftablesBackend;
use super::subsystem::Subsystem;
use super::KillSwitchError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsystemSnapshot {
    pub name: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp_unix: u64,
    pub nftables_chain_present: bool,
    pub subsystems: Vec<SubsystemSnapshot>,
}

impl SystemSnapshot {
    pub fn capture(nftables: &dyn NftablesBackend, subsystems: &[&dyn Subsystem]) -> Self {
        Self {
            timestamp_unix: now_unix(),
            nftables_chain_present: nftables.is_applied(),
            subsystems: subsystems
                .iter()
                .map(|s| SubsystemSnapshot {
                    name: s.name().to_string(),
                    active: s.is_active(),
                })
                .collect(),
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn events_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        });
    base.join("vitrail").join("system_events.jsonl")
}

/// Ajoute un événement au journal JSONL append-only (permissions 600). Erreur loggée et
/// remontée, jamais de panic : une défaillance d'écriture d'audit ne doit pas bloquer le
/// kill switch (les appelants ignorent volontairement l'erreur avec `let _ =`).
pub fn append_event(label: &str, snapshot: &SystemSnapshot) -> Result<(), KillSwitchError> {
    let path = events_path();
    ensure_parent_dir(&path)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&path)
        .map_err(|error| {
            tracing::error!(error = %error, path = %path.display(), "ouverture de system_events.jsonl échouée");
            KillSwitchError::Persistence(error.to_string())
        })?;

    let line = serde_json::json!({ "label": label, "snapshot": snapshot });
    writeln!(file, "{line}").map_err(|error| {
        tracing::error!(error = %error, "écriture dans system_events.jsonl échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}

fn ensure_parent_dir(path: &std::path::Path) -> Result<(), KillSwitchError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent).map_err(|error| {
        tracing::error!(error = %error, path = %parent.display(), "création du dossier system_events échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}
