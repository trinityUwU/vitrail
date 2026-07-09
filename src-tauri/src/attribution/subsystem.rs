//! `AttributionSubsystem` — implémente `Subsystem` (killswitch/subsystem.rs), remplace le
//! `StubSubsystem` "attribution" (PLAN.md §6quinquies). `start()` : lance le serveur gRPC
//! (server.rs) PUIS reconfigure `opensnitchd` pour qu'il pointe vers le socket Vitrail.
//! `stop()` : restaure la config d'origine du daemon PUIS arrête le serveur gRPC — un échec de
//! restauration n'est jamais silencieux, il remonte en erreur (divergence visible en 7.4).
//! `is_active()` reflète l'état du serveur gRPC.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::killswitch::{KillSwitchError, Subsystem};

use super::cache::ProcessCache;
use super::daemon_config::{
    read_last_original_address, save_original_address, DaemonConfigurator, DaemonDetection,
    SystemDaemonConfigurator,
};
use super::server::{self, ServerHandle};

pub struct AttributionSubsystem {
    active: AtomicBool,
    handle: Mutex<Option<ServerHandle>>,
    /// `Arc` (et non `Box`) : une référence clonée doit pouvoir migrer dans la closure
    /// `on_abnormal_exit` du thread serveur (`server::start`, `'static`) pour le filet de
    /// sécurité de restauration (robustesse, audit EPIC 1) — un `Box` emprunté par `&self` ne
    /// peut pas survivre au-delà de l'appel à `start()`.
    configurator: Arc<dyn DaemonConfigurator>,
    cache: Arc<ProcessCache>,
    /// Dernière détection (story 1.1) — pas encore de commande IPC dédiée (hors scope EPIC 1,
    /// réservé à EPIC 8), mais accessible pour un futur `get_system_status` enrichi plutôt
    /// que de jeter l'information après le seul log `tracing::warn!` de `detect()`.
    last_detection: Mutex<Option<DaemonDetection>>,
}

impl AttributionSubsystem {
    pub fn new() -> Self {
        Self::with_configurator(Box::new(SystemDaemonConfigurator))
    }

    /// Constructeur pour les tests : injecte un `DaemonConfigurator` en mémoire, jamais de
    /// `pkexec`/`systemctl` réel déclenché. Le serveur gRPC lui-même reste réel (socket UNIX
    /// local, story 1.5) — seule la reconfiguration système est mockée.
    pub fn with_configurator(configurator: Box<dyn DaemonConfigurator>) -> Self {
        Self {
            active: AtomicBool::new(false),
            handle: Mutex::new(None),
            configurator: Arc::from(configurator),
            cache: Arc::new(ProcessCache::new()),
            last_detection: Mutex::new(None),
        }
    }

    /// Dernière détection connue (story 1.1) — `installed`/`active`/`points_elsewhere`,
    /// consultable par un futur appelant (EPIC 8) sans redéclencher de détection.
    #[cfg(test)]
    pub fn last_detection(&self) -> Option<DaemonDetection> {
        self.last_detection
            .lock()
            .expect("mutex détection attribution empoisonné")
            .clone()
    }

    fn exec_error(reason: impl ToString) -> KillSwitchError {
        KillSwitchError::SubsystemExec {
            subsystem: "attribution".to_string(),
            reason: reason.to_string(),
        }
    }
}

/// Filet de sécurité de dernier recours : appelé par `server::AbnormalExitGuard` si le thread
/// serveur gRPC meurt SANS passer par `stop()` (panique ou erreur fatale non prévue) — sans ce
/// rattrapage, `opensnitchd` resterait configuré indéfiniment vers un socket mort. Ne doit
/// JAMAIS se déclencher en fonctionnement normal : le `tracing::error!` en tête est volontaire,
/// c'est un signal d'alerte à investiguer si jamais il apparaît en log.
fn restore_on_abnormal_exit(configurator: &dyn DaemonConfigurator) {
    tracing::error!(
        "serveur gRPC attribution terminé anormalement (hors stop()) — restauration de secours \
         de la configuration opensnitchd déclenchée"
    );
    match read_last_original_address() {
        Some(original) => {
            if let Err(error) = configurator.set_socket(&original) {
                tracing::error!(
                    error = %error,
                    original = %original,
                    "restauration de secours opensnitchd échouée après mort anormale du serveur gRPC"
                );
            }
        }
        None => tracing::warn!(
            "restauration de secours ignorée : aucune adresse d'origine opensnitchd connue"
        ),
    }
}

impl Default for AttributionSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for AttributionSubsystem {
    fn name(&self) -> &'static str {
        "attribution"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        let socket_path = server::socket_path();
        let target = server::socket_uri(&socket_path);

        let configurator_for_guard = self.configurator.clone();
        let on_abnormal_exit: Box<dyn Fn() + Send + 'static> =
            Box::new(move || restore_on_abnormal_exit(configurator_for_guard.as_ref()));

        let handle = server::start(socket_path.clone(), self.cache.clone(), on_abnormal_exit)
            .map_err(Self::exec_error)?;

        let detection = self.configurator.detect(&target);
        if !detection.installed {
            tracing::warn!(
                "opensnitchd non installé — attribution restera vide (état dégradé, story 1.1)"
            );
        }

        if let Some(original) = &detection.current_address {
            if let Err(error) = save_original_address(original) {
                tracing::error!(error = %error, "sauvegarde de l'adresse d'origine opensnitchd échouée");
            }
        }

