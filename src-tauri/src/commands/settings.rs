//! Commandes IPC pour l'écran Paramètres (UI_SPEC.md #9) et le Journal système (#11).

use tauri::State;

use crate::decryption::{self, SystemHelperBackend};
use crate::storage::sessions::SessionRow;
use crate::storage::{self, StorageHandle};

use super::types::{Exclusion, PurgeResult, Session, SessionDetail, Settings};

pub mod log_entries;

/// Constantes de config système réelles (pas des valeurs mock) — portées ici depuis
/// `mock_data.rs` (supprimé, PLAN.md §6decies) dont elles étaient le seul contenu non fictif.
const NFTABLES_CHAIN: &str = "VITRAIL_REDIRECT";
const MONITORED_INTERFACES: [&str; 3] = ["wlan0", "wg0", "enp3s0"];

/// EPIC 6/9 remplaceront le reste de ce mock par storage::get_settings() (config TOML
/// utilisateur) — `ca_fingerprint`/`ca_trust_store_installed` sont réels depuis EPIC 4
/// (storage::decryption::get_ca), le reste (interfaces/rétention/BDD/notifications) reste
/// hors périmètre de cette passe.
#[tauri::command]
pub fn get_settings(storage: State<'_, StorageHandle>) -> Settings {
    get_settings_impl(&storage)
}

fn get_settings_impl(storage: &StorageHandle) -> Settings {
    let ca = storage::decryption::get_ca(storage).unwrap_or_else(|error| {
        tracing::error!(error = %error, "get_settings: lecture des métadonnées CA échouée");
        None
    });
    Settings {
        ca_fingerprint: ca
            .as_ref()
            .map(|meta| meta.fingerprint_sha256.clone())
            .unwrap_or_else(|| "aucune CA générée".to_string()),
        ca_trust_store_installed: ca.is_some(),
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

/// EPIC 4.5 : persistance réelle + application en amont nftables pour `kind == "destination"`
/// (résolution DNS + `nft-set-exclusions`) via `decryption::add_exclusion`. `kind ==
/// "processus"` est persisté mais jamais appliqué au niveau nftables (limite documentée,
/// `decryption::exclusions`).
///
/// Retourne `Result` (audit EPIC 4, point 5 — même pattern que `purge_data`, EPIC 6) : un échec
/// d'application nftables (persistée mais pas vraiment appliquée au niveau réseau) doit remonter
/// une erreur explicite au frontend, jamais un succès trompeur silencieusement dégradé en log.
#[tauri::command]
pub fn add_exclusion(
    storage: State<'_, StorageHandle>,
    name: String,
    kind: String,
) -> Result<Exclusion, String> {
    decryption::add_exclusion(&storage, &SystemHelperBackend, &name, &kind).map_err(|error| {
        tracing::error!(error = %error, name, kind, "add_exclusion échoué");
        format!("exclusion persistée mais non appliquée au niveau réseau : {error}")
    })?;
    Ok(Exclusion { name, kind })
}

/// EPIC 4.5 : retrait réel — recalcule et repousse la liste d'IPs exclues restantes si
/// l'exclusion retirée était de type `destination`. `Result` (point 5, même raison que
/// `add_exclusion` ci-dessus) : un échec de réapplication nftables après retrait ne doit jamais
/// être avalé en simple `false` sans message exploitable côté frontend.
#[tauri::command]
pub fn remove_exclusion(storage: State<'_, StorageHandle>, name: String) -> Result<(), String> {
    decryption::remove_exclusion(&storage, &SystemHelperBackend, &name).map_err(|error| {
        tracing::error!(error = %error, name, "remove_exclusion échoué");
        format!("exclusion retirée du stockage mais réapplication réseau échouée : {error}")
    })
}

/// EPIC 4.1 : rotation réelle (retrait de l'ancienne CA par empreinte exacte, génération +
/// installation d'une CA neuve) via `decryption::rotate_ca`.
#[tauri::command]
pub fn rotate_ca(storage: State<'_, StorageHandle>) -> Settings {
    if let Err(error) = decryption::rotate_ca(&storage, &SystemHelperBackend) {
        tracing::error!(error = %error, "rotate_ca échoué");
    }
    get_settings_impl(&storage)
}

/// EPIC 6.5 remplacera ce mock par storage::export_config() (JSON, config uniquement).
#[tauri::command]
pub fn export_config(storage: State<'_, StorageHandle>) -> String {
    serde_json::to_string_pretty(&get_settings_impl(&storage)).unwrap_or_default()
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

/// EPIC 3.5 : liste réelle depuis `storage::keylog` (remplace le mock en mémoire). Vide par
/// défaut — décision non explicitement tranchée dans PLAN.md : les 3 apps mock du placeholder
/// UI référençaient des chemins non vérifiés sur la machine réelle ; une liste vide honnête est
/// préférable à une fausse impression de couverture pré-remplie (story 3.5, "pas de faux
/// sentiment de couverture totale") — l'utilisateur ajoute lui-même ses apps.
#[tauri::command]
pub fn list_keylog_apps(storage: State<'_, StorageHandle>) -> Vec<String> {
    list_keylog_apps_impl(&storage)
}

fn list_keylog_apps_impl(storage: &StorageHandle) -> Vec<String> {
    storage::keylog::list_apps(storage)
        .map(|rows| rows.into_iter().map(|row| row.binary_path).collect())
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "list_keylog_apps (storage) échoué");
            Vec::new()
        })
}

/// EPIC 3.5 : persiste réellement via `storage::keylog` — l'injection effective (wrapper +
/// surcharge `.desktop`) n'a lieu qu'au prochain `KeylogSubsystem::start()` (activation du kill
/// switch), pas immédiatement à l'ajout (cf. `keylog::subsystem`).
#[tauri::command]
pub fn add_keylog_app(storage: State<'_, StorageHandle>, path: String) -> Vec<String> {
    if let Err(error) = storage::keylog::add_app(&storage, &path) {
        tracing::error!(error = %error, path, "add_keylog_app (storage) échoué");
    }
    list_keylog_apps_impl(&storage)
}

/// EPIC 3.5 : persiste réellement via `storage::keylog`. Restaure d'abord une éventuelle
/// injection active (`keylog::restore_app_injection`) AVANT de supprimer la ligne — sinon une
/// app retirée pendant que le kill switch est actif laisserait sa surcharge `.desktop`
/// orpheline, jamais restaurée par un futur `stop()` qui ne la connaîtrait plus (décision non
/// explicite dans PLAN.md, tranchée au rapport de livraison EPIC 3).
#[tauri::command]
pub fn remove_keylog_app(storage: State<'_, StorageHandle>, path: String) -> Vec<String> {
    crate::keylog::restore_app_injection(&storage, &path);
    if let Err(error) = storage::keylog::remove_app(&storage, &path) {
        tracing::error!(error = %error, path, "remove_keylog_app (storage) échoué");
    }
    list_keylog_apps_impl(&storage)
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
