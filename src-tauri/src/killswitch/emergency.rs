//! Bouton d'urgence (7.5) : flush nftables direct + arrêt best-effort de chaque
//! sous-système, sans passer par la séquence ordonnée de `sequence.rs`. Priorité à la
//! restauration réseau même si le nettoyage est incomplet — retourne un rapport des étapes
//! non confirmées plutôt que d'échouer globalement.

use super::nftables::NftablesBackend;
use super::subsystem::Subsystem;

pub struct EmergencyReport {
    pub unconfirmed_steps: Vec<String>,
}

pub fn emergency_stop(
    nftables: &dyn NftablesBackend,
    subsystems: &[&dyn Subsystem],
) -> EmergencyReport {
    let mut unconfirmed_steps = Vec::new();

    if let Err(error) = nftables.flush() {
        tracing::error!(error = %error, "emergency_stop: flush nftables échoué");
        unconfirmed_steps.push("nftables".to_string());
    }

    for subsystem in subsystems {
        if let Err(error) = subsystem.stop() {
            tracing::error!(subsystem = subsystem.name(), error = %error, "emergency_stop: arrêt échoué");
            unconfirmed_steps.push(subsystem.name().to_string());
        }
    }

    EmergencyReport { unconfirmed_steps }
}
