//! Persistance des paquets capturés via `storage::events::record_capture_packet` (table
//! `capture_events`, EPIC 6 — remplace `capture_events.jsonl` posé en EPIC 2, PLAN.md
//! §6quater/§6sexies, même comportement observable).

use serde::{Deserialize, Serialize};

use crate::killswitch::KillSwitchError;
use crate::storage::{self, events::CapturePacketRecord, StorageHandle};

/// Miroir du JSON Lines émis par `vitrail-capture-helper` (stories 2.2/2.3/2.4) — mêmes noms
/// de champs en `snake_case`, aucun `rename` : le helper sérialise sans renommage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPacket {
    pub timestamp_unix_ms: u128,
    pub interface: String,
    pub protocol: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub bytes: usize,
    pub sni: Option<String>,
    pub detected_protocol: Option<String>,
}

/// Persiste un paquet capturé via `storage::`. Erreur loggée et remontée, jamais de panic :
/// une défaillance de persistance ne doit pas interrompre le thread de lecture stdout du
/// helper.
pub fn append_packet(
    storage: &StorageHandle,
    packet: &CapturedPacket,
) -> Result<(), KillSwitchError> {
    let record = CapturePacketRecord {
        timestamp_unix_ms: packet.timestamp_unix_ms as i64,
        interface: &packet.interface,
        protocol: &packet.protocol,
        src_ip: &packet.src_ip,
        dst_ip: &packet.dst_ip,
        src_port: packet.src_port,
        dst_port: packet.dst_port,
        bytes: packet.bytes as i64,
        sni: packet.sni.as_deref(),
        detected_protocol: packet.detected_protocol.as_deref(),
    };

    storage::events::record_capture_packet(storage, record).map_err(|error| {
        tracing::error!(error = %error, "persistance d'un paquet capturé (storage) échouée");
        KillSwitchError::Persistence(error.to_string())
    })
}
