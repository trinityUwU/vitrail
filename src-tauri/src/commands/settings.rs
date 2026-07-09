//! Commandes IPC pour l'écran Paramètres (UI_SPEC.md #9) et le Journal système (#11).

use tauri::State;

use crate::storage::sessions::SessionRow;
use crate::storage::{self, StorageHandle};

use super::mock_data::{MONITORED_INTERFACES, NFTABLES_CHAIN};
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

/// EPIC 6 (écran Historique #12) : dérive les sessions réelles depuis `system_events` (paires
/// pre-activation/post-deactivation, `storage::sessions`). Décision non explicitement tranchée
/// dans PLAN.md §6sexies : rendue réelle en même temps que `get_session_detail`/
/// `delete_session` — sinon un clic sur une session mockée (`s1`) ne retrouverait jamais son
/// détail réel (ids incompatibles), rompant le flux fonctionnel de l'écran Historique.
#[tauri::command]
pub fn list_sessions(storage: State<'_, StorageHandle>) -> Vec<Session> {
    match storage::sessions::list_sessions(&storage) {
        Ok(rows) => rows
            .into_iter()
            .map(|row| to_wire_session(&storage, row))
            .collect(),
        Err(error) => {
            tracing::error!(error = %error, "list_sessions (storage) échoué");
            Vec::new()
        }
    }
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

/// EPIC 6 (6.6) : purge réelle du journal système (`system_events`) via `storage::retention`.
#[tauri::command]
pub fn purge_logs(storage: State<'_, StorageHandle>) -> u64 {
    storage::retention::purge_logs(&storage).unwrap_or_else(|error| {
        tracing::error!(error = %error, "purge_logs (storage) échoué");
        0
    })
}

/// EPIC 6 (6.6) : purge réelle (totale ou avant une date `YYYY-MM-DD`, format de
/// `<input type="date">` côté frontend) via `storage::retention` (`DELETE` + `VACUUM`).
/// `before: Some(s)` avec `s` illisible est une erreur explicite — jamais confondu avec
/// `before: None` (pas de borne demandée), qui seul déclenche la purge totale.
#[tauri::command]
pub fn purge_data(
    storage: State<'_, StorageHandle>,
    before: Option<String>,
) -> Result<PurgeResult, String> {
    purge_data_impl(&storage, before)
}

/// Cœur de `purge_data`, extrait pour être testable sans `tauri::State` (non constructible
/// hors du runtime Tauri). `before: Some(s)` avec `s` illisible échoue explicitement — jamais
/// confondu avec `before: None` (pas de borne demandée), seul cas déclenchant la purge totale.
fn purge_data_impl(storage: &StorageHandle, before: Option<String>) -> Result<PurgeResult, String> {
    let before_unix = match before {
        Some(ref s) => Some(
            parse_date_to_unix(s)
                .ok_or_else(|| format!("date de purge invalide : « {s} » (attendu AAAA-MM-JJ)"))?,
        ),
        None => None,
    };
    storage::retention::purge_data_before(storage, before_unix)
        .map(|stats| PurgeResult {
            deleted_flows: stats.deleted_rows,
            freed_mb: (stats.freed_bytes as f64 / (1024.0 * 1024.0)).max(0.0),
        })
        .map_err(|error| {
            tracing::error!(error = %error, "purge_data (storage) échoué");
            "échec de la purge des données".to_string()
        })
}

/// EPIC 6 : requête réelle via `storage::sessions`. `flows` reste vide (table `flows` non
/// alimentée avant EPIC 5/corrélation) — attendu, cf. rapport de livraison.
#[tauri::command]
pub fn get_session_detail(storage: State<'_, StorageHandle>, id: String) -> Option<SessionDetail> {
    match storage::sessions::get_session(&storage, &id) {
        Ok(Some(row)) => Some(SessionDetail {
            session: to_wire_session(&storage, row),
            flows: Vec::new(),
        }),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(error = %error, id, "get_session_detail (storage) échoué");
            None
        }
    }
}

/// EPIC 6 : suppression réelle via `storage::sessions` (supprime les événements bornant la
/// session — pas de table `sessions` dédiée, cf. `storage::sessions`).
#[tauri::command]
pub fn delete_session(storage: State<'_, StorageHandle>, id: String) {
    if let Err(error) = storage::sessions::delete_session(&storage, &id) {
        tracing::error!(error = %error, id, "delete_session (storage) échoué");
    }
}

