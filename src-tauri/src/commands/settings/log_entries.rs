//! Journal système (UI_SPEC.md #11) — extrait de `settings.rs` pour rester sous la limite de
//! 500 lignes (code-standards.md), même raison que `commands::mock_flows` en son temps.

use tauri::State;

use crate::storage::{self, StorageHandle};

use super::super::types::LogEntry;

/// Borne du Journal système — même ordre de grandeur que ce que le mock exposait.
const LOG_ENTRIES_LIMIT: u32 = 200;

/// PLAN.md §6decies : requête réelle sur `system_events` (déjà utilisée par `purge_logs`,
/// alimentée par `killswitch::snapshot::append_event` — labels `pre-activation`/
/// `post-activation`/`post-deactivation`/`emergency-stop`, `snapshot_json` = `SystemSnapshot`
/// sérialisé). Parsé en `serde_json::Value` générique plutôt qu'en `SystemSnapshot` : ce type
/// est interne à `killswitch/` (module `snapshot` non `pub`), `commands/` n'a pas à traverser
/// cette frontière pour un simple affichage (ARCHITECTURE.md).
#[tauri::command]
pub fn get_log_entries(storage: State<'_, StorageHandle>) -> Vec<LogEntry> {
    get_log_entries_impl(&storage)
}

/// Cœur de `get_log_entries`, extrait pour être testable sans `tauri::State` (même raison que
/// `purge_data_impl` dans `settings.rs`).
fn get_log_entries_impl(storage: &StorageHandle) -> Vec<LogEntry> {
    storage::events::list_system_events(storage, LOG_ENTRIES_LIMIT)
        .unwrap_or_else(|error| {
            tracing::error!(error = %error, "get_log_entries (storage) échoué");
            Vec::new()
        })
        .into_iter()
        .map(to_log_entry)
        .collect()
}

fn to_log_entry(row: storage::events::SystemEventRow) -> LogEntry {
    let snapshot: serde_json::Value =
        serde_json::from_str(&row.snapshot_json).unwrap_or(serde_json::Value::Null);
    LogEntry {
        time: format_hms(row.timestamp_unix),
        level: log_level_for_label(&row.label),
        subsystem: "killswitch".into(),
        message: log_message_for_event(&row.label, &snapshot),
    }
}

/// `emergency-stop` est le seul chemin de sortie non-nominal de `killswitch/` — seul label à
/// mériter une visibilité supérieure à `info`.
fn log_level_for_label(label: &str) -> String {
    if label == "emergency-stop" {
        "warn".into()
    } else {
        "info".into()
    }
}

fn log_message_for_event(label: &str, snapshot: &serde_json::Value) -> String {
    let chain_present = snapshot
        .get("nftables_chain_present")
        .and_then(|v| v.as_bool());
    let active_subsystems = snapshot
        .get("subsystems")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter(|s| s.get("active").and_then(|v| v.as_bool()).unwrap_or(false))
                .count()
        });
    let (Some(chain_present), Some(active_subsystems)) = (chain_present, active_subsystems) else {
        return format!("Événement système « {label} »");
    };
    let chain_state = if chain_present {
        "présente"
    } else {
        "absente"
    };
    let label_text = match label {
        "pre-activation" => "Snapshot avant activation",
        "post-activation" => "Activation terminée",
        "post-deactivation" => "Désactivation terminée",
        "emergency-stop" => "Arrêt d'urgence déclenché",
        other => other,
    };
    format!(
        "{label_text} : chaîne nftables {chain_state}, {active_subsystems} sous-système(s) actif(s)"
    )
}

/// Même format `HH:MM:SS` que `Flow.timestamp`/`commands::types::format_hms` — dupliqué ici
/// (petite fonction de présentation, pas une règle métier) plutôt que traversé depuis
/// `types.rs`, qui n'exporte pas ce détail hors de son usage `From<...>`.
fn format_hms(timestamp_unix: i64) -> String {
    let secs = timestamp_unix.max(0) as u64;
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_log_entries_impl_mappe_label_et_snapshot_reels() {
        let storage = StorageHandle::open_in_memory().expect("ouverture en mémoire");
        let snapshot = r#"{"timestamp_unix":1000,"nftables_chain_present":true,"subsystems":[{"name":"capture","active":true},{"name":"decryption","active":false}]}"#;
        storage::events::record_system_event(&storage, "post-activation", snapshot)
            .expect("event réel");
        storage::events::record_system_event(&storage, "emergency-stop", snapshot)
            .expect("event emergency");

        let entries = get_log_entries_impl(&storage);
        assert_eq!(entries.len(), 2, "plus récent en premier");
        assert_eq!(entries[0].subsystem, "killswitch");
        assert_eq!(
            entries[0].level, "warn",
            "emergency-stop doit rester visible"
        );
        assert!(entries[0].message.contains("Arrêt d'urgence déclenché"));
        assert!(entries[0].message.contains("1 sous-système"));

        assert_eq!(entries[1].level, "info");
        assert!(entries[1].message.contains("Activation terminée"));
        assert!(entries[1].message.contains("présente"));
    }

    #[test]
    fn get_log_entries_impl_reste_honnete_sur_un_snapshot_illisible() {
        let storage = StorageHandle::open_in_memory().expect("ouverture en mémoire");
        storage::events::record_system_event(&storage, "pre-activation", "{}")
            .expect("event legacy/test");

        let entries = get_log_entries_impl(&storage);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].message, "Événement système « pre-activation »",
            "un snapshot sans les champs attendus ne doit jamais faire planter le mapping"
        );
    }
}
