//! Capture réseau brute (AF_PACKET), 5-tuple, SNI en clair — ne déchiffre jamais de contenu
//! TLS (ARCHITECTURE.md). EPIC 2 : `CaptureSubsystem` spawn/pilote le binaire privilégié
//! `vitrail-capture-helper` (`cap_net_raw,cap_net_admin`, PLAN.md §6quater) et persiste ses
//! enregistrements JSON Lines dans `capture_events.jsonl`, transitoire avant EPIC 6/SQLite.

mod events;
mod subsystem;

pub use subsystem::CaptureSubsystem;

#[cfg(test)]
pub use subsystem::FakeCaptureSubsystem;

/// Contrat public du domaine, consommé par EPIC 5 (corrélation) — même principe que
/// `attribution::{AttributionEvent, FiveTuple}` (`attribution/mod.rs`).
pub use events::CapturedPacket;
