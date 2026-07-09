//! Process `tshark` live (story 3.3/3.4) — un seul process (plusieurs `-i`), même rigueur
//! d'arrêt que `capture/subsystem.rs` (SIGTERM puis SIGKILL, thread stderr relayé via
//! `tracing::warn!`). `TsharkBackend` abstrait détection ET exécution : les tests injectent
//! `FakeTsharkBackend`, jamais de vrai `tshark` invoqué (absent sur cette machine de dev, cf.
//! rapport de livraison) — la détection réelle (`detection::detect_tshark`) n'est appelée que
//! par `SystemTsharkBackend::detect()`, jamais depuis un test.

use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

use super::detection::{detect_tshark, TsharkAvailability};

const STOP_TIMEOUT: Duration = Duration::from_secs(2);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Flux de lignes JSON (`-T ek`) et handle d'arrêt — deux objets distincts (et non un seul,
/// contrairement à `capture/subsystem.rs` qui garde `Child` dans la struct parente) : le
/// lecteur est déplacé dans le thread de lecture, le handle reste côté `KeylogSubsystem` pour
/// que `stop()` puisse interrompre la lecture bloquante en tuant le process depuis le thread
/// principal (même effet que fermer le pipe stdout du process, cf. `capture::subsystem`).
pub struct SpawnedTshark {
    pub reader: Box<dyn TsharkReader>,
    pub handle: Box<dyn TsharkHandle>,
}

pub trait TsharkReader: Send {
    /// Bloque jusqu'à la prochaine ligne, `None` quand le flux se termine (process arrêté).
    fn next_line(&mut self) -> Option<String>;
}

pub trait TsharkHandle: Send {
    /// Arrêt coopératif — appelé une seule fois par `KeylogSubsystem::stop()` (protégé par son
    /// `Mutex`), jamais concurrent avec lui-même.
    fn stop(&mut self);
}

pub trait TsharkBackend: Send + Sync {
    fn detect(&self) -> TsharkAvailability;
    fn spawn(&self, keyfile: &Path, interfaces: &[String]) -> std::io::Result<SpawnedTshark>;
}

pub struct SystemTsharkBackend;

impl TsharkBackend for SystemTsharkBackend {
    fn detect(&self) -> TsharkAvailability {
        detect_tshark()
    }

    fn spawn(&self, keyfile: &Path, interfaces: &[String]) -> std::io::Result<SpawnedTshark> {
        let mut command = Command::new("tshark");
        for interface in interfaces {
            command.arg("-i").arg(interface);
        }
        command
            .arg("-o")
            .arg(format!("tls.keylog_file:{}", keyfile.display()))
            .arg("-T")
            .arg("ek")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| stdio_missing("stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| stdio_missing("stderr"))?;
        let stderr_reader = std::thread::spawn(move || read_stderr_stream(stderr));

        Ok(SpawnedTshark {
            reader: Box::new(RealTsharkReader {
                lines: BufReader::new(stdout).lines(),
            }),
            handle: Box::new(RealTsharkHandle {
                child,
                stderr_reader: Some(stderr_reader),
            }),
        })
    }
}

fn stdio_missing(which: &str) -> std::io::Error {
    std::io::Error::other(format!("tshark: {which} indisponible après spawn"))
}

struct RealTsharkReader {
    lines: Lines<BufReader<ChildStdout>>,
}

impl TsharkReader for RealTsharkReader {
    fn next_line(&mut self) -> Option<String> {
        match self.lines.next()? {
            Ok(line) => Some(line),
            Err(error) => {
                tracing::error!(error = %error, "lecture stdout tshark échouée");
                None
            }
        }
    }
}

struct RealTsharkHandle {
    child: Child,
    stderr_reader: Option<std::thread::JoinHandle<()>>,
}

impl TsharkHandle for RealTsharkHandle {
    fn stop(&mut self) {
        if let Err(error) = send_sigterm(&self.child) {
            tracing::warn!(error = %error, "échec d'envoi de SIGTERM à tshark");
        }
        if !wait_for_exit(&mut self.child, STOP_TIMEOUT) {
            tracing::warn!("tshark toujours actif après timeout, SIGKILL");
            if let Err(error) = self.child.kill() {
                tracing::error!(error = %error, "échec du SIGKILL sur tshark");
            }
            let _ = self.child.wait();
        }
        if let Some(handle) = self.stderr_reader.take() {
            if let Err(error) = handle.join() {
                tracing::error!(error = ?error, "thread de lecture stderr tshark paniqué");
            }
        }
    }
}

fn send_sigterm(child: &Child) -> std::io::Result<()> {
    // SAFETY: `kill(2)` sur le pid du process enfant que ce `Child` possède — même opération
    // que `capture::subsystem::send_sigterm`, aucun UB possible.
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
                tracing::warn!(error = %error, "échec de try_wait sur tshark");
                return false;
            }
        }
    }
    false
}

fn read_stderr_stream(stderr: ChildStderr) {
    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        match line {
            Ok(line) if !line.trim().is_empty() => tracing::warn!(line = %line, "tshark stderr"),
            Ok(_) => {}
            Err(error) => {
                tracing::error!(error = %error, "lecture stderr tshark échouée");
                break;
            }
        }
    }
}

/// Backend en mémoire pour les tests — jamais de vrai process `tshark` spawné.
#[cfg(test)]
pub struct FakeTsharkBackend {
    pub lines: std::sync::Mutex<Vec<String>>,
    pub availability: TsharkAvailability,
    pub fail_spawn: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl FakeTsharkBackend {
    pub fn available(lines: Vec<String>) -> Self {
        Self {
            lines: std::sync::Mutex::new(lines),
            availability: TsharkAvailability {
                installed: true,
                can_capture: true,
                interfaces: vec!["1".to_string()],
                reason: None,
            },
            fail_spawn: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn unavailable() -> Self {
        Self {
            lines: std::sync::Mutex::new(Vec::new()),
            availability: TsharkAvailability {
                installed: false,
                can_capture: false,
                interfaces: Vec::new(),
                reason: Some("tshark absent (fake)".to_string()),
            },
            fail_spawn: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl TsharkBackend for FakeTsharkBackend {
    fn detect(&self) -> TsharkAvailability {
        self.availability.clone()
    }

    fn spawn(&self, _keyfile: &Path, _interfaces: &[String]) -> std::io::Result<SpawnedTshark> {
        use std::sync::atomic::Ordering;
        if self.fail_spawn.load(Ordering::SeqCst) {
            return Err(std::io::Error::other("spawn tshark refusé (fake)"));
        }
        let lines = std::mem::take(&mut *self.lines.lock().unwrap());
        Ok(SpawnedTshark {
            reader: Box::new(FakeTsharkReader {
                lines: lines.into_iter(),
            }),
            handle: Box::new(FakeTsharkHandle),
        })
    }
}

#[cfg(test)]
struct FakeTsharkReader {
    lines: std::vec::IntoIter<String>,
}

#[cfg(test)]
impl TsharkReader for FakeTsharkReader {
    fn next_line(&mut self) -> Option<String> {
        self.lines.next()
    }
}

#[cfg(test)]
struct FakeTsharkHandle;

#[cfg(test)]
impl TsharkHandle for FakeTsharkHandle {
    fn stop(&mut self) {}
}
