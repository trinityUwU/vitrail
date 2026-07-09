//! Commandes IPC pour l'écran Paramètres (UI_SPEC.md #9) et le Journal système (#11).

use super::mock_data::{MONITORED_INTERFACES, NFTABLES_CHAIN};
use super::mock_flows;
use super::types::{Exclusion, LogEntry, PurgeResult, Session, SessionDetail, Settings};

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

/// EPIC 6 remplacera ce mock par storage::purge_logs() (troncature du journal persistant).
#[tauri::command]
pub fn purge_logs() -> u64 {
    get_log_entries().len() as u64
}

/// EPIC 6 remplacera ce mock par storage::purge_data(before) (DELETE ciblé + VACUUM SQLite).
#[tauri::command]
pub fn purge_data(before: Option<String>) -> PurgeResult {
    match before {
        Some(_) => PurgeResult {
            deleted_flows: 420,
            freed_mb: 18.6,
        },
        None => PurgeResult {
            deleted_flows: mock_flows::flows().len() as u64,
            freed_mb: 42.3,
        },
    }
}

/// EPIC 6 remplacera ce mock par storage::get_session(id) (flux réels de la session).
#[tauri::command]
pub fn get_session_detail(id: String) -> Option<SessionDetail> {
    let session = list_sessions().into_iter().find(|s| s.id == id)?;
    Some(SessionDetail {
        session,
        flows: mock_flows::flows(),
    })
}

/// EPIC 6 remplacera ce mock par storage::delete_session(id).
#[tauri::command]
pub fn delete_session(id: String) {
    let _ = id;
}

const SEED_KEYLOG_APPS: [&str; 3] = [
    "/usr/bin/google-chrome-stable",
    "/usr/lib/firefox/firefox",
    "/usr/share/code/code",
];

/// EPIC 3.5 remplacera ce mock par keylog::list_covered_apps() (config persistée).
#[tauri::command]
pub fn list_keylog_apps() -> Vec<String> {
    SEED_KEYLOG_APPS.iter().map(|s| s.to_string()).collect()
}

/// EPIC 3.5 remplacera ce mock par keylog::add_covered_app(path) (injection SSLKEYLOGFILE).
#[tauri::command]
pub fn add_keylog_app(path: String) -> Vec<String> {
    let mut apps = list_keylog_apps();
    if !apps.contains(&path) {
        apps.push(path);
    }
    apps
}

/// EPIC 3.5 remplacera ce mock par keylog::remove_covered_app(path).
#[tauri::command]
pub fn remove_keylog_app(path: String) -> Vec<String> {
    list_keylog_apps()
        .into_iter()
        .filter(|a| a != &path)
        .collect()
}
