//! Commandes IPC pour l'écran Vue par processus (UI_SPEC.md #3).

use tauri::State;

use crate::storage::{self, StorageHandle};

use super::types::ProcessInfo;

/// PLAN.md §6decies : `storage::aggregates::list_processes_aggregated` remplace
/// `mock_data::processes()` (group by `process` sur `flows`).
#[tauri::command]
pub fn list_processes(storage: State<'_, StorageHandle>) -> Vec<ProcessInfo> {
    storage::aggregates::list_processes_aggregated(&storage)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "list_processes_aggregated (storage) échoué");
            Vec::new()
        })
        .into_iter()
        .map(ProcessInfo::from)
        .collect()
}

#[tauri::command]
pub fn get_process_detail(storage: State<'_, StorageHandle>, name: String) -> Option<ProcessInfo> {
    storage::aggregates::get_process_aggregated(&storage, &name)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, name, "get_process_aggregated (storage) échoué");
            None
        })
        .map(ProcessInfo::from)
}
