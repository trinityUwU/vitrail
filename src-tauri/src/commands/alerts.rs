//! Commandes IPC pour l'écran Alertes & Règles (UI_SPEC.md #7) — CRUD des règles et
//! historique de leurs déclenchements. Regroupe `list_alert_rules`/`toggle_alert_rule`
//! (déjà présentes avant cette passe, déplacées ici depuis `settings.rs`).

use super::mock_flows;
use super::types::{AlertEvent, AlertRule, FlowVisibility};

fn seed_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            id: "r1".into(),
            name: "Nouveau processus détecté".into(),
            description: "Déclenché quand un processus jamais vu initie une connexion".into(),
            criteria: "processus = nouveau".into(),
            active: true,
            trigger_count: 2,
        },
        AlertRule {
            id: "r2".into(),
            name: "Destination surveillée contactée".into(),
            description: "Notifie si une destination taguée \"à surveiller\" est contactée".into(),
            criteria: "destination.tag = surveillé".into(),
            active: true,
            trigger_count: 1,
        },
        AlertRule {
            id: "r3".into(),
            name: "Changement de visibilité inattendu".into(),
            description: "Un processus en Métadonnées passe soudain en Déchiffré — signal de dégradation potentielle".into(),
            criteria: "visibilité: meta -> fully".into(),
            active: false,
            trigger_count: 0,
        },
    ]
}

/// EPIC 5/7 remplaceront ce mock par correlation::list_alert_rules() (écran Alertes #7).
#[tauri::command]
pub fn list_alert_rules() -> Vec<AlertRule> {
    seed_alert_rules()
}

/// EPIC 5/7 remplaceront ce mock par correlation::toggle_alert_rule(id).
#[tauri::command]
pub fn toggle_alert_rule(id: String) -> bool {
    !id.is_empty()
}

/// EPIC 5/7 remplaceront ce mock par correlation::create_alert_rule() (règle persistée).
/// Pas de persistance côté backend mock : le frontend garde l'objet retourné en état local.
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

/// EPIC 5/7 remplaceront ce mock par correlation::update_alert_rule(id, ...).
#[tauri::command]
pub fn update_alert_rule(
    id: String,
    name: String,
    description: String,
    criteria: String,
) -> AlertRule {
    let existing = seed_alert_rules().into_iter().find(|r| r.id == id);
    let (active, trigger_count) = existing
        .map(|r| (r.active, r.trigger_count))
        .unwrap_or((true, 0));
    AlertRule {
        id,
        name,
        description,
        criteria,
        active,
        trigger_count,
    }
}

/// EPIC 5/7 remplaceront ce mock par correlation::delete_alert_rule(id).
#[tauri::command]
pub fn delete_alert_rule(id: String) {
    let _ = id;
}

/// EPIC 5/7 remplaceront ce mock par correlation::list_alert_events(rule_id) (historique
/// des déclenchements, stocké par `storage`).
#[tauri::command]
pub fn list_alert_events(rule_id: Option<String>) -> Vec<AlertEvent> {
    let flows = mock_flows::flows();
    let rules = seed_alert_rules();
    let ids: Vec<String> = match rule_id {
        Some(id) => vec![id],
        None => rules.iter().map(|r| r.id.clone()).collect(),
    };
    ids.into_iter()
        .flat_map(|rid| build_events_for_rule(&rid, &flows))
        .collect()
}

fn build_events_for_rule(rule_id: &str, flows: &[super::types::Flow]) -> Vec<AlertEvent> {
    flows
        .iter()
        .filter(|f| matches!(f.visibility, FlowVisibility::Meta | FlowVisibility::Attrib))
        .take(2)
        .map(|f| AlertEvent {
            id: format!("evt-{rule_id}-{}", f.id),
            rule_id: rule_id.into(),
            flow_id: f.id.clone(),
            time: f.timestamp.clone(),
            summary: format!("{} → {}", f.process, f.destination),
            visibility: f.visibility,
        })
        .collect()
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
