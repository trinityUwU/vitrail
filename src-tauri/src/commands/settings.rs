//! Commandes IPC pour l'écran Paramètres (UI_SPEC.md #9) et le Journal système (#11).

use super::mock_data::{MONITORED_INTERFACES, NFTABLES_CHAIN};
use super::types::{AlertRule, Exclusion, LogEntry, Session, Settings};

/// EPIC 6/9 remplaceront ce mock par storage::get_settings() (config TOML utilisateur).
#[tauri::command]
pub fn get_settings() -> Settings {
    Settings {
        ca_fingerprint: "4A:2B:C1:D8:...:F1:E3:00".into(),
        ca_trust_store_installed: true,
        nftables_chain: NFTABLES_CHAIN.into(),
        monitored_interfaces: MONITORED_INTERFACES.iter().map(|s| s.to_string()).collect(),
        retention_days: None,
        database_size_mb: 42.3,
        notifications_enabled: true,
        notification_sound: false,
    }
}

/// EPIC 6/9 remplaceront ce mock par storage::update_settings(config).
#[tauri::command]
pub fn update_settings(settings: Settings) -> Settings {
    settings
}

/// EPIC 4.5 remplacera ce mock par decryption::add_exclusion() (appliqué en amont nftables).
#[tauri::command]
pub fn add_exclusion(name: String, kind: String) -> Exclusion {
    Exclusion { name, kind }
}

/// EPIC 4.5 remplacera ce mock par decryption::remove_exclusion().
#[tauri::command]
pub fn remove_exclusion(name: String) -> bool {
    !name.is_empty()
}

/// EPIC 4.1 remplacera ce mock par decryption::rotate_ca() (génération + réinstallation CA).
#[tauri::command]
pub fn rotate_ca() -> Settings {
    get_settings()
}

/// EPIC 6.5 remplacera ce mock par storage::export_config() (JSON, config uniquement).
#[tauri::command]
pub fn export_config() -> String {
    serde_json::to_string_pretty(&get_settings()).unwrap_or_default()
}

/// EPIC 6.5 remplacera ce mock par storage::import_config(payload).
#[tauri::command]
pub fn import_config(payload: String) -> Result<Settings, String> {
    serde_json::from_str(&payload).map_err(|e| e.to_string())
}

/// EPIC 5/7 remplaceront ce mock par correlation::list_alert_rules() (écran Alertes #7).
#[tauri::command]
pub fn list_alert_rules() -> Vec<AlertRule> {
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

/// EPIC 5/7 remplaceront ce mock par correlation::toggle_alert_rule(id).
#[tauri::command]
pub fn toggle_alert_rule(id: String) -> bool {
    !id.is_empty()
}

/// EPIC 6 remplacera ce mock par storage::list_sessions() (écran Historique #12).
#[tauri::command]
pub fn list_sessions() -> Vec<Session> {
    vec![
        Session {
            id: "s1".into(),
            started_at: "2026-07-07T14:00:00Z".into(),
            ended_at: "2026-07-07T16:00:00Z".into(),
            volume_mb: 1200.0,
            process_count: 8,
            alert_count: 3,
        },
        Session {
            id: "s2".into(),
            started_at: "2026-07-08T14:00:00Z".into(),
            ended_at: "2026-07-08T15:30:00Z".into(),
            volume_mb: 800.0,
            process_count: 6,
            alert_count: 1,
        },
        Session {
            id: "s3".into(),
            started_at: "2026-07-09T02:00:00Z".into(),
            ended_at: "2026-07-09T05:00:00Z".into(),
            volume_mb: 2100.0,
            process_count: 11,
            alert_count: 5,
        },
    ]
}

/// EPIC 6/9 remplaceront ce mock par storage::query_logs() (écran Journal système #11).
#[tauri::command]
pub fn get_log_entries() -> Vec<LogEntry> {
    vec![
        LogEntry {
            time: "14:58:22".into(),
            level: "info".into(),
            subsystem: "killswitch".into(),
            message: "Vérification post-activation terminée — conforme".into(),
        },
        LogEntry {
            time: "14:58:18".into(),
            level: "info".into(),
            subsystem: "capture".into(),
            message: format!("Chaîne nftables {NFTABLES_CHAIN} active sur wlan0, wg0"),
        },
        LogEntry {
            time: "14:58:15".into(),
            level: "info".into(),
            subsystem: "keylog".into(),
            message: "Fichier SSLKEYLOGFILE initialisé : ~/.vitrail/keylog/sslkeys.log".into(),
        },
        LogEntry {
            time: "14:58:12".into(),
            level: "info".into(),
            subsystem: "decryption".into(),
            message: "PolarProxy démarré (PID 8842), port 8081, CA chargée".into(),
        },
        LogEntry {
            time: "14:58:08".into(),
            level: "info".into(),
            subsystem: "attribution".into(),
            message: "OpenSnitch daemon détecté (v1.6.6), connecté via socket UNIX".into(),
        },
        LogEntry {
            time: "14:57:55".into(),
            level: "warn".into(),
            subsystem: "decryption".into(),
            message:
                "PolarProxy : le flux vers discord.gg a été mis en fail-open (pinning détecté)"
                    .into(),
        },
        LogEntry {
            time: "14:45:12".into(),
            level: "error".into(),
            subsystem: "attribution".into(),
            message: "Timeout de connexion au socket OpenSnitch — nouvelle tentative dans 2s"
                .into(),
        },
    ]
}
