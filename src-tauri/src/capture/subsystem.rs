//! `CaptureSubsystem` — implémente `Subsystem` (killswitch/subsystem.rs) en remplacement du
//! `StubSubsystem` "capture" (PLAN.md §6quater) : spawn/pilote le binaire privilégié
//! `vitrail-capture-helper`, lit son stdout JSON Lines en continu, persiste chaque paquet
//! via `events::append_packet`. `stop()` envoie `SIGTERM` puis `SIGKILL` en dernier recours.
//!
//! EPIC 5 : chaque paquet retenu est AUSSI publié vers `correlation/` via `CorrelationSender`
//! (en plus de la persistance `storage::capture_events` existante, jamais à la place) —
//! `send_capture` est non-bloquant (`try_send`), un canal plein ne fait jamais échouer ni
//! ralentir la lecture du flux stdout du helper (PLAN.md §6septies).

use std::io::{BufRead, BufReader};
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::correlation::CorrelationSender;
use crate::killswitch::{KillSwitchError, Subsystem};
use crate::storage::StorageHandle;

use super::events::{append_packet, CapturedPacket};

const DEFAULT_HELPER_PATH: &str = "/usr/local/bin/vitrail-capture-helper";
const STOP_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

struct RunningProcess {
    child: Child,
    reader: JoinHandle<()>,
    stderr_reader: JoinHandle<()>,
}

pub struct CaptureSubsystem {
    active: AtomicBool,
    running: Mutex<Option<RunningProcess>>,
    storage: StorageHandle,
    correlation: CorrelationSender,
}

impl CaptureSubsystem {
    pub fn new(storage: StorageHandle, correlation: CorrelationSender) -> Self {
        Self {
            active: AtomicBool::new(false),
            running: Mutex::new(None),
            storage,
            correlation,
        }
    }

    fn helper_path() -> String {
        std::env::var("VITRAIL_CAPTURE_HELPER_PATH")
            .unwrap_or_else(|_| DEFAULT_HELPER_PATH.to_string())
    }

    fn exec_error(reason: impl ToString) -> KillSwitchError {
        KillSwitchError::SubsystemExec {
            subsystem: "capture".to_string(),
            reason: reason.to_string(),
        }
    }
}

// Pas d'impl `Default` : `new()` exige désormais un `StorageHandle` explicite (EPIC 6), un
// `Default` masquerait implicitement quelle connexion storage est utilisée.

impl Subsystem for CaptureSubsystem {
    fn name(&self) -> &'static str {
        "capture"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        let helper = Self::helper_path();
        let mut child = Command::new(&helper)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                tracing::error!(
                    error = %error,
                    helper = %helper,
                    "échec de démarrage de vitrail-capture-helper \
                     (binaire absent, setcap manquant, ou permission refusée)"
                );
                Self::exec_error(error)
            })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            tracing::error!("vitrail-capture-helper: stdout indisponible après spawn");
            Self::exec_error("stdout indisponible")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            tracing::error!("vitrail-capture-helper: stderr indisponible après spawn");
            Self::exec_error("stderr indisponible")
        })?;

        let storage = self.storage.clone();
        let correlation = self.correlation.clone();
        let reader =
            std::thread::spawn(move || read_capture_stream(&storage, &correlation, stdout));
        let stderr_reader = std::thread::spawn(move || read_stderr_stream(stderr));

        let mut running = self.running.lock().expect("mutex capture empoisonné");
        *running = Some(RunningProcess {
            child,
            reader,
            stderr_reader,
        });
        drop(running);

        self.active.store(true, Ordering::SeqCst);
        tracing::info!(helper = %helper, "vitrail-capture-helper démarré");
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        let running = {
            let mut guard = self.running.lock().expect("mutex capture empoisonné");
            guard.take()
        };

        let Some(process) = running else {
            self.active.store(false, Ordering::SeqCst);
            return Ok(());
        };

        terminate_process(process);
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("vitrail-capture-helper arrêté");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

