//! Commandes IPC pour l'écran Vue par destination (UI_SPEC.md #4).

use tauri::State;

use crate::storage::{self, StorageHandle};

use super::types::{DestinationInfo, FlowVisibility};

/// PLAN.md §6decies : `storage::aggregates::list_destinations_aggregated` remplace
/// `mock_data::destinations()` (group by `destination` sur `flows`, tag fusionné depuis
/// `destination_tags`).
#[tauri::command]
pub fn list_destinations(storage: State<'_, StorageHandle>) -> Vec<DestinationInfo> {
    storage::aggregates::list_destinations_aggregated(&storage)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "list_destinations_aggregated (storage) échoué");
            Vec::new()
        })
        .into_iter()
        .map(DestinationInfo::from)
        .collect()
}

#[tauri::command]
pub fn get_destination_detail(
    storage: State<'_, StorageHandle>,
    domain: String,
) -> Option<DestinationInfo> {
    storage::aggregates::get_destination_aggregated(&storage, &domain)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, domain, "get_destination_aggregated (storage) échoué");
            None
        })
        .map(DestinationInfo::from)
}

/// PLAN.md §6decies (EPIC 6.3, jamais fait) : persistance réelle via `storage::destinations`.
/// Une destination jamais vue dans `flows` peut quand même être taguée à l'avance (même
/// contrat que le mock précédent) — repli sur une entrée à volume nul si `flows` ne la connaît
/// pas encore, plutôt qu'un échec de la commande.
#[tauri::command]
pub fn tag_destination(
    storage: State<'_, StorageHandle>,
    domain: String,
    tag: String,
) -> DestinationInfo {
    if let Err(error) = storage::destinations::set_tag(&storage, &domain, &tag) {
        tracing::error!(error = %error, domain, "tag_destination (storage) échoué");
    }
    storage::aggregates::get_destination_aggregated(&storage, &domain)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, domain, "get_destination_aggregated après tag échoué");
            None
        })
        .map(DestinationInfo::from)
        .unwrap_or_else(|| DestinationInfo {
            domain,
            ip: "0.0.0.0".into(),
            volume_mb: 0.0,
            process_count: 0,
            visibility: FlowVisibility::Unknown,
            tls: false,
            pinning: false,
            first_seen: "—".into(),
            last_seen: "—".into(),
            tag: Some(tag),
        })
}
