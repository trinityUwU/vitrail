//! Trait `Subsystem` — cycle de vie start/stop/is_active pour chaque domaine orchestré par
//! le kill switch (capture, attribution, decryption/CA, decryption/PolarProxy, keylog).
//!
//! Les 6 slots sont désormais tous de vraies implémentations (EPIC 4 a remplacé les deux
//! derniers stubs, `"ca"` et `"polarproxy"`, par `decryption::CaSubsystem`/
//! `decryption::PolarProxySubsystem`) — le `StubSubsystem` générique posé en EPIC 7 pour
//! amorcer la séquence 7.2/7.3 avant que les domaines réels n'existent n'a plus de raison
//! d'être et a été retiré (aucun appelant restant).

use super::KillSwitchError;

pub trait Subsystem: Send + Sync {
    fn name(&self) -> &'static str;
    fn start(&self) -> Result<(), KillSwitchError>;
    fn stop(&self) -> Result<(), KillSwitchError>;
    fn is_active(&self) -> bool;
}