/// SIGTERM coopératif, timeout court, SIGKILL en dernier recours ; joint ensuite le thread de
/// lecture stdout (son `lines()` se termine naturellement à la fermeture du pipe). Prend
/// `RunningProcess` par valeur : `JoinHandle::join` consomme le handle, et `process` a déjà
/// été extrait du `Mutex` par l'appelant, donc aucun autre code ne peut y accéder entretemps.
fn terminate_process(mut process: RunningProcess) {
    if let Err(error) = send_sigterm(&process.child) {
        tracing::warn!(error = %error, "échec d'envoi de SIGTERM à vitrail-capture-helper");
    }

    if !wait_for_exit(&mut process.child, STOP_TIMEOUT) {
        tracing::warn!("vitrail-capture-helper toujours actif après timeout, SIGKILL");
        if let Err(error) = process.child.kill() {
            tracing::error!(error = %error, "échec du SIGKILL sur vitrail-capture-helper");
        }
        let _ = process.child.wait();
    }

    if let Err(error) = process.reader.join() {
        tracing::error!(error = ?error, "thread de lecture stdout capture paniqué");
    }
    if let Err(error) = process.stderr_reader.join() {
        tracing::error!(error = ?error, "thread de lecture stderr capture paniqué");
    }
}

fn send_sigterm(child: &Child) -> std::io::Result<()> {
    // SAFETY: `kill(2)` sur le pid du process enfant que ce `Child` possède — opération
    // standard d'arrêt coopératif, pas de mémoire partagée, aucun UB possible.
    let result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_status)) => return true,
            Ok(None) => std::thread::sleep(STOP_POLL_INTERVAL),
            Err(error) => {
                tracing::warn!(error = %error, "échec de try_wait sur vitrail-capture-helper");
                return false;
            }
        }
    }
    false
}

/// Lit le stdout du helper ligne par ligne, parse chaque enregistrement JSON, persiste, ET
/// publie vers `correlation/` (EPIC 5 — en plus de la persistance, jamais à la place). Une
/// ligne invalide est loggée et ignorée (jamais fatale) ; une erreur de lecture stoppe le
/// thread (le pipe est de toute façon fermé si le process est mort).
fn read_capture_stream(
    storage: &StorageHandle,
    correlation: &CorrelationSender,
    stdout: ChildStdout,
) {
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                tracing::error!(error = %error, "lecture stdout vitrail-capture-helper échouée");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<CapturedPacket>(&line) {
            Ok(packet) => {
                if let Err(error) = append_packet(storage, &packet) {
                    tracing::error!(error = %error, "persistance d'un paquet capturé échouée");
                }
                correlation.send_capture(packet);
            }
            Err(error) => {
                tracing::warn!(error = %error, line = %line, "ligne JSON de capture invalide, ignorée");
            }
        }
    }
}

/// Lit le stderr du helper ligne par ligne et relaie chaque ligne via `tracing` — symétrique
/// à `read_capture_stream`. C'est la seule voie de diagnostic pour les `eprintln!` du helper
/// (avertissement périodique de drop du rate-limiter, erreurs de canal AF_PACKET) : sans ce
/// thread ils étaient jetés vers `/dev/null` et jamais relayés à l'app parente.
fn read_stderr_stream(stderr: ChildStderr) {
    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                tracing::error!(error = %error, "lecture stderr vitrail-capture-helper échouée");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        tracing::warn!(line = %line, "vitrail-capture-helper stderr");
    }
}

/// Variante testable — jamais de vrai process spawné (même principe que
/// `FakeNftablesBackend` dans `killswitch/nftables.rs`). Consommée uniquement par le test
/// des 100 cycles (`killswitch::tests`).
#[cfg(test)]
pub struct FakeCaptureSubsystem {
    active: AtomicBool,
}

#[cfg(test)]
impl FakeCaptureSubsystem {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakeCaptureSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Subsystem for FakeCaptureSubsystem {
    fn name(&self) -> &'static str {
        "capture"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        self.active.store(true, Ordering::SeqCst);
        tracing::info!("capture (fake): démarré");
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("capture (fake): arrêté");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}
