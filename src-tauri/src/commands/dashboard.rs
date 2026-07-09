//! Commandes IPC pour l'écran Vue d'ensemble (UI_SPEC.md #1).

use tauri::State;

use crate::killswitch::KillSwitchState;
use crate::storage::{self, StorageHandle};

use super::types::{DashboardSummary, DestinationInfo, ProcessInfo};

/// Fenêtre de connexions "actives" (PLAN.md §6decies propose 5 min).
const ACTIVE_WINDOW_SECS: i64 = 300;
const TOP_N: usize = 6;

/// PLAN.md §6decies : `top_processes`/`top_destinations`/`active_connections`/`total_in_mb`/
/// `meta_only_count` viennent de `storage::aggregates`. `kill_switch_active`/`degraded`
/// viennent de `KillSwitchState::current_status()` (déjà réel, EPIC 7) — pas de nouvelle
/// donnée à fabriquer. `active_since` reste `None` : aucun horodatage d'activation n'est
/// suivi côté `killswitch/` (seul un booléen `active`) ; `None` honnête plutôt qu'une valeur
/// inventée, cohérent avec la discipline "pas de faux sentiment" du reste du projet.
#[tauri::command]
pub fn get_dashboard_summary(
    storage: State<'_, StorageHandle>,
    killswitch: State<'_, KillSwitchState>,
) -> DashboardSummary {
    get_dashboard_summary_impl(&storage, &killswitch)
}

fn get_dashboard_summary_impl(
    storage: &StorageHandle,
    killswitch: &KillSwitchState,
) -> DashboardSummary {
    let status = killswitch.current_status();
    let aggregate = storage::aggregates::summarize_dashboard(storage, ACTIVE_WINDOW_SECS)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "summarize_dashboard (storage) échoué");
            storage::aggregates::DashboardAggregate {
                active_connections: 0,
                total_volume_bytes: 0,
                meta_only_count: 0,
            }
        });

    let top_processes: Vec<ProcessInfo> = storage::aggregates::list_processes_aggregated(storage)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "list_processes_aggregated (storage) échoué");
            Vec::new()
        })
        .into_iter()
        .take(TOP_N)
        .map(ProcessInfo::from)
        .collect();

    let top_destinations: Vec<DestinationInfo> =
        storage::aggregates::list_destinations_aggregated(storage)
            .unwrap_or_else(|error| {
                tracing::error!(error = %error, "list_destinations_aggregated (storage) échoué");
                Vec::new()
            })
            .into_iter()
            .take(TOP_N)
            .map(DestinationInfo::from)
            .collect();

    // `flows` ne porte pas de direction (in/out) par 5-tuple (schéma EPIC 5/6) : le volume
    // total observé est reporté en entrée, `total_out_mb` reste à 0 plutôt que d'inventer un
    // partage arbitraire sans donnée pour l'étayer.
    DashboardSummary {
        kill_switch_active: status.kill_switch_state == "active",
        active_since: None,
        active_connections: aggregate.active_connections,
        total_in_mb: aggregate.total_volume_bytes.max(0) as f64 / (1024.0 * 1024.0),
        total_out_mb: 0.0,
        meta_only_count: aggregate.meta_only_count,
        top_processes,
        top_destinations,
        degraded: !status.last_verification_clean,
        degraded_reason: (!status.last_verification_clean)
            .then_some("dernière vérification post-activation non conforme".to_string()),
    }
}
