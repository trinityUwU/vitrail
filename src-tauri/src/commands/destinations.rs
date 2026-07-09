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

/// EPIC 6 remplacera ce mock par storage::tag_destination() (tag persisté en base).
/// Pas de persistance côté backend mock : le frontend garde l'objet retourné en état local.
#[tauri::command]
pub fn tag_destination(domain: String, tag: String) -> DestinationInfo {
    let mut destination = mock_data::destinations()
        .into_iter()
        .find(|d| d.domain == domain)
        .unwrap_or(DestinationInfo {
            domain: domain.clone(),
            ip: "0.0.0.0".into(),
            volume_mb: 0.0,
            process_count: 0,
            visibility: super::types::FlowVisibility::Unknown,
            tls: false,
            pinning: false,
            first_seen: "—".into(),
            last_seen: "—".into(),
            tag: None,
        });
    destination.tag = Some(tag);
    destination
}
