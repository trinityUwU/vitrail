//! Trait `Subsystem` — cycle de vie start/stop/is_active pour chaque domaine orchestré par
//! le kill switch (capture, attribution, decryption, keylog, CA, PolarProxy).

use std::sync::atomic::{AtomicBool, Ordering};

use super::KillSwitchError;

pub trait Subsystem: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(&self) -> Result<(), KillSwitchError>;
    fn stop(&self) -> Result<(), KillSwitchError>;
    fn is_active(&self) -> bool;
}

/// Implémentation stub pour un sous-système pas encore implémenté : flip d'un booléen
/// atomique + log `tracing`, aucune action système réelle. Permet à la séquence 7.2/7.3 de
/// tourner de bout en bout dès maintenant (PLAN.md §6ter).
///
/// Décision non explicitement tranchée dans PLAN.md : la CA locale et PolarProxy sont
/// modélisés comme des `StubSubsystem` au même titre que capture/attribution/keylog plutôt
/// que comme des étapes ad hoc dans `sequence.rs` — uniformise le traitement succès/échec de
/// chaque étape (même `Result`, même log) et évite un cas spécial dans l'orchestrateur.
pub struct StubSubsystem {
    name: &'static str,
    active: AtomicBool,
}

impl StubSubsystem {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            active: AtomicBool::new(false),
        }
    }
}

impl Subsystem for StubSubsystem {
    fn name(&self) -> &'static str {
        self.name
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        self.active.store(true, Ordering::SeqCst);
        tracing::info!(subsystem = self.name, "sous-système démarré (stub)");
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        tracing::info!(subsystem = self.name, "sous-système arrêté (stub)");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}
