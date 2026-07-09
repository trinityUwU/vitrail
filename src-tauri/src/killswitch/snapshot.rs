//! Snapshot horodaté de l'état système observable (chaîne nftables + sous-systèmes) et
//! persistance pour l'audit (`system_events`, table SQLite EPIC 6 — remplace le JSONL
//! provisoire posé en EPIC 7, PLAN.md §6ter/§6sexies, même comportement observable).

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::storage::{self, StorageHandle};

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

/// Insère un événement dans `storage::events::record_system_event` (remplace l'ancien append
/// JSONL). Erreur loggée et remontée, jamais de panic : une défaillance d'écriture d'audit ne
/// doit pas bloquer le kill switch (les appelants ignorent volontairement l'erreur avec
/// `let _ =`).
pub fn append_event(
    storage: &StorageHandle,
    label: &str,
    snapshot: &SystemSnapshot,
) -> Result<(), KillSwitchError> {
    let snapshot_json = serde_json::to_string(snapshot).map_err(|error| {
        tracing::error!(error = %error, "sérialisation d'un snapshot système échouée");
        KillSwitchError::Persistence(error.to_string())
    })?;

    storage::events::record_system_event(storage, label, &snapshot_json).map_err(|error| {
        tracing::error!(error = %error, "écriture de system_events (storage) échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}
