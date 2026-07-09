//! Test 7.6 — 100 cycles activate/deactivate, doit détecter toute fuite d'état.
//! `FakeNftablesBackend` uniquement : jamais de `pkexec` ni d'accès au vrai système nftables.

use crate::attribution::FakeAttributionSubsystem;
use crate::capture::FakeCaptureSubsystem;
use crate::keylog::FakeKeylogSubsystem;
use crate::storage::StorageHandle;

use super::nftables::FakeNftablesBackend;
use super::KillSwitchState;

#[test]
fn cent_cycles_activation_desactivation_sans_fuite() {
    let state = KillSwitchState::with_backend(
        Box::new(FakeNftablesBackend::new()),
        Box::new(FakeCaptureSubsystem::new()),
        Box::new(FakeAttributionSubsystem::new()),
        Box::new(FakeKeylogSubsystem::new()),
        StorageHandle::open_in_memory().expect("ouverture storage en mémoire pour le test"),
    );

    for cycle in 0..100 {
        let activated = state.activate();
        assert_eq!(
            activated.kill_switch_state, "active",
            "cycle {cycle}: activation dégradée"
        );
        assert!(
            activated.subsystems.iter().all(|s| s.status == "ok"),
            "cycle {cycle}: un sous-système n'a pas démarré: {:?}",
            activated.subsystems
        );

        let deactivated = state.deactivate();
        assert_eq!(
            deactivated.kill_switch_state, "inactive",
            "cycle {cycle}: désactivation dégradée"
        );
        assert!(
            deactivated.subsystems.iter().all(|s| s.status == "off"),
            "cycle {cycle}: un sous-système est resté actif après deactivate(): {:?}",
            deactivated.subsystems
        );

        let report = state.verify_teardown();
        assert!(
            report.clean,
            "cycle {cycle}: divergences détectées: {:?}",
            report.divergences
        );
    }
}
