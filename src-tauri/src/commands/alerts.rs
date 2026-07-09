//! Commandes IPC pour l'écran Alertes & Règles (UI_SPEC.md #7).
//!
//! PLAN.md §6decies (décision explicite de Chris, 2026-07-10) : stub honnête vide — pas de
//! moteur d'évaluation ni de table `alert_rules`/`alert_events` dans cette passe. Aucune règle
//! fictive pré-remplie, aucun événement fabriqué à partir de flows : le frontend doit afficher
//! un état vide clair, jamais un faux sentiment de couverture. Un futur EPIC dédié construira
//! la persistance + le moteur temps réel.

use super::types::{AlertEvent, AlertRule};

#[tauri::command]
pub fn list_alert_rules() -> Vec<AlertRule> {
    Vec::new()
}

/// Rien n'est persisté : il n'existe aucune règle à basculer dans cette passe.
#[tauri::command]
pub fn toggle_alert_rule(id: String) -> bool {
    let _ = id;
    false
}

/// Opération en mémoire non persistée (comme documenté avant cette passe) : le frontend garde
/// l'objet retourné en état local, rien n'est relu au prochain `list_alert_rules`.
#[tauri::command]
pub fn create_alert_rule(name: String, description: String, criteria: String) -> AlertRule {
    AlertRule {
        id: format!("r-{}", uuid_like()),
        name,
        description,
        criteria,
        active: true,
        trigger_count: 0,
    }
}

/// Même raisonnement que `create_alert_rule` : aucune règle persistée à relire, `active`/
/// `trigger_count` retombent sur leurs défauts plutôt que sur un ancien état fictif.
#[tauri::command]
pub fn update_alert_rule(
    id: String,
    name: String,
    description: String,
    criteria: String,
) -> AlertRule {
    AlertRule {
        id,
        name,
        description,
        criteria,
        active: true,
        trigger_count: 0,
    }
}

#[tauri::command]
pub fn delete_alert_rule(id: String) {
    let _ = id;
}

#[tauri::command]
pub fn list_alert_events(rule_id: Option<String>) -> Vec<AlertEvent> {
    let _ = rule_id;
    Vec::new()
}

fn uuid_like() -> String {
    format!(
        "{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}
