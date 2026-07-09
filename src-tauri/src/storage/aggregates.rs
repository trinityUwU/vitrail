//! Agrégations SQL sur `flows` (EPIC 5/6, jamais branchées — raccordées PLAN.md §6decies) pour
//! les écrans Vue d'ensemble/Processus/Destinations. Pas de nouvelle table : `flows` porte déjà
//! tous les champs nécessaires (`process`, `destination`, `ip`, `size_bytes`, `visibility`,
//! `timestamp_unix`). `ip`/`visibility` par groupe sont dérivés par sous-requête corrélée
//! (respectivement "IP du flow le plus récent" et "valeur la plus fréquente") plutôt que par un
//! `MIN`/`MAX` arbitraire, qui donnerait une valeur non représentative pour une destination
//! CDN multi-IP ou un processus ayant changé de visibilité au fil du temps.

use rusqlite::{params, OptionalExtension};

use crate::shared::FlowVisibility;

use super::connection::{now_unix, StorageHandle};
use super::destinations;
use super::error::StorageError;
use super::flows::{visibility_from_str, visibility_to_str};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardAggregate {
    pub active_connections: u32,
    pub total_volume_bytes: i64,
    pub meta_only_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessAggregate {
    pub name: String,
    pub volume_bytes: i64,
    pub destination_count: u32,
    pub visibility: FlowVisibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationAggregate {
    pub domain: String,
    pub ip: String,
    pub volume_bytes: i64,
    pub process_count: u32,
    pub visibility: FlowVisibility,
    pub tls: bool,
    pub first_seen_unix: i64,
    pub last_seen_unix: i64,
    pub tag: Option<String>,
}

/// Connexions actives = flows vus dans les `active_window_secs` dernières secondes (Vue
/// d'ensemble, fenêtre proposée par PLAN.md : 5 min). `meta_only_count` reste global (pas
/// borné à la fenêtre) : c'est un compteur d'exposition cumulée, pas un indicateur temps réel.
pub fn summarize_dashboard(
    storage: &StorageHandle,
    active_window_secs: i64,
) -> Result<DashboardAggregate, StorageError> {
    let earliest = now_unix() - active_window_secs;
    let conn = storage.lock();
    conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM flows WHERE timestamp_unix >= ?1),
            (SELECT COALESCE(SUM(size_bytes), 0) FROM flows),
            (SELECT COUNT(*) FROM flows WHERE visibility = ?2)",
        params![earliest, visibility_to_str(FlowVisibility::Meta)],
        |row| {
            Ok(DashboardAggregate {
                active_connections: row.get::<_, i64>(0)? as u32,
                total_volume_bytes: row.get(1)?,
                meta_only_count: row.get::<_, i64>(2)? as u32,
            })
        },
    )
    .map_err(Into::into)
}

const PROCESS_AGGREGATE_SQL: &str = "SELECT
        process,
        SUM(size_bytes) AS volume_bytes,
        COUNT(DISTINCT destination) AS destination_count,
        (SELECT visibility FROM flows f2 WHERE f2.process = f.process
         GROUP BY visibility ORDER BY COUNT(*) DESC, visibility LIMIT 1) AS dominant_visibility
     FROM flows f
     WHERE process IS NOT NULL";

