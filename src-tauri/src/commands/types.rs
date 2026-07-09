//! Types partagés entre modules de commandes — sérialisation IPC vers le frontend.
//! Remplacés à terme par les types réels exportés depuis `correlation`/`storage` (EPIC 8.5).

use serde::{Deserialize, Serialize};

use crate::storage::aggregates::{DestinationAggregate, ProcessAggregate};

/// `SystemStatus`/`SubsystemStatus`/`TeardownReport` sont possédés par `crate::shared`
/// (produits par `killswitch/`, EPIC 7) — ré-exportés ici tels quels pour ne pas casser le
/// contrat IPC existant (`#[tauri::command]` dans `commands/killswitch.rs`). `SubsystemStatus`
/// n'est jamais nommé directement via ce chemin (seulement imbriqué dans `SystemStatus`), d'où
/// l'allow ciblé plutôt qu'un faux import mort.
///
/// `Flow`/`FlowVisibility`/`HttpHeader`/`CertificateInfo`/`CorrelationSource` suivent le même
/// principe depuis EPIC 5 : possédés par `crate::shared` (produits par `correlation/`, aussi
/// consommés par `storage/`), ré-exportés ici tels quels.
#[allow(unused_imports)]
pub use crate::shared::{
    CertificateInfo, CorrelationSource, Flow, FlowVisibility, HttpHeader, SubsystemStatus,
    SystemStatus, TeardownReport,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub name: String,
    pub path: String,
    pub pids: Vec<u32>,
    pub volume_mb: f64,
    pub destinations: u32,
    pub visibility: FlowVisibility,
    pub keylog_covered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationInfo {
    pub domain: String,
    pub ip: String,
    pub volume_mb: f64,
    pub process_count: u32,
    pub visibility: FlowVisibility,
    pub tls: bool,
    pub pinning: bool,
    pub first_seen: String,
    pub last_seen: String,
    pub tag: Option<String>,
}

/// `path`/`pids`/`keylog_covered` n'ont pas de source dans `flows` (PLAN.md §6decies exclut
/// explicitement toute nouvelle table pour cette passe) : défauts honnêtes plutôt qu'une
/// valeur inventée — un futur EPIC pid-keyed (colonne déjà réservée dans `attribution_state`,
/// migration 0001) les alimentera.
impl From<ProcessAggregate> for ProcessInfo {
    fn from(aggregate: ProcessAggregate) -> Self {
        ProcessInfo {
            name: aggregate.name,
            path: String::new(),
            pids: Vec::new(),
            volume_mb: bytes_to_mb(aggregate.volume_bytes),
            destinations: aggregate.destination_count,
            visibility: aggregate.visibility,
            keylog_covered: false,
        }
    }
}

/// `pinning` n'a pas de source dans cette agrégation (`pinning_events` est un domaine distinct,
/// non joint ici — hors périmètre PLAN.md §6decies) : défaut honnête `false`.
impl From<DestinationAggregate> for DestinationInfo {
    fn from(aggregate: DestinationAggregate) -> Self {
        DestinationInfo {
            domain: aggregate.domain,
            ip: aggregate.ip,
            volume_mb: bytes_to_mb(aggregate.volume_bytes),
            process_count: aggregate.process_count,
            visibility: aggregate.visibility,
            tls: aggregate.tls,
            pinning: false,
            first_seen: format_hms(aggregate.first_seen_unix),
            last_seen: format_hms(aggregate.last_seen_unix),
            tag: aggregate.tag,
        }
    }
}

fn bytes_to_mb(bytes: i64) -> f64 {
    bytes.max(0) as f64 / (1024.0 * 1024.0)
}

/// Même format d'affichage `HH:MM:SS` que `Flow.timestamp` (storage::flows::timestamp_display,
/// non exportée hors de `storage/` — frontière stricte, ARCHITECTURE.md) : recalculé ici plutôt
/// que traversé depuis `storage`, qui ne fait pas de présentation.
fn format_hms(timestamp_unix: i64) -> String {
    let secs = timestamp_unix.max(0) as u64;
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub kill_switch_active: bool,
    pub active_since: Option<String>,
    pub active_connections: u32,
    pub total_in_mb: f64,
    pub total_out_mb: f64,
    pub meta_only_count: u32,
    pub top_processes: Vec<ProcessInfo>,
    pub top_destinations: Vec<DestinationInfo>,
    pub degraded: bool,
    pub degraded_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exclusion {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub criteria: String,
    pub active: bool,
    pub trigger_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub started_at: String,
    pub ended_at: String,
    pub volume_mb: f64,
    pub process_count: u32,
    pub alert_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub subsystem: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub ca_fingerprint: String,
    pub ca_trust_store_installed: bool,
    pub nftables_chain: String,
    pub monitored_interfaces: Vec<String>,
    pub retention_days: Option<u32>,
    pub database_size_mb: f64,
    pub notifications_enabled: bool,
    pub notification_sound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertEvent {
    pub id: String,
    pub rule_id: String,
    pub flow_id: String,
    pub time: String,
    pub summary: String,
    pub visibility: FlowVisibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCriteria {
    pub process: Option<String>,
    pub destination: Option<String>,
    pub port: Option<String>,
    pub visibility: Option<FlowVisibility>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQuery {
    pub id: String,
    pub name: String,
    pub criteria: SearchCriteria,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeResult {
    pub deleted_flows: u64,
    pub freed_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session: Session,
    pub flows: Vec<Flow>,
}
