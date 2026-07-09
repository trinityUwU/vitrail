//! Commandes IPC pour l'écran Recherche avancée (UI_SPEC.md #6) — requêtes sauvegardées et
//! conversion d'une requête en règle d'alerte.

use super::types::{AlertRule, SavedQuery, SearchCriteria};

/// EPIC 6 remplacera ce mock par storage::save_search_query() (requête persistée en base).
/// Pas de persistance côté backend mock : le frontend garde l'objet retourné en état local.
#[tauri::command]
pub fn save_search_query(name: String, criteria: SearchCriteria) -> SavedQuery {
    SavedQuery {
        id: format!("q-{}", query_id_seed(&name)),
        name,
        criteria,
    }
}

/// EPIC 6 remplacera ce mock par storage::list_saved_queries().
#[tauri::command]
pub fn list_saved_queries() -> Vec<SavedQuery> {
    Vec::new()
}

/// EPIC 6 remplacera ce mock par storage::delete_saved_query(id).
#[tauri::command]
pub fn delete_saved_query(id: String) {
    let _ = id;
}

/// EPIC 6/7 remplaceront ce mock par correlation::convert_query_to_alert() (requête stockée
/// convertie en règle d'alerte active).
#[tauri::command]
pub fn convert_query_to_alert(query_id: String, alert_name: String) -> AlertRule {
    AlertRule {
        id: format!("r-from-{query_id}"),
        name: alert_name,
        description: "Règle générée depuis une requête de recherche sauvegardée".into(),
        criteria: query_id,
        active: true,
        trigger_count: 0,
    }
}

fn query_id_seed(name: &str) -> String {
    let sum: u32 = name.bytes().map(u32::from).sum();
    format!("{sum:x}")
}
