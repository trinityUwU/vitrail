//! Types communs, config, logging (tracing). Pas de logique métier.
//!
//! `SystemStatus`/`SubsystemStatus`/`TeardownReport` vivent ici (et non dans
//! `commands/types.rs`) car ils sont produits par `killswitch/` (EPIC 7) : `commands/`
//! n'agrège/délègue jamais l'inverse de ce sens (ARCHITECTURE.md). `commands/types.rs` les
//! ré-exporte tels quels pour préserver le contrat IPC existant.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsystemStatus {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub kill_switch_state: String,
    pub subsystems: Vec<SubsystemStatus>,
    pub last_verification: Option<String>,
    pub last_verification_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeardownReport {
    pub clean: bool,
    pub divergences: Vec<String>,
    pub checked_at: String,
}
