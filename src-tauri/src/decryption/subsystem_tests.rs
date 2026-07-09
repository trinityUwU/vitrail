//! Tests de `subsystem.rs`, extraits dans un fichier séparé (audit EPIC 4, point 6 — limite de
//! taille de fichier). Inclus via `#[path = "subsystem_tests.rs"] mod tests;` dans `subsystem.rs`
//! — accès aux items privés du module parent via `use super::*;`, comme un `mod tests` inline.

use super::*;
use crate::correlation;
use crate::decryption::helper_backend::FakeHelperBackend;
use crate::decryption::polarproxy_process::fake::FakePolarProxyBackend;
use crate::shared::ENV_GUARD;
use crate::storage::decryption::CaMetadata;

fn isolated_env(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!(
        "vitrail-polarproxy-subsys-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::env::set_var("XDG_DATA_HOME", &base);
    base
}

fn cleanup(base: &PathBuf) {
    let _ = std::fs::remove_dir_all(base);
    std::env::remove_var("XDG_DATA_HOME");
}

fn storage_with_ca() -> StorageHandle {
    let storage = StorageHandle::open_in_memory().unwrap();
    storage_decryption::save_ca(
        &storage,
        &CaMetadata {
            cert_path: "/tmp/vitrail-test-ca.pem".to_string(),
            key_path: "/tmp/vitrail-test-ca.key".to_string(),
            fingerprint_sha256: "a".repeat(64),
        },
    )
    .unwrap();
    storage
}

#[test]
fn start_sans_ca_connue_reste_actif_en_degrade_sans_jamais_rediriger() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("no-ca");
    let storage = StorageHandle::open_in_memory().unwrap();
    let helper = FakeHelperBackend::new();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(FakePolarProxyBackend::available_and_listening()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    subsystem
        .start()
        .expect("start() doit réussir même sans CA (dégradé)");
    assert!(subsystem.is_active());
    assert_eq!(
        helper.redirect_calls().len(),
        0,
        "aucune redirection sans CA"
    );

    subsystem.stop().unwrap();
    cleanup(&base);
}

#[test]
fn start_sans_polarproxy_installe_reste_actif_en_degrade_sans_jamais_rediriger() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("not-installed");
    let storage = storage_with_ca();
    let helper = FakeHelperBackend::new();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(FakePolarProxyBackend::unavailable()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    subsystem
        .start()
        .expect("start() doit réussir même sans PolarProxy installé (dégradé)");
    assert!(subsystem.is_active());
    assert_eq!(
        helper.redirect_calls().len(),
        0,
        "aucune redirection possible sans PolarProxy installé"
    );

    subsystem.stop().unwrap();
    cleanup(&base);
}

#[test]
fn start_sans_confirmation_d_ecoute_ne_redirige_jamais() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("never-listens");
    let storage = storage_with_ca();
    let helper = FakeHelperBackend::new();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(FakePolarProxyBackend::available_but_never_listens()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    let result = subsystem.start();
    assert!(
        result.is_err(),
        "un échec de confirmation d'écoute doit remonter une erreur"
    );
    assert_eq!(
        helper.redirect_calls().len(),
        0,
        "GARDE-FOU: jamais de redirection sans confirmation d'écoute réelle"
    );

    cleanup(&base);
}

#[test]
fn start_confirme_applique_la_redirection_puis_stop_la_retire_dans_le_bon_ordre() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("happy-path");
    let storage = storage_with_ca();
    let helper = FakeHelperBackend::new();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(FakePolarProxyBackend::available_and_listening()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    subsystem.start().expect("start() doit réussir");
    assert_eq!(helper.redirect_calls(), vec![DEFAULT_LISTEN_PORT]);
    assert_eq!(helper.clear_redirect_calls(), 0);

    subsystem.stop().expect("stop() doit réussir");
    assert_eq!(
        helper.clear_redirect_calls(),
        1,
        "stop() doit retirer la redirection appliquée par start()"
    );
    assert!(!subsystem.is_active());

    cleanup(&base);
}

/// LE TEST LE PLUS IMPORTANT DE CETTE PASSE (PLAN.md §6nonies 4.2/4.6) : simule la mort
/// anormale de PolarProxy PENDANT que la redirection est active (sans passer par `stop()`)
/// et vérifie que le garde-fou retire bien la redirection nftables via le backend fake, ET
/// (point 3, audit EPIC 4) que `is_active()` redevient honnêtement `false` sans `stop()` manuel.
#[test]
fn garde_fou_retire_la_redirection_si_polarproxy_meurt_anormalement() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("abnormal-death");
    let storage = storage_with_ca();
    let helper = FakeHelperBackend::new();
    let backend = FakePolarProxyBackend::available_and_listening();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(backend.clone()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    subsystem.start().expect("start() doit réussir");
    assert_eq!(
        helper.redirect_calls().len(),
        1,
        "la redirection doit être active avant la simulation de crash"
    );
    assert_eq!(helper.clear_redirect_calls(), 0);
    assert!(subsystem.is_active());

    // Simule le crash : le process meurt SANS que `subsystem.stop()` n'ait été appelé.
    backend.trigger_abnormal_death();

    // Le thread de garde tourne en tâche de fond : attend (best-effort borné) que le
    // Drop du guard ait eu le temps de s'exécuter et d'appeler le backend fake.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while helper.clear_redirect_calls() == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(
        helper.clear_redirect_calls(),
        1,
        "GARDE-FOU: la redirection nftables doit être retirée immédiatement quand \
         PolarProxy meurt anormalement, sans attendre un stop() explicite"
    );
    assert!(
        !subsystem.is_active(),
        "point 3 (audit EPIC 4): is_active() ne doit plus mentir après une mort anormale, \
         même sans stop() manuel"
    );

    cleanup(&base);
}

