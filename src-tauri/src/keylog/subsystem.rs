//! `KeylogSubsystem` — implémente `Subsystem` (killswitch/subsystem.rs), remplace le
//! `StubSubsystem` "keylog" (PLAN.md §6octies). `start()` : tronque le fichier de clés, injecte
//! les apps ciblées (best-effort par app), démarre `tshark` si disponible (état dégradé
//! explicite sinon — jamais un échec de toute l'activation pour cette seule raison, même
//! philosophie que `attribution::AttributionSubsystem` face à un `opensnitchd` absent).
//! `stop()` : arrête `tshark`, restaure chaque `.desktop` overridé à son état exact d'origine,
//! ne tronque JAMAIS le fichier de clés (seulement au prochain `start()`, story 3.1).

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;

use crate::correlation::CorrelationSender;
use crate::killswitch::{KillSwitchError, Subsystem};
use crate::storage::{self, StorageHandle};

use super::app_injection::{self, InjectionOutcome};
use super::keyfile::truncate_keyfile;
use super::parser::parse_ek_line;
use super::tshark_process::{SpawnedTshark, SystemTsharkBackend, TsharkBackend, TsharkHandle};

struct RunningTshark {
    handle: Box<dyn TsharkHandle>,
    reader: JoinHandle<()>,
}

pub struct KeylogSubsystem {
    active: AtomicBool,
    running: Mutex<Option<RunningTshark>>,
    storage: StorageHandle,
    correlation: CorrelationSender,
    backend: Box<dyn TsharkBackend>,
}

impl KeylogSubsystem {
    pub fn new(storage: StorageHandle, correlation: CorrelationSender) -> Self {
        Self::with_backend(Box::new(SystemTsharkBackend), storage, correlation)
    }

    /// Constructeur pour les tests : injecte un `TsharkBackend` en mémoire, jamais de vrai
    /// `tshark` déclenché.
    pub fn with_backend(
        backend: Box<dyn TsharkBackend>,
        storage: StorageHandle,
        correlation: CorrelationSender,
    ) -> Self {
        Self {
            active: AtomicBool::new(false),
            running: Mutex::new(None),
            storage,
            correlation,
            backend,
        }
    }

    fn exec_error(reason: impl ToString) -> KillSwitchError {
        KillSwitchError::SubsystemExec {
            subsystem: "keylog".to_string(),
            reason: reason.to_string(),
        }
    }

    fn inject_apps(&self, wrapper: &Path) {
        let apps = match storage::keylog::list_apps(&self.storage) {
            Ok(apps) => apps,
            Err(error) => {
                tracing::error!(error = %error, "keylog: lecture des apps ciblées échouée");
                return;
            }
        };
        for app in apps {
            match app_injection::inject_app(&self.storage, &app, wrapper) {
                InjectionOutcome::Injected { desktop_path, .. } => tracing::info!(
                    binary = %app.binary_path, desktop = %desktop_path.display(), "keylog: app injectée"
                ),
                InjectionOutcome::NoDesktopFile => tracing::warn!(
                    binary = %app.binary_path, "keylog: aucun .desktop résolvable, app non couverte"
                ),
                InjectionOutcome::AlreadyInjected => {}
            }
        }
    }

    fn restore_apps(&self) {
        let apps = match storage::keylog::list_apps(&self.storage) {
            Ok(apps) => apps,
            Err(error) => {
                tracing::error!(error = %error, "keylog: lecture des apps ciblées échouée (restauration)");
                return;
            }
        };
        for app in apps {
            app_injection::restore_app(&self.storage, &app);
        }
    }

    fn start_tshark(&self, keyfile: &Path) -> Result<(), KillSwitchError> {
        let availability = self.backend.detect();
        if !availability.can_capture {
            tracing::warn!(
                installed = availability.installed,
                reason = availability.reason.as_deref().unwrap_or("raison inconnue"),
                "keylog: tshark indisponible, aucun contenu déchiffré ne sera produit \
                 (état dégradé explicite — les apps injectées écrivent quand même leurs clés)"
            );
            return Ok(());
        }

        let spawned = self
            .backend
            .spawn(keyfile, &availability.interfaces)
            .map_err(|error| {
                tracing::error!(error = %error, "keylog: démarrage de tshark échoué malgré une détection positive");
                Self::exec_error(error)
            })?;

        let SpawnedTshark { mut reader, handle } = spawned;
        let correlation = self.correlation.clone();
        let reader_thread = std::thread::spawn(move || {
            while let Some(line) = reader.next_line() {
                if let Some(fragment) = parse_ek_line(&line) {
                    correlation.send_decryption(fragment);
                }
            }
        });

        *self.running.lock().expect("mutex keylog empoisonné") = Some(RunningTshark {
            handle,
            reader: reader_thread,
        });
        tracing::info!(interfaces = ?availability.interfaces, "keylog: tshark démarré");
        Ok(())
    }
}

impl Subsystem for KeylogSubsystem {
    fn name(&self) -> &'static str {
        "keylog"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        let keyfile = truncate_keyfile().map_err(|error| {
            tracing::error!(error = %error, "keylog: initialisation du fichier de clés échouée");
            Self::exec_error(error)
        })?;

