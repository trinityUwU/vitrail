//! Point d'entrée Tauri — monte les commandes IPC et démarre le moteur de corrélation.
//! Tous les domaines sont réels : attribution (EPIC 1), capture (EPIC 2), keylog (EPIC 3),
//! decryption/PolarProxy (EPIC 4), corrélation (EPIC 5), storage (EPIC 6), killswitch (EPIC 7).

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
    alerts, dashboard, destinations, flows, killswitch as ks_commands, processes, search, settings,
};
use tauri::Emitter;

/// Démarre le moteur de corrélation (EPIC 5.4) — consomme le récepteur créé avant la
/// construction de `tauri::Builder` (le canal doit déjà exister pour être cloné dans
/// `KillSwitchState::new`, mais le thread du moteur n'a besoin d'un `AppHandle` pour émettre
/// `vitrail://flow` qu'une fois `.setup()` atteint). `receiver` est pris dans un `Option`
/// capturé par la closure `FnOnce` de `.setup()` : ne peut être appelé qu'une fois, ce qui est
/// exactement la sémantique voulue (un seul moteur pour la durée de vie de l'app).
fn start_correlation_engine(
    app: &tauri::App,
    receiver: correlation::CorrelationEventReceiver,
    storage: storage::StorageHandle,
) {
    let handle = app.handle().clone();
    correlation::spawn(receiver, storage, move |flow| {
        if let Err(error) = handle.emit("vitrail://flow", flow) {
            tracing::error!(error = %error, "échec d'émission de l'événement vitrail://flow");
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    // Canal capture/attribution → correlation (EPIC 5) : créé AVANT `KillSwitchState::new()`
    // pour que l'émetteur (`CorrelationSender`) puisse être cloné dans `CaptureSubsystem`/
    // `AttributionSubsystem` dès leur construction — le récepteur n'est consommé par le
    // moteur qu'au `.setup()`, une fois l'`AppHandle` Tauri disponible pour émettre
    // `vitrail://flow`.
    let (correlation_sender, correlation_receiver) = correlation::channel();

    // Connexion storage ouverte une seule fois ici (migrations exécutées avant tout le reste,
    // PLAN.md §6sexies) : `KillSwitchState::new()` l'ouvre en interne puis l'expose via
    // `storage_handle()` pour que `commands/settings.rs`/`commands/flows.rs` la partagent
    // (même `Arc<Mutex<...>>`, jamais une deuxième connexion vers le même fichier).
    let killswitch_state = killswitch::KillSwitchState::new(correlation_sender);
    let storage_handle = killswitch_state.storage_handle();

    let mut correlation_receiver = Some(correlation_receiver);
    let storage_for_correlation = storage_handle.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(killswitch_state)
        .manage(storage_handle)
        .setup(move |app| {
            let receiver = correlation_receiver
                .take()
                .expect("le récepteur de corrélation ne doit être pris qu'une seule fois");
            start_correlation_engine(app, receiver, storage_for_correlation.clone());
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
            settings::log_entries::get_log_entries,
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
