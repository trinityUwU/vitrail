//! Commandes IPC pour la Timeline, l'Inspecteur de flux et la Recherche avancée
//! (UI_SPEC.md #2, #5, #6).

use super::mock_data;
use super::types::Flow;

/// EPIC 5/6 remplaceront ce mock par correlation::list_flows() + storage::query_flows().
#[tauri::command]
pub fn list_flows() -> Vec<Flow> {
    mock_data::flows()
}

/// EPIC 6 remplacera ce mock par storage::get_flow(id).
#[tauri::command]
pub fn get_flow_detail(id: String) -> Option<Flow> {
    mock_data::flows().into_iter().find(|f| f.id == id)
}

/// EPIC 6 remplacera ce mock par storage::search_flows() (FTS5 sur le contenu déchiffré).
#[tauri::command]
pub fn search_flows(query: String) -> Vec<Flow> {
    let needle = query.to_lowercase();
    mock_data::flows()
        .into_iter()
        .filter(|f| {
            needle.is_empty()
                || f.process.to_lowercase().contains(&needle)
                || f.destination.to_lowercase().contains(&needle)
        })
        .collect()
}
