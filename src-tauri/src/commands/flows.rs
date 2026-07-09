//! Commandes IPC pour la Timeline, l'Inspecteur de flux et la Recherche avancée
//! (UI_SPEC.md #2, #5, #6). EPIC 5 : branché sur `storage::flows` (flux réels persistés par
//! le moteur de corrélation), remplace `mock_flows::flows()`.

use tauri::State;

use crate::storage::{self, StorageHandle};

use super::types::Flow;

/// Borne de la Timeline (story 8.1) — même ordre de grandeur que ce que le mock exposait.
const DEFAULT_FLOW_LIMIT: u32 = 200;

#[tauri::command]
pub fn list_flows(storage: State<'_, StorageHandle>) -> Vec<Flow> {
    storage::flows::list_flows(&storage, DEFAULT_FLOW_LIMIT).unwrap_or_else(|error| {
        tracing::error!(error = %error, "list_flows (storage) échoué");
        Vec::new()
    })
}

#[tauri::command]
pub fn get_flow_detail(storage: State<'_, StorageHandle>, id: String) -> Option<Flow> {
    storage::flows::get_flow(&storage, &id).unwrap_or_else(|error| {
        tracing::error!(error = %error, id = %id, "get_flow_detail (storage) échoué");
        None
    })
}

/// FTS5 réel (`storage::flows::search_flows`) — remplace le filtre `contains` en mémoire du
/// mock (5.4/6.4).
#[tauri::command]
pub fn search_flows(storage: State<'_, StorageHandle>, query: String) -> Vec<Flow> {
    storage::flows::search_flows(&storage, &query).unwrap_or_else(|error| {
        tracing::error!(error = %error, query = %query, "search_flows (storage) échoué");
        Vec::new()
    })
}
