//! Persistance SQLite WAL, rétention, recherche — ne contient aucune logique métier de
//! corrélation (ARCHITECTURE.md). Frontière stricte : aucun accès direct à la connexion
//! SQLite depuis l'extérieur de ce domaine — uniquement les fonctions publiques de
//! `events`/`attribution`/`retention`/`sessions`/`flows`, consommées par `killswitch/`,
//! `capture/`, `attribution/`, `correlation/` et `commands/` (PLAN.md §6sexies/§6septies).

mod connection;
mod error;
mod migrations;

pub mod attribution;
pub mod events;
pub mod flows;
pub mod retention;
pub mod sessions;

#[cfg(test)]
mod tests;

pub use connection::StorageHandle;
// Contrat public du domaine (erreur explicite en cas de future consommation directe hors des
// wrappers `KillSwitchError`/`String` actuels des domaines appelants) — pas encore référencée
// par son nom en dehors de `storage/`, d'où l'allow ciblé plutôt qu'un export mort silencieux.
#[allow(unused_imports)]
pub use error::StorageError;