/// Convertit une ligne storage (timestamps unix bruts) vers le type de wire IPC — reste hors
/// de `storage/` (présentation, pas persistance). `volume_mb` est dérivé de `capture_events`
/// dans la fenêtre de la session (seule donnée réellement disponible) ; `process_count`/
/// `alert_count` restent à 0 tant que `processes`/les alertes ne sont pas des domaines
/// persistés (EPIC 5, hors périmètre EPIC 6 — signalé au rapport de livraison).
fn to_wire_session(storage: &StorageHandle, row: SessionRow) -> Session {
    let volume_bytes = storage::sessions::session_volume_bytes(
        storage,
        row.started_at_unix,
        row.ended_at_unix,
    )
    .unwrap_or_else(|error| {
        tracing::error!(error = %error, session_id = %row.id, "session_volume_bytes échoué");
        0
    });

    Session {
        id: row.id,
        started_at: format_unix_rfc3339(row.started_at_unix),
        ended_at: format_unix_rfc3339(row.ended_at_unix),
        volume_mb: volume_bytes as f64 / (1024.0 * 1024.0),
        process_count: 0,
        alert_count: 0,
    }
}

/// Parse une date `YYYY-MM-DD` (`<input type="date">`) en timestamp unix (minuit UTC).
/// Algorithme de Howard Hinnant (jours depuis epoch civil) — évite d'ajouter une dépendance
/// chrono/time pour cette seule conversion. http://howardhinnant.github.io/date_algorithms.html
fn parse_date_to_unix(date: &str) -> Option<i64> {
    let mut parts = date.splitn(3, '-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !is_valid_civil_date(y, m, d) {
        return None;
    }
    Some(days_from_civil(y, m, d) * 86_400)
}

/// Rejette les dates calendaires impossibles (`2026-02-30`, `2026-04-31`, ...) que
/// `days_from_civil` accepterait silencieusement en les décalant vers le jour suivant.
fn is_valid_civil_date(y: i64, m: i64, d: i64) -> bool {
    if !(1..=12).contains(&m) || d < 1 {
        return false;
    }
    d <= days_in_month(y, m)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    const DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if m == 2 && is_leap_year(y) {
        29
    } else {
        DAYS[(m - 1) as usize]
    }
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn format_unix_rfc3339(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let secs_of_day = ts.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let h = secs_of_day / 3600;
    let min = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{s:02}Z")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purge_data_avec_date_illisible_renvoie_une_erreur_sans_purger() {
        let storage = StorageHandle::open_in_memory().expect("ouverture en mémoire");
        storage::events::record_system_event(&storage, "pre-activation", "{}")
            .expect("event de test");

        let result = purge_data_impl(&storage, Some("format-cassé".into()));
        assert!(
            result.is_err(),
            "une date fournie mais illisible doit échouer, jamais purger silencieusement"
        );

        // La purge n'a pas dû avoir lieu : l'event de test est toujours purgeable ensuite.
        let stats = storage::retention::purge_data_before(&storage, None).expect("purge totale");
        assert_eq!(
            stats.deleted_rows, 1,
            "l'event de test doit toujours être présent après l'échec du parsing de date"
        );
    }

    #[test]
    fn purge_data_sans_borne_reste_une_purge_totale() {
        let storage = StorageHandle::open_in_memory().expect("ouverture en mémoire");
        storage::events::record_system_event(&storage, "pre-activation", "{}")
            .expect("event de test");

        let result = purge_data_impl(&storage, None);
        assert!(
            result.is_ok(),
            "before: None doit rester la purge totale explicite"
        );
    }

    #[test]
    fn parse_date_to_unix_rejette_les_dates_calendaires_invalides() {
        assert!(parse_date_to_unix("2026-02-30").is_none());
        assert!(parse_date_to_unix("2026-04-31").is_none());
        assert!(parse_date_to_unix("2026-13-01").is_none());
        assert!(parse_date_to_unix("2026-00-10").is_none());
        assert!(
            parse_date_to_unix("2026-02-29").is_none(),
            "2026 n'est pas bissextile"
        );
        assert!(
            parse_date_to_unix("2024-02-29").is_some(),
            "2024 est bissextile"
        );
        assert!(parse_date_to_unix("2026-02-28").is_some());
    }
}