/// Point 2 (audit EPIC 4) : un échec TRANSITOIRE de `nft_clear_redirect` depuis le garde-fou
/// (ex: `pkexec`/`vitrail-helper` momentanément indisponible) doit être retenté, pas abandonné
/// après une seule tentative.
#[test]
fn garde_fou_reessaie_nft_clear_redirect_apres_un_echec_transitoire() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("guard-retry-transient");
    let storage = storage_with_ca();
    let helper = FakeHelperBackend::new();
    let backend = FakePolarProxyBackend::available_and_listening();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(backend.clone()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    subsystem.start().expect("start() doit réussir");
    // Échoue les 2 premières tentatives, réussit à la 3e (dans la limite de
    // ABNORMAL_EXIT_CLEAR_MAX_ATTEMPTS = 3).
    helper.fail_clear_redirect_times(2);

    backend.trigger_abnormal_death();

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while helper.clear_redirect_calls() < 3 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(
        helper.clear_redirect_calls(),
        3,
        "le garde-fou doit retenter nft_clear_redirect jusqu'à réussir (2 échecs + 1 succès)"
    );
    assert!(
        !subsystem.is_active(),
        "is_active() doit refléter l'arrêt même après des tentatives retentées"
    );

    cleanup(&base);
}

/// Point 2/3 (audit EPIC 4) : si TOUTES les tentatives de `nft_clear_redirect` échouent
/// (`pkexec` totalement indisponible), le garde-fou doit épuiser ses tentatives bornées puis,
/// même dans cet échec définitif, remettre `active` à `false` — jamais un `is_active() == true`
/// qui mentirait sur l'état réel pendant que le trafic reste potentiellement bloqué.
#[test]
fn garde_fou_marque_inactif_meme_si_toutes_les_tentatives_echouent() {
    let _guard = ENV_GUARD.lock().unwrap();
    let base = isolated_env("guard-retry-permanent");
    let storage = storage_with_ca();
    let helper = FakeHelperBackend::new();
    let backend = FakePolarProxyBackend::available_and_listening();
    let subsystem = PolarProxySubsystem::with_backend(
        Box::new(backend.clone()),
        storage,
        correlation::channel().0,
        Arc::new(helper.clone()),
    );

    subsystem.start().expect("start() doit réussir");
    helper.fail_clear_redirect_times(usize::MAX);

    backend.trigger_abnormal_death();

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while helper.clear_redirect_calls() < 3 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }

    assert_eq!(
        helper.clear_redirect_calls(),
        3,
        "le garde-fou doit épuiser exactement ABNORMAL_EXIT_CLEAR_MAX_ATTEMPTS tentatives"
    );
    // Laisse le temps au Drop de terminer son dernier store() après l'échec définitif.
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while subsystem.is_active() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        !subsystem.is_active(),
        "GARDE-FOU (point 3): is_active() doit devenir false même si le retrait de la \
         redirection a définitivement échoué — jamais un état qui ment"
    );

    cleanup(&base);
}
