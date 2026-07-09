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
    dashboard, destinations, flows, killswitch as ks_commands, mock_data, processes, settings,
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
            let flow = mock_data::mock_live_flow(seq);
            if let Err(error) = handle.emit("vitrail://flow", &flow) {
                eprintln!("échec d'émission de l'événement flow factice: {error}");
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            settings::list_alert_rules,
            settings::toggle_alert_rule,
            settings::list_sessions,
            settings::get_log_entries,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
