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