/// Par volume décroissant (Vue par processus + top 6 du dashboard).
pub fn list_processes_aggregated(
    storage: &StorageHandle,
) -> Result<Vec<ProcessAggregate>, StorageError> {
    let conn = storage.lock();
    let sql = format!("{PROCESS_AGGREGATE_SQL} GROUP BY process ORDER BY volume_bytes DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_process)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn get_process_aggregated(
    storage: &StorageHandle,
    name: &str,
) -> Result<Option<ProcessAggregate>, StorageError> {
    let conn = storage.lock();
    let sql = format!("{PROCESS_AGGREGATE_SQL} AND process = ?1 GROUP BY process");
    conn.query_row(&sql, params![name], row_to_process)
        .optional()
        .map_err(Into::into)
}

fn row_to_process(row: &rusqlite::Row) -> rusqlite::Result<ProcessAggregate> {
    let visibility_str: String = row.get(3)?;
    Ok(ProcessAggregate {
        name: row.get(0)?,
        volume_bytes: row.get(1)?,
        destination_count: row.get::<_, i64>(2)? as u32,
        visibility: visibility_from_str(&visibility_str),
    })
}

const DESTINATION_AGGREGATE_SQL: &str = "SELECT
        destination,
        SUM(size_bytes) AS volume_bytes,
        COUNT(DISTINCT process) AS process_count,
        MIN(timestamp_unix) AS first_seen_unix,
        MAX(timestamp_unix) AS last_seen_unix,
        (SELECT ip FROM flows f2 WHERE f2.destination = f.destination
         ORDER BY f2.timestamp_unix DESC LIMIT 1) AS ip,
        (SELECT visibility FROM flows f3 WHERE f3.destination = f.destination
         GROUP BY visibility ORDER BY COUNT(*) DESC, visibility LIMIT 1) AS dominant_visibility,
        EXISTS(SELECT 1 FROM flows f4 WHERE f4.destination = f.destination
               AND f4.protocol LIKE 'TLS%') AS tls
     FROM flows f
     WHERE destination IS NOT NULL";

/// Par volume décroissant (Vue par destination + top 6 du dashboard). Le tag est fusionné en
/// une seule requête batch (`get_all_tags`) pour éviter un aller-retour SQL par destination.
pub fn list_destinations_aggregated(
    storage: &StorageHandle,
) -> Result<Vec<DestinationAggregate>, StorageError> {
    let tags = destinations::get_all_tags(storage)?;
    let conn = storage.lock();
    let sql =
        format!("{DESTINATION_AGGREGATE_SQL} GROUP BY destination ORDER BY volume_bytes DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to_destination)?;
    let mut aggregates = rows.collect::<Result<Vec<_>, _>>()?;
    for aggregate in &mut aggregates {
        aggregate.tag = tags.get(&aggregate.domain).cloned();
    }
    Ok(aggregates)
}

pub fn get_destination_aggregated(
    storage: &StorageHandle,
    domain: &str,
) -> Result<Option<DestinationAggregate>, StorageError> {
    let tag = destinations::get_tag(storage, domain)?;
    let conn = storage.lock();
    let sql = format!("{DESTINATION_AGGREGATE_SQL} AND destination = ?1 GROUP BY destination");
    let mut aggregate = conn
        .query_row(&sql, params![domain], row_to_destination)
        .optional()?;
    if let Some(aggregate) = aggregate.as_mut() {
        aggregate.tag = tag;
    }
    Ok(aggregate)
}

fn row_to_destination(row: &rusqlite::Row) -> rusqlite::Result<DestinationAggregate> {
    let visibility_str: String = row.get(6)?;
    Ok(DestinationAggregate {
        domain: row.get(0)?,
        volume_bytes: row.get(1)?,
        process_count: row.get::<_, i64>(2)? as u32,
        first_seen_unix: row.get(3)?,
        last_seen_unix: row.get(4)?,
        ip: row.get(5)?,
        visibility: visibility_from_str(&visibility_str),
        tls: row.get(7)?,
        tag: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::{CorrelationSource, Flow, HttpHeader};
    use crate::storage::flows::insert_flow;

    fn flow(id: &str, process: &str, destination: &str, ip: &str, size_bytes: u64) -> Flow {
        Flow {
            id: id.into(),
            timestamp: "14:00:00".into(),
            process: process.into(),
            destination: destination.into(),
            ip: ip.into(),
            port: 443,
            protocol: "TLS 1.3".into(),
            size_bytes,
            duration_ms: 100,
            visibility: FlowVisibility::Fully,
            method: None,
            path: None,
            status: None,
            source_ip: "192.168.1.42".into(),
            source_port: 51000,
            request_headers: Vec::<HttpHeader>::new(),
            response_headers: Vec::<HttpHeader>::new(),
            body_preview: None,
            content_type: None,
            certificate: None,
            sources: Vec::<CorrelationSource>::new(),
        }
    }

    #[test]
    fn summarize_dashboard_agrege_connexions_volume_et_meta_only() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        insert_flow(&storage, &flow("f1", "Chrome", "a.com", "1.1.1.1", 100)).unwrap();
        let mut meta_flow = flow("f2", "Slack", "b.com", "2.2.2.2", 200);
        meta_flow.visibility = FlowVisibility::Meta;
        insert_flow(&storage, &meta_flow).unwrap();

        let summary = summarize_dashboard(&storage, 300).unwrap();
        assert_eq!(summary.active_connections, 2);
        assert_eq!(summary.total_volume_bytes, 300);
        assert_eq!(summary.meta_only_count, 1);
    }

    #[test]
    fn summarize_dashboard_exclut_les_flows_hors_fenetre() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        insert_flow(&storage, &flow("f1", "Chrome", "a.com", "1.1.1.1", 100)).unwrap();

        let summary = summarize_dashboard(&storage, -1).unwrap();
        assert_eq!(
            summary.active_connections, 0,
            "une fenêtre négative ne doit inclure aucun flow"
        );
    }

    #[test]
    fn list_processes_aggregated_groupe_par_processus() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        insert_flow(&storage, &flow("f1", "Chrome", "a.com", "1.1.1.1", 100)).unwrap();
        insert_flow(&storage, &flow("f2", "Chrome", "b.com", "2.2.2.2", 50)).unwrap();
        insert_flow(&storage, &flow("f3", "Firefox", "a.com", "1.1.1.1", 10)).unwrap();

        let processes = list_processes_aggregated(&storage).unwrap();
        assert_eq!(processes.len(), 2);
        assert_eq!(processes[0].name, "Chrome", "tri par volume décroissant");
        assert_eq!(processes[0].volume_bytes, 150);
        assert_eq!(processes[0].destination_count, 2);
        assert_eq!(processes[1].name, "Firefox");
    }

    #[test]
    fn get_process_aggregated_absent_renvoie_none() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        assert_eq!(get_process_aggregated(&storage, "inconnu").unwrap(), None);
    }

    #[test]
    fn list_destinations_aggregated_fusionne_le_tag_et_derive_le_tls() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        insert_flow(&storage, &flow("f1", "Chrome", "a.com", "1.1.1.1", 100)).unwrap();
        let mut plain_flow = flow("f2", "curl", "b.com", "2.2.2.2", 20);
        plain_flow.protocol = "TCP".into();
        insert_flow(&storage, &plain_flow).unwrap();
        destinations::set_tag(&storage, "a.com", "surveillé").unwrap();

        let list = list_destinations_aggregated(&storage).unwrap();
        assert_eq!(list.len(), 2);
        let a = list.iter().find(|d| d.domain == "a.com").unwrap();
        assert_eq!(a.tag, Some("surveillé".to_string()));
        assert!(a.tls);
        let b = list.iter().find(|d| d.domain == "b.com").unwrap();
        assert_eq!(b.tag, None);
        assert!(!b.tls);
    }

    #[test]
    fn get_destination_aggregated_calcule_first_et_last_seen() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        insert_flow(&storage, &flow("f1", "Chrome", "a.com", "1.1.1.1", 100)).unwrap();
        insert_flow(&storage, &flow("f2", "Chrome", "a.com", "1.1.1.1", 50)).unwrap();

        let aggregate = get_destination_aggregated(&storage, "a.com")
            .unwrap()
            .unwrap();
        assert_eq!(aggregate.volume_bytes, 150);
        assert_eq!(aggregate.process_count, 1);
        assert!(aggregate.first_seen_unix <= aggregate.last_seen_unix);
    }
}
