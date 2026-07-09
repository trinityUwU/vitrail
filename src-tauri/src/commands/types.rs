//! Types partagés entre modules de commandes — sérialisation IPC vers le frontend.
//! Remplacés à terme par les types réels exportés depuis `correlation`/`storage` (EPIC 8.5).

use serde::{Deserialize, Serialize};

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
