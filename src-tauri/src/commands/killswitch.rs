//! Commandes IPC pour le panneau Kill Switch (UI_SPEC.md #8).

use super::mock_data;
use super::types::{SystemStatus, TeardownReport};

/// EPIC 7.2 remplacera ce mock par killswitch::activate() (séquence orchestrée réelle).
#[tauri::command]
pub fn activate_vitrail() -> SystemStatus {
    SystemStatus {
        kill_switch_state: "active".into(),
        subsystems: mock_data::subsystems(true),
        last_verification: Some("14:58:22".into()),
        last_verification_clean: true,
    }
}

/// EPIC 7.3 remplacera ce mock par killswitch::deactivate() (séquence inverse + diff).
#[tauri::command]
pub fn deactivate_vitrail() -> SystemStatus {
    SystemStatus {
        kill_switch_state: "inactive".into(),
        subsystems: mock_data::subsystems(false),
        last_verification: Some("14:58:22".into()),
        last_verification_clean: true,
    }
}

/// EPIC 7.5 remplacera ce mock par killswitch::emergency_stop() (best-effort, hors séquence).
/// Description corrigée (MOCKUP_REVIEW.md #3) : "Force la désactivation immédiate de tous
/// les sous-systèmes sans séquence orchestrée."
#[tauri::command]
pub fn emergency_stop() -> SystemStatus {
    SystemStatus {
        kill_switch_state: "inactive".into(),
        subsystems: mock_data::subsystems(false),
        last_verification: None,
        last_verification_clean: false,
    }
}

/// EPIC 7 remplacera ce mock par killswitch::current_status().
#[tauri::command]
pub fn get_system_status() -> SystemStatus {
    SystemStatus {
        kill_switch_state: "active".into(),
        subsystems: mock_data::subsystems(true),
        last_verification: Some("14:58:22".into()),
        last_verification_clean: true,
    }
}

/// EPIC 7.4 remplacera ce mock par killswitch::verify_teardown() (diff snapshot pré/post).
#[tauri::command]
pub fn verify_teardown() -> TeardownReport {
    TeardownReport {
        clean: true,
        divergences: vec![],
        checked_at: "14:58:22".into(),
    }
}