        let wrapper = app_injection::ensure_wrapper(&keyfile).map_err(|error| {
            tracing::error!(error = %error, "keylog: écriture du wrapper de lancement échouée");
            Self::exec_error(error)
        })?;
        self.inject_apps(&wrapper);

        self.start_tshark(&keyfile)?;
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        let running = {
            let mut guard = self.running.lock().expect("mutex keylog empoisonné");
            guard.take()
        };
        if let Some(mut running) = running {
            running.handle.stop();
            if let Err(error) = running.reader.join() {
                tracing::error!(error = ?error, "thread de lecture tshark paniqué");
            }
        }

        self.restore_apps();
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("keylog: arrêté");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

/// Variante testable pour le test des 100 cycles kill switch (`killswitch::tests`) — même
/// principe que `capture::FakeCaptureSubsystem`/`attribution::FakeAttributionSubsystem` :
/// jamais de vrai `tshark`/injection réelle déclenchée.
#[cfg(test)]
pub struct FakeKeylogSubsystem {
    active: AtomicBool,
}

#[cfg(test)]
impl FakeKeylogSubsystem {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakeKeylogSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Subsystem for FakeKeylogSubsystem {
    fn name(&self) -> &'static str {
        "keylog"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        self.active.store(true, Ordering::SeqCst);
        tracing::info!("keylog (fake): démarré");
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("keylog (fake): arrêté");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering as AtomicOrdering;
    use std::time::Duration;

    use super::*;
    use crate::correlation::{self, CorrelationEvent};
    use crate::keylog::tshark_process::FakeTsharkBackend;
    use crate::keylog::DecryptedFragment;

    use crate::shared::ENV_GUARD;

    fn isolated_env(tag: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "vitrail-keylog-subsys-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("XDG_DATA_HOME", base.join("data"));
        std::env::set_var("XDG_DATA_DIRS", base.join("system"));
        base
    }

    fn cleanup(base: &std::path::Path) {
        let _ = std::fs::remove_dir_all(base);
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("XDG_DATA_DIRS");
    }

    fn ek_line_for(protocol: &str) -> String {
        format!(
            r#"{{"layers":{{"ip_ip_src":"10.0.0.5","ip_ip_dst":"93.184.216.34","{protocol}_{protocol}_srcport":"51000","{protocol}_{protocol}_dstport":"443","http_http_host":"example.com"}}}}"#
        )
    }

    #[test]
    fn start_sans_tshark_disponible_reste_actif_en_degrade() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("degraded");

        let storage = StorageHandle::open_in_memory().unwrap();
        let subsystem = KeylogSubsystem::with_backend(
            Box::new(FakeTsharkBackend::unavailable()),
            storage,
            correlation::channel().0,
        );

        subsystem
            .start()
            .expect("start() doit réussir même sans tshark (dégradé)");
        assert!(subsystem.is_active());
        subsystem.stop().expect("stop() doit réussir");
        assert!(!subsystem.is_active());

        cleanup(&base);
    }

    #[test]
    fn start_avec_tshark_publie_un_fragment_decrypte_vers_correlation() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("publish");

        let storage = StorageHandle::open_in_memory().unwrap();
        let (sender, receiver) = correlation::channel();
        let backend = FakeTsharkBackend::available(vec![ek_line_for("tcp")]);
        let subsystem = KeylogSubsystem::with_backend(Box::new(backend), storage, sender);

        subsystem.start().expect("start() doit réussir");
        assert!(subsystem.is_active());

        let event = receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("un événement de corrélation doit être publié");
        match event {
            CorrelationEvent::Decryption(fragment) => {
                let f: DecryptedFragment = fragment;
                assert_eq!(f.host.as_deref(), Some("example.com"));
            }
            _ => panic!("un CorrelationEvent::Decryption était attendu"),
        }

        subsystem.stop().expect("stop() doit réussir");
        assert!(!subsystem.is_active());

        cleanup(&base);
    }

    #[test]
    fn start_avec_echec_de_spawn_malgre_detection_positive_remonte_une_erreur() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("spawn-fail");

        let storage = StorageHandle::open_in_memory().unwrap();
        let backend = FakeTsharkBackend::available(vec![]);
        backend.fail_spawn.store(true, AtomicOrdering::SeqCst);
        let subsystem =
            KeylogSubsystem::with_backend(Box::new(backend), storage, correlation::channel().0);

        let result = subsystem.start();
        assert!(
            result.is_err(),
            "un échec de spawn malgré détection positive doit remonter"
        );
        assert!(!subsystem.is_active());

        cleanup(&base);
    }

    #[test]
    fn stop_sans_start_prealable_ne_panique_pas() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("stop-only");
        let storage = StorageHandle::open_in_memory().unwrap();
        let subsystem = KeylogSubsystem::with_backend(
            Box::new(FakeTsharkBackend::unavailable()),
            storage,
            correlation::channel().0,
        );

        subsystem
            .stop()
            .expect("stop() sans start() préalable doit rester un no-op propre");
        assert!(!subsystem.is_active());

        cleanup(&base);
    }
}
