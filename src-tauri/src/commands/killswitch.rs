//! Commandes IPC pour le panneau Kill Switch (UI_SPEC.md #8). Délègue intégralement à
//! `killswitch::KillSwitchState` (EPIC 7) — aucune logique métier ici, agrégation pure.

use super::types::{SystemStatus, TeardownReport};
use crate::killswitch::KillSwitchState;

#[tauri::command]
pub fn activate_vitrail(state: tauri::State<'_, KillSwitchState>) -> SystemStatus {
    state.activate()
}

#[tauri::command]
pub fn deactivate_vitrail(state: tauri::State<'_, KillSwitchState>) -> SystemStatus {
    state.deactivate()
}

/// "Force la désactivation immédiate de tous les sous-systèmes sans séquence orchestrée."
/// (description corrigée, MOCKUP_REVIEW.md #3).
#[tauri::command]
pub fn emergency_stop(state: tauri::State<'_, KillSwitchState>) -> SystemStatus {
    state.emergency_stop()
}

#[tauri::command]
pub fn get_system_status(state: tauri::State<'_, KillSwitchState>) -> SystemStatus {
    state.current_status()
}

#[tauri::command]
pub fn verify_teardown(state: tauri::State<'_, KillSwitchState>) -> TeardownReport {
    state.verify_teardown()
}
