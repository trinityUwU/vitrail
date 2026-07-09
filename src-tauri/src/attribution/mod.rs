//! Consommer les événements OpenSnitch (pid ↔ connexion) — EPIC 1.
//!
//! Correction d'architecture actée en PLAN.md §6quinquies / EPICS.md : `opensnitchd` est le
//! **client** gRPC `ui.proto`, `attribution/` implémente le **serveur**. `AttributionSubsystem`
//! implémente `Subsystem` (killswitch/subsystem.rs) : `start()` lance le serveur gRPC
//! (server.rs) sur un socket UNIX dédié Vitrail, puis reconfigure `opensnitchd`
//! (daemon_config.rs) pour qu'il s'y connecte ; `stop()` restaure la config d'origine du
//! daemon puis arrête le serveur gRPC. Ne capture pas de paquets, ne décide pas de blocage
//! (ARCHITECTURE.md) : chaque requête `AskRule` du daemon reçoit une réponse "allow" non
//! persistante — Vitrail observe, jamais n'arbitre.

mod cache;
mod daemon_config;
mod desktop_resolver;
mod pb;
mod server;
mod subsystem;

#[cfg(test)]
mod tests;

#[allow(unused_imports)] // contrat public du domaine, consommé par EPIC 5 (corrélation)
pub use server::{AttributionEvent, FiveTuple};
pub use subsystem::AttributionSubsystem;

#[cfg(test)]
pub use subsystem::FakeAttributionSubsystem;
