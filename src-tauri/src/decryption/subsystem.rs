//! `PolarProxySubsystem` — implémente `Subsystem`, remplace le `StubSubsystem` "polarproxy"
//! (PLAN.md §6nonies). GARDE-FOU ABSOLU (4.2) : `start()` n'applique la redirection nftables
//! qu'APRÈS confirmation que PolarProxy écoute réellement ; un `AbnormalExitGuard`
//! (`abnormal_exit_guard.rs`, même pattern Drop-based que `attribution/server.rs`) retire
//! IMMÉDIATEMENT la redirection (avec retry) si le process meurt sans passer par `stop()`, et
//! remet `active` à `false`. `stop()` retire la redirection PUIS arrête le process (ordre
//! inverse strict de `start()`, jamais l'inverse).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::correlation::CorrelationSender;
use crate::killswitch::{KillSwitchError, Subsystem};
use crate::storage::decryption as storage_decryption;
use crate::storage::StorageHandle;

use super::abnormal_exit_guard::AbnormalExitGuard;
use super::helper_backend::HelperBackend;
use super::output::{spawn_output_pipeline, OutputPipeline};
use super::polarproxy_process::{PolarProxyBackend, PolarProxyConfig, PolarProxyController};
use super::vitrail_data_dir;

const LISTEN_PORT_ENV: &str = "VITRAIL_POLARPROXY_LISTEN_PORT";
const DEFAULT_LISTEN_PORT: u16 = 10443;
const DEFAULT_DECRYPTED_PORT: u16 = 10080;
const DEFAULT_PCAPOVERIP_PORT: u16 = 10444;
const CONFIRM_LISTENING_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_GRACE_PERIOD: Duration = Duration::from_secs(2);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

struct RunningPolarProxy {
    controller: Box<dyn PolarProxyController>,
    watchdog_thread: std::thread::JoinHandle<()>,
    clean_shutdown: Arc<AtomicBool>,
    #[allow(dead_code)] // conservées pour un futur join/diagnostic explicite (EPIC 8)
    output: OutputPipeline,
}

pub struct PolarProxySubsystem {
    // `Arc` (pas juste `AtomicBool`) : partagé avec l'`AbnormalExitGuard` déporté sur le thread
    // de garde (`arm_guard`), qui doit pouvoir le remettre à `false` de façon honnête quand
    // PolarProxy meurt anormalement (point 3, audit EPIC 4) sans emprunter `&PolarProxySubsystem`
    // au-delà de sa durée de vie ('static requis par `std::thread::spawn`).
    active: Arc<AtomicBool>,
    running: Mutex<Option<RunningPolarProxy>>,
    storage: StorageHandle,
    correlation: CorrelationSender,
    backend: Box<dyn PolarProxyBackend>,
    redirect: Arc<dyn HelperBackend>,
}

impl PolarProxySubsystem {
    pub fn new(
        storage: StorageHandle,
        correlation: CorrelationSender,
        redirect: Arc<dyn HelperBackend>,
    ) -> Self {
        Self::with_backend(
            Box::new(super::polarproxy_process::SystemPolarProxyBackend),
            storage,
            correlation,
            redirect,
        )
    }

