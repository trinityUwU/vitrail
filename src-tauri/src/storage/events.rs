//! API publique storage pour `killswitch/` et `capture/` — `record_system_event` remplace
//! `killswitch::snapshot::append_event` (JSONL), `record_capture_packet` remplace
//! `capture::events::append_packet` (JSONL). Aucun type des domaines appelants importé ici :
//! le caller sérialise/déstructure lui-même (frontière stricte, storage ignore leurs structs).

use rusqlite::params;

use super::connection::{now_unix, StorageHandle};
use super::error::StorageError;

/// Un enregistrement de paquet capturé — mêmes champs que `capture::events::CapturedPacket`,
/// dupliqués ici pour ne pas faire dépendre `storage/` du domaine `capture/`.
pub struct CapturePacketRecord<'a> {
    pub timestamp_unix_ms: i64,
    pub interface: &'a str,
    pub protocol: &'a str,
    pub src_ip: &'a str,
    pub dst_ip: &'a str,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub bytes: i64,
    pub sni: Option<&'a str>,
    pub detected_protocol: Option<&'a str>,
}

/// Insère un événement système horodaté (label + snapshot déjà sérialisé en JSON par
/// l'appelant — `killswitch::snapshot`).
pub fn record_system_event(
    storage: &StorageHandle,
    label: &str,
    snapshot_json: &str,
) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT INTO system_events (timestamp_unix, label, snapshot_json) VALUES (?1, ?2, ?3)",
        params![now_unix(), label, snapshot_json],
    )?;
    Ok(())
}

/// Une ligne `system_events` brute — `commands::settings::get_log_entries` (PLAN.md §6decies)
/// dérive `LogEntry` à partir de `label`/`snapshot_json`, jamais ici : `storage/` ignore le
/// schéma métier du JSON qu'il stocke (même principe que `record_system_event`).
pub struct SystemEventRow {
    pub timestamp_unix: i64,
    pub label: String,
    pub snapshot_json: String,
}

/// Les plus récents en premier, bornés à `limit` (Journal système, #11).
pub fn list_system_events(
    storage: &StorageHandle,
    limit: u32,
) -> Result<Vec<SystemEventRow>, StorageError> {
    let conn = storage.lock();
    // `id DESC` en clé secondaire : `timestamp_unix` a une résolution à la seconde et
    // pre-activation/post-activation (killswitch::activate) peuvent tomber dans la même
    // seconde — sans elle, SQLite ne garantit pas l'ordre chronologique réel entre les deux.
    let mut stmt = conn.prepare(
        "SELECT timestamp_unix, label, snapshot_json FROM system_events
         ORDER BY timestamp_unix DESC, id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(SystemEventRow {
            timestamp_unix: row.get(0)?,
            label: row.get(1)?,
            snapshot_json: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Insère un paquet capturé (remplace `capture_events.jsonl`).
pub fn record_capture_packet(
    storage: &StorageHandle,
    record: CapturePacketRecord<'_>,
) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT INTO capture_events
            (timestamp_unix_ms, interface, protocol, src_ip, dst_ip, src_port, dst_port,
             bytes, sni, detected_protocol)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            record.timestamp_unix_ms,
            record.interface,
            record.protocol,
            record.src_ip,
            record.dst_ip,
            record.src_port,
            record.dst_port,
            record.bytes,
            record.sni,
            record.detected_protocol,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::connection::StorageHandle;
    use super::*;

    #[test]
    fn list_system_events_renvoie_les_plus_recents_en_premier_borne_a_limit() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        record_system_event(&storage, "pre-activation", "{}").expect("event 1");
        record_system_event(&storage, "post-activation", "{}").expect("event 2");
        record_system_event(&storage, "post-deactivation", "{}").expect("event 3");

        let events = list_system_events(&storage, 2).expect("list_system_events");
        assert_eq!(events.len(), 2, "borné à limit");
        assert_eq!(
            events[0].label, "post-deactivation",
            "le plus récent d'abord"
        );
        assert_eq!(events[1].label, "post-activation");
    }
}
