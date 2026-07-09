//! Fusionner les 4 sources en une timeline unique — ne capture ni ne déchiffre rien lui-même
//! (ARCHITECTURE.md). EPIC 5 (PLAN.md §6septies) : seules `capture/` et `attribution/`
//! existent réellement à ce stade (`decryption/`/`keylog/` arrivent en EPIC 3/4) — la fusion
//! fonctionne déjà avec ces deux sources, sans réécriture prévue quand les deux autres
//! arriveront (`visibility::determine_visibility` accepte déjà leurs paramètres).

mod builder;
mod channel;
mod engine;
mod update;
mod visibility;

pub use channel::{channel, CorrelationEvent, CorrelationSender};
pub use engine::spawn;
// Contrat public du domaine pas encore consommé ailleurs que dans ses propres tests
// (`EngineHandle` : type de retour opaque de `spawn`, ignoré par `lib.rs` — utile le jour où
// un arrêt propre du moteur sera nécessaire ; `CORRELATION_WINDOW`/`determine_visibility` :
// valeur/logique qu'un futur `commands/settings.rs` ou test externe pourrait vouloir
// inspecter) — allow ciblé plutôt qu'un export mort silencieux, même principe que
// `commands/types.rs::SubsystemStatus`.
#[allow(unused_imports)]
pub use engine::{EngineHandle, CORRELATION_WINDOW};
#[allow(unused_imports)]
pub use visibility::determine_visibility;

/// Type nommé pour le récepteur du canal capture/attribution → correlation — évite à `lib.rs`
/// de connaître le type concret `std::sync::mpsc::Receiver` (détail d'implémentation de
/// `channel.rs`).
pub type CorrelationEventReceiver = std::sync::mpsc::Receiver<CorrelationEvent>;
