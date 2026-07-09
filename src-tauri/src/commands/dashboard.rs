//! Commandes IPC pour l'écran Vue d'ensemble (UI_SPEC.md #1).

use super::mock_data;
use super::mock_flows;
use super::types::DashboardSummary;

/// EPIC 5/6 remplaceront ce mock par correlation::summarize() + storage::query_summary().
#[tauri::command]
pub fn get_dashboard_summary() -> DashboardSummary {
    let processes = mock_data::processes();
    let destinations = mock_data::destinations();
    let flows = mock_flows::flows();
    let meta_only_count = flows
        .iter()
        .filter(|f| matches!(f.visibility, super::types::FlowVisibility::Meta))
        .count() as u32;

    DashboardSummary {
        kill_switch_active: true,
        active_since: Some("14:02:11".into()),
        active_connections: 23,
        total_in_mb: 847.3,
        total_out_mb: 312.8,
        meta_only_count,
        top_processes: processes.into_iter().take(6).collect(),
        top_destinations: destinations.into_iter().take(6).collect(),
        degraded: false,
        degraded_reason: None,
    }
}