    /// Constructeur pour les tests : injecte un `PolarProxyBackend` en mémoire, jamais de vrai
    /// `PolarProxy`/`tshark`/`pkexec` déclenché.
    pub fn with_backend(
        backend: Box<dyn PolarProxyBackend>,
        storage: StorageHandle,
        correlation: CorrelationSender,
        redirect: Arc<dyn HelperBackend>,
    ) -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            running: Mutex::new(None),
            storage,
            correlation,
            backend,
            redirect,
        }
    }

    fn exec_error(reason: impl ToString) -> KillSwitchError {
        KillSwitchError::SubsystemExec {
            subsystem: "polarproxy".to_string(),
            reason: reason.to_string(),
        }
    }

    fn listen_port() -> u16 {
        std::env::var(LISTEN_PORT_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_LISTEN_PORT)
    }

    fn flowlog_path() -> PathBuf {
        vitrail_data_dir().join("polarproxy-flow.log")
    }

    /// `None` signifie "état dégradé accepté" (PolarProxy absent ou CA pas encore générée) —
    /// `start()` doit rester `Ok(())` dans ce cas (même philosophie que `keylog` face à
    /// `tshark` absent), jamais faire échouer toute l'activation pour un outil externe manquant.
    fn try_start_polarproxy(&self) -> Result<Option<RunningPolarProxy>, KillSwitchError> {
        let Some(ca) = storage_decryption::get_ca(&self.storage).ok().flatten() else {
            tracing::warn!(
                "decryption: aucune CA locale connue — PolarProxy non démarré (état dégradé, \
                 l'étape CA doit précéder polarproxy dans la séquence d'activation)"
            );
            return Ok(None);
        };

        let availability = self.backend.detect();
        if !availability.installed {
            tracing::warn!(
                reason = availability.reason.as_deref().unwrap_or("raison inconnue"),
                "decryption: PolarProxy indisponible — décryptage actif désactivé (état dégradé \
                 explicite, aucune redirection nftables ne sera jamais appliquée)"
            );
            return Ok(None);
        }

        let config = PolarProxyConfig {
            ca_cert_path: PathBuf::from(&ca.cert_path),
            ca_key_path: PathBuf::from(&ca.key_path),
            listen_port: Self::listen_port(),
            decrypted_port: DEFAULT_DECRYPTED_PORT,
            pcapoverip_port: DEFAULT_PCAPOVERIP_PORT,
            flowlog_path: Self::flowlog_path(),
        };

        self.spawn_and_confirm(&config)
    }

    fn spawn_and_confirm(
        &self,
        config: &PolarProxyConfig,
    ) -> Result<Option<RunningPolarProxy>, KillSwitchError> {
        let spawned = self.backend.spawn(config).map_err(|error| {
            tracing::error!(error = %error, "decryption: démarrage de PolarProxy échoué malgré une détection positive");
            Self::exec_error(error)
        })?;

        // GARDE-FOU point 1 : confirmation réelle AVANT toute redirection — jamais un
        // lancement optimiste (PLAN.md §6nonies 4.2).
        if !spawned
            .controller
            .confirm_listening(CONFIRM_LISTENING_TIMEOUT)
        {
            spawned.controller.request_stop();
            tracing::error!(
                "decryption: PolarProxy lancé mais jamais confirmé à l'écoute — \
                 redirection nftables JAMAIS appliquée"
            );
            return Err(Self::exec_error("PolarProxy non confirmé à l'écoute"));
        }

        if let Err(error) = self.redirect.nft_redirect(config.listen_port) {
            spawned.controller.request_stop();
            return Err(Self::exec_error(error));
        }

        Ok(Some(self.arm_guard(spawned, config)))
    }

    /// Arme le garde-fou (point 2) et démarre le pipeline de sortie — appelé UNIQUEMENT après
    /// confirmation d'écoute ET application de la redirection (rien à surveiller avant ce point,
    /// même raisonnement que le guard d'attribution "créé APRÈS le signal prêt").
    fn arm_guard(
        &self,
        spawned: super::polarproxy_process::SpawnedPolarProxy,
        config: &PolarProxyConfig,
    ) -> RunningPolarProxy {
        let clean_shutdown = Arc::new(AtomicBool::new(false));
        let guard = AbnormalExitGuard {
            clean_shutdown: clean_shutdown.clone(),
            active: self.active.clone(),
            redirect: self.redirect.clone(),
        };

        let watchdog = spawned.watchdog;
        let watchdog_thread = std::thread::spawn(move || {
            let _guard = guard;
            watchdog.wait_exit();
        });

        let output = spawn_output_pipeline(
            config.pcapoverip_port,
            config.flowlog_path.clone(),
            self.correlation.clone(),
            self.storage.clone(),
        );

        RunningPolarProxy {
            controller: spawned.controller,
            watchdog_thread,
            clean_shutdown,
            output,
        }
    }
}

impl Subsystem for PolarProxySubsystem {
    fn name(&self) -> &'static str {
        "polarproxy"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        let running = self.try_start_polarproxy()?;
        *self.running.lock().expect("mutex polarproxy empoisonné") = running;
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Ordre inverse STRICT de `start()` : retire la redirection PUIS arrête le process
    /// (PLAN.md §6nonies, jamais l'inverse — sinon une fenêtre existerait où PolarProxy est
    /// mort mais le trafic y est encore redirigé).
    fn stop(&self) -> Result<(), KillSwitchError> {
        let running = {
            let mut guard = self.running.lock().expect("mutex polarproxy empoisonné");
            guard.take()
        };
        let Some(running) = running else {
            self.active.store(false, Ordering::SeqCst);
            return Ok(());
        };

        // Positionné AVANT toute action : le garde-fou ne doit jamais interpréter cet arrêt
        // volontaire comme une mort anormale.
        running.clean_shutdown.store(true, Ordering::SeqCst);

        if let Err(error) = self.redirect.nft_clear_redirect() {
            tracing::error!(error = %error, "decryption: retrait de la redirection nftables à l'arrêt échoué");
        }

        running.controller.request_stop();
        if !wait_thread_finished(&running.watchdog_thread, STOP_GRACE_PERIOD) {
            tracing::warn!(
                "decryption: PolarProxy toujours actif après le délai de grâce, SIGKILL"
            );
            running.controller.request_kill();
        }
        if let Err(error) = running.watchdog_thread.join() {
            tracing::error!(error = ?error, "decryption: thread de garde PolarProxy paniqué");
        }

        self.active.store(false, Ordering::SeqCst);
        tracing::info!("decryption: PolarProxy arrêté, redirection retirée");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

fn wait_thread_finished(thread: &std::thread::JoinHandle<()>, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if thread.is_finished() {
            return true;
        }
        std::thread::sleep(STOP_POLL_INTERVAL);
    }
    thread.is_finished()
}

/// Variante testable pour le test des 100 cycles kill switch — jamais de vrai
/// `PolarProxy`/`pkexec` déclenché.
#[cfg(test)]
pub struct FakePolarProxySubsystem {
    active: AtomicBool,
}

#[cfg(test)]
impl FakePolarProxySubsystem {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakePolarProxySubsystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Subsystem for FakePolarProxySubsystem {
    fn name(&self) -> &'static str {
        "polarproxy"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
#[path = "subsystem_tests.rs"]
mod tests;
