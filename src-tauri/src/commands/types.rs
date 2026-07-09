//! Types partagés entre modules de commandes — sérialisation IPC vers le frontend.
//! Remplacés à terme par les types réels exportés depuis `correlation`/`storage` (EPIC 8.5).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FlowVisibility {
    Fully,
    Meta,
    Attrib,
    Unknown,
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub id: String,
    pub timestamp: String,
    pub process: String,
    pub destination: String,
    pub ip: String,
    pub port: u16,
    pub protocol: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub visibility: FlowVisibility,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
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
pub struct SubsystemStatus {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub kill_switch_state: String,
    pub subsystems: Vec<SubsystemStatus>,
    pub last_verification: Option<String>,
    pub last_verification_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeardownReport {
    pub clean: bool,
    pub divergences: Vec<String>,
    pub checked_at: String,
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
