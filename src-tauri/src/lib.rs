//! Point d'entrée Tauri — monte les commandes IPC et l'émetteur d'événements de dev.
//! Domaines réels (attribution, capture, decryption, keylog, correlation, storage,
//! killswitch) sont stubés (EPIC 0) et implémentés dans les EPICs 1 à 7.

mod attribution;
mod capture;
mod commands;
mod correlation;
mod decryption;
mod keylog;
mod killswitch;
mod shared;
mod storage;

use commands::{
    alerts, dashboard, destinations, flows, killswitch as ks_commands, mock_flows, processes,
    search, settings,
};
use tauri::Emitter;

/// Intervalle de l'émetteur factice temporaire (EPIC 8.4) — remplacé par le streaming réel
/// de `correlation` quand EPIC 5.4 sera implémenté.
const MOCK_LIVE_FLOW_INTERVAL_SECS: u64 = 4;

fn spawn_mock_live_flow_emitter(app: &tauri::App) {
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        let mut seq: u64 = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(MOCK_LIVE_FLOW_INTERVAL_SECS));
            seq += 1;
            let flow = mock_flows::mock_live_flow(seq);
            if let Err(error) = handle.emit("vitrail://flow", &flow) {
                eprintln!("échec d'émission de l'événement flow factice: {error}");
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(killswitch::KillSwitchState::new())
        .setup(|app| {
            spawn_mock_live_flow_emitter(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            dashboard::get_dashboard_summary,
            flows::list_flows,
            flows::get_flow_detail,
            flows::search_flows,
            processes::list_processes,
            processes::get_process_detail,
            destinations::list_destinations,
            destinations::get_destination_detail,
            destinations::tag_destination,
            ks_commands::activate_vitrail,
            ks_commands::deactivate_vitrail,
            ks_commands::emergency_stop,
            ks_commands::get_system_status,
            ks_commands::verify_teardown,
            settings::get_settings,
            settings::update_settings,
            settings::add_exclusion,
            settings::remove_exclusion,
            settings::rotate_ca,
            settings::export_config,
            settings::import_config,
            settings::list_sessions,
            settings::get_session_detail,
            settings::delete_session,
            settings::get_log_entries,
            settings::purge_logs,
            settings::purge_data,
            settings::list_keylog_apps,
            settings::add_keylog_app,
            settings::remove_keylog_app,
            alerts::list_alert_rules,
            alerts::toggle_alert_rule,
            alerts::create_alert_rule,
            alerts::update_alert_rule,
            alerts::delete_alert_rule,
            alerts::list_alert_events,
            search::save_search_query,
            search::list_saved_queries,
            search::delete_saved_query,
            search::convert_query_to_alert,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