        *self
            .last_detection
            .lock()
            .expect("mutex détection attribution empoisonné") = Some(detection.clone());

        if let Err(error) = self.configurator.set_socket(&target) {
            tracing::error!(error = %error, "reconfiguration d'opensnitchd échouée, arrêt du serveur gRPC");
            handle.shutdown();
            return Err(error);
        }

        *self.handle.lock().expect("mutex attribution empoisonné") = Some(handle);
        self.active.store(true, Ordering::SeqCst);
        tracing::info!(socket = %socket_path.display(), "attribution: serveur gRPC démarré et opensnitchd reconfiguré");
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        let restore_result = match read_last_original_address() {
            Some(original) => self.configurator.set_socket(&original).map_err(|error| {
                tracing::error!(
                    error = %error,
                    original = %original,
                    "restauration de la configuration opensnitchd échouée — divergence à signaler (7.4)"
                );
                error
            }),
            None => {
                tracing::warn!("aucune adresse d'origine opensnitchd connue, restauration ignorée");
                Ok(())
            }
        };

        let handle = self
            .handle
            .lock()
            .expect("mutex attribution empoisonné")
            .take();
        if let Some(handle) = handle {
            handle.shutdown();
        }
        self.cache.evict_dead();
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("attribution: serveur gRPC arrêté");

        restore_result
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

/// Variante testable pour le test des 100 cycles kill switch (`killswitch::tests`) — même
/// principe que `capture::FakeCaptureSubsystem` : jamais de vrai serveur gRPC/socket/pkexec.
#[cfg(test)]
pub struct FakeAttributionSubsystem {
    active: AtomicBool,
}

#[cfg(test)]
impl FakeAttributionSubsystem {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakeAttributionSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Subsystem for FakeAttributionSubsystem {
    fn name(&self) -> &'static str {
        "attribution"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        self.active.store(true, Ordering::SeqCst);
        tracing::info!("attribution (fake): démarré");
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("attribution (fake): arrêté");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering as AtomicOrdering;
    use std::sync::{Arc, Mutex as StdMutex};

    use super::*;
    use crate::attribution::daemon_config::FakeDaemonConfigurator;

    // `XDG_DATA_HOME`/`XDG_RUNTIME_DIR` sont globales au process : verrou nécessaire pour que
    // les deux tests ci-dessous (parallélisés par défaut par `cargo test`) ne se marchent pas
    // dessus (même précaution que `desktop_resolver::tests::ENV_GUARD`).
    static ENV_GUARD: StdMutex<()> = StdMutex::new(());

    // `Box<dyn DaemonConfigurator>` exige la possession — passer un `Arc` clonable permet de
    // garder une référence côté test pour inspecter `set_calls` après `start()`/`stop()`.
    impl DaemonConfigurator for Arc<FakeDaemonConfigurator> {
        fn detect(&self, vitrail_socket: &str) -> DaemonDetection {
            (**self).detect(vitrail_socket)
        }
        fn set_socket(&self, socket_address: &str) -> Result<(), KillSwitchError> {
            (**self).set_socket(socket_address)
        }
    }

    fn isolated_env(tag: &str) -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("vitrail-attr-subsys-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        std::env::set_var("XDG_DATA_HOME", base.join("data"));
        std::env::set_var("XDG_RUNTIME_DIR", base.join("runtime"));
        base
    }

    fn cleanup(base: &std::path::Path) {
        let _ = std::fs::remove_dir_all(base);
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn start_sauvegarde_origine_reconfigure_et_expose_la_detection() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("start");

        let configurator = Arc::new(FakeDaemonConfigurator::new());
        let subsystem = AttributionSubsystem::with_configurator(Box::new(configurator.clone()));

        subsystem.start().expect("start() aurait dû réussir");
        assert!(subsystem.is_active());

        {
            let calls = configurator.set_calls.lock().unwrap();
            assert_eq!(
                calls.len(),
                1,
                "set_socket doit être appelé une fois au démarrage"
            );
            assert!(
                calls[0].starts_with("unix://"),
                "l'adresse cible doit être le socket Vitrail"
            );
        }

        let detection = subsystem.last_detection().expect("détection non conservée");
        assert!(
            detection.points_elsewhere,
            "la fake config pointe ailleurs par défaut (story 1.1)"
        );

        subsystem.stop().expect("stop() aurait dû réussir");
        assert!(!subsystem.is_active());

        let calls = configurator.set_calls.lock().unwrap();
        assert_eq!(
            calls.len(),
            2,
            "set_socket doit aussi être appelé à la restauration (1.6)"
        );
        assert_eq!(
            calls[1], "unix:///tmp/osui.sock",
            "doit restaurer l'adresse d'origine exacte"
        );

        cleanup(&base);
    }

    #[test]
    fn stop_remonte_une_erreur_si_la_restauration_echoue_mais_arrete_quand_meme() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("restore-fail");

        let configurator = Arc::new(FakeDaemonConfigurator::new());
        let subsystem = AttributionSubsystem::with_configurator(Box::new(configurator.clone()));

        subsystem.start().expect("start() aurait dû réussir");
        configurator.fail_set.store(true, AtomicOrdering::SeqCst);

        let result = subsystem.stop();
        assert!(
            result.is_err(),
            "un échec de restauration doit remonter une erreur, jamais un no-op silencieux"
        );
        assert!(
            !subsystem.is_active(),
            "le serveur gRPC doit être arrêté même si la restauration échoue (best-effort)"
        );

        cleanup(&base);
    }
}
