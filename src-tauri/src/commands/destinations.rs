//! Commandes IPC pour l'écran Vue par destination (UI_SPEC.md #4).

use super::mock_data;
use super::types::DestinationInfo;

/// EPIC 5/6 remplaceront ce mock par correlation::list_destinations() + storage.
#[tauri::command]
pub fn list_destinations() -> Vec<DestinationInfo> {
    mock_data::destinations()
}

/// EPIC 5/6 remplaceront ce mock par correlation::get_destination(domain) + storage.
#[tauri::command]
pub fn get_destination_detail(domain: String) -> Option<DestinationInfo> {
    mock_data::destinations()
        .into_iter()
        .find(|d| d.domain == domain)
}
