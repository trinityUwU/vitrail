//! Commandes IPC pour l'écran Vue par processus (UI_SPEC.md #3).

use super::mock_data;
use super::types::ProcessInfo;

/// EPIC 1/6 remplaceront ce mock par attribution::list_known_processes() + storage.
#[tauri::command]
pub fn list_processes() -> Vec<ProcessInfo> {
    mock_data::processes()
}

/// EPIC 1/6 remplaceront ce mock par attribution::get_process(name) + storage.
#[tauri::command]
pub fn get_process_detail(name: String) -> Option<ProcessInfo> {
    mock_data::processes().into_iter().find(|p| p.name == name)
}
