//! Cycle de vie du process externe PolarProxy (Netresec, non bundlé — comme `tshark` en
//! EPIC 3). `PolarProxyBackend` sépare détection ET spawn (comme `TsharkBackend`) ; le spawn
//! retourne un `PolarProxyController` (léger, `Send`, peut envoyer SIGTERM/SIGKILL par pid brut
//! SANS posséder le `Child` — appelable depuis `PolarProxySubsystem::stop()`) ET un
//! `PolarProxyWatchdog` (possède le `Child` exclusivement, bloque sur `wait()` — consommé par le
//! thread de garde ci-dessous). Cette séparation évite tout `Mutex<Child>` partagé entre le
//! thread appelant `stop()` et le thread de garde (qui bloque potentiellement plusieurs
//! secondes sur `wait()`), donc aucun risque de deadlock entre les deux.
//!
//! CLI réelle vérifiée contre le vrai binaire PolarProxy 2.0.1 (2026-07-10) : `PolarProxy -p
//! <port>,<decrypted-port>,<target> --cacert load:<pkcs12>:<motdepasse> -f <flowlog>
//! --pcapoverip <port> -v`. `--cacert load` exige un PKCS12, jamais le PEM `rcgen` brut —
//! reproduit manuellement, confirmé par `PolarProxy --help`. La conversion PEM→PKCS12
//! (`decryption::ca::export_pkcs12`, shelle vers `openssl`) a lieu DANS
//! `SystemPolarProxyBackend::spawn()`, jamais dans `subsystem.rs` : `PolarProxyConfig` ne
//! porte que les chemins PEM, sinon un test injectant `FakePolarProxyBackend` déclencherait
//! quand même un vrai `openssl` avant d'atteindre le fake (violerait "jamais de process réel
//! en test" — bug réel introduit puis corrigé le 2026-07-10, cf. STATE.md).

use std::io;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const CONNECT_POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone)]
pub struct PolarProxyAvailability {
    pub installed: bool,
    pub reason: Option<String>,
}

pub struct PolarProxyConfig {
    pub ca_cert_path: PathBuf,
    pub ca_key_path: PathBuf,
    pub listen_port: u16,
    pub decrypted_port: u16,
    pub pcapoverip_port: u16,
    pub flowlog_path: PathBuf,
}

pub struct SpawnedPolarProxy {
    pub controller: Box<dyn PolarProxyController>,
    pub watchdog: Box<dyn PolarProxyWatchdog>,
}

/// Capacités légères, `&self`, appelables depuis n'importe quel thread sans exclusion mutuelle
/// avec le watchdog (implémentation réelle : signal Unix par pid brut, aucune possession du
/// `Child`).
pub trait PolarProxyController: Send {
    /// Confirme que PolarProxy écoute RÉELLEMENT sur `listen_port` (le port MITM réel, cible
    /// de la redirection DNAT nftables) — PAS sur `pcapoverip_port` (flux diagnostic tshark,
    /// EPIC 4 audit point 1). Sonder le mauvais port confirmerait à tort un lancement où le
    /// listener MITM a échoué à binder pendant que le flux diagnostic est ouvert, ce qui
    /// appliquerait la redirection nftables vers un port mort et blackholerait tout le trafic
    /// web de la machine — exactement ce que ce garde-fou doit rendre impossible (PLAN.md
    /// §6nonies 4.2, point 1). Jamais un lancement optimiste.
    fn confirm_listening(&self, timeout: Duration) -> bool;
    fn request_stop(&self);
    fn request_kill(&self);
}

/// Possède le `Child` exclusivement — seul le thread de garde y accède, bloque sur `wait_exit`.
pub trait PolarProxyWatchdog: Send {
    fn wait_exit(self: Box<Self>);
}

pub trait PolarProxyBackend: Send + Sync {
    fn detect(&self) -> PolarProxyAvailability;
    fn spawn(&self, config: &PolarProxyConfig) -> io::Result<SpawnedPolarProxy>;
}

pub struct SystemPolarProxyBackend;

impl PolarProxyBackend for SystemPolarProxyBackend {
    /// Détection honnête (comme `tshark`) : `PolarProxy --help` best-effort — n'importe quelle
    /// sortie (stdout/stderr) est traitée comme preuve d'un binaire fonctionnel, aucune
    /// convention `--version` confirmée pour ce binaire par la recherche EPIC 4.
    fn detect(&self) -> PolarProxyAvailability {
        match Command::new("PolarProxy").arg("--help").output() {
            Ok(output) if !output.stdout.is_empty() || !output.stderr.is_empty() => {
                PolarProxyAvailability {
                    installed: true,
                    reason: None,
                }
            }
            Ok(_) => PolarProxyAvailability {
                installed: false,
                reason: Some("PolarProxy --help n'a produit aucune sortie".to_string()),
            },
            Err(error) => PolarProxyAvailability {
                installed: false,
                reason: Some(format!("PolarProxy introuvable: {error}")),
            },
        }
    }

    fn spawn(&self, config: &PolarProxyConfig) -> io::Result<SpawnedPolarProxy> {
        let (pkcs12_path, pkcs12_password) = super::ca::export_pkcs12(
            &config.ca_cert_path.to_string_lossy(),
            &config.ca_key_path.to_string_lossy(),
        )
        .map_err(io::Error::other)?;

        let child = Command::new("PolarProxy")
            .arg("-p")
            .arg(format!(
                "{},{},443",
                config.listen_port, config.decrypted_port
            ))
            .arg("--cacert")
            .arg(format!(
                "load:{}:{}",
                pkcs12_path.display(),
                pkcs12_password
            ))
            .arg("-f")
            .arg(&config.flowlog_path)
            .arg("--pcapoverip")
            .arg(config.pcapoverip_port.to_string())
            // Sans ceci, un client qui rejette le certificat forgé (pinning, ou magasin de
            // confiance propre à l'app — Electron/Node.js n'utilisent PAS le trust store
            // système où la CA Vitrail est installée) boucle indéfiniment sur le handshake
            // TLS (jusqu'à 2×`--tlstimeout` avant échec) au lieu de basculer en pass-through
            // — observé en conditions réelles (Discord + Claude Code CLI déconnectés ~1min
            // après activation, 2026-07-10). 1 échec suffit à basculer, mémorisé 5 min pour
            // éviter de retenter la voie lente à chaque reconnexion du même client.
            .arg("--bypassonfail")
            .arg("1:300")
            .arg("--tlstimeout")
            .arg("5")
            .arg("-v")
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        let pid = child.id();
        if child.stderr.is_none() {
            tracing::warn!("PolarProxy: stderr indisponible après spawn (diagnostic dégradé)");
        }

        Ok(SpawnedPolarProxy {
            controller: Box::new(RealController {
                pid,
                listen_port: config.listen_port,
            }),
            watchdog: Box::new(RealWatchdog { child }),
        })
    }
}

struct RealController {
    pid: u32,
    listen_port: u16,
}

impl PolarProxyController for RealController {
    fn confirm_listening(&self, timeout: Duration) -> bool {
        wait_until_listening(self.listen_port, timeout)
    }

    fn request_stop(&self) {
        send_signal(self.pid, libc::SIGTERM);
    }

    fn request_kill(&self) {
        send_signal(self.pid, libc::SIGKILL);
    }
}

fn send_signal(pid: u32, signal: libc::c_int) {
    // SAFETY: `kill(2)` sur le pid du process enfant que ce controller représente — même
    // opération que `capture::subsystem::send_sigterm`, aucun UB possible.
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result != 0 {
        tracing::warn!(pid, signal, error = %io::Error::last_os_error(), "échec d'envoi de signal à PolarProxy");
    }
}

/// Probe TCP répétée sur le port donné — succès = un listener y répond réellement. Appelée par
/// `RealController::confirm_listening` avec `listen_port`, jamais `pcapoverip_port` (point 1).
fn wait_until_listening(port: u16, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(CONNECT_POLL_INTERVAL);
    }
    false
}

struct RealWatchdog {
    child: Child,
}

impl PolarProxyWatchdog for RealWatchdog {
    fn wait_exit(mut self: Box<Self>) {
        if let Err(error) = self.child.wait() {
            tracing::warn!(error = %error, "PolarProxy: échec de wait() sur le process");
        }
    }
}

/// Preuve directe du fix point 1 (audit EPIC 4) : `RealController::confirm_listening` doit
/// sonder `listen_port`, jamais `pcapoverip_port`. Utilise de vrais `TcpListener` (pas le
/// backend `fake`, qui simule au niveau du scénario mais pas au niveau du port réellement
/// probé) pour prouver que la confirmation échoue si SEUL un port diagnostic est ouvert, et
/// réussit uniquement quand `listen_port` lui-même est à l'écoute.
#[cfg(test)]
mod confirm_listening_tests {
    use super::*;
    use std::net::TcpListener;

    const PROBE_TIMEOUT: Duration = Duration::from_millis(300);

    fn free_port() -> u16 {
        TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn echoue_si_seul_le_port_pcapoverip_diagnostic_est_ouvert() {
        // Le port PCAP-over-IP diagnostic est ouvert, mais `listen_port` (la vraie cible de la
        // redirection DNAT) ne l'est pas — scénario exact de l'audit : PolarProxy a bindé son
        // flux diagnostic mais a échoué à démarrer son listener MITM réel.
        let pcapoverip_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let pcapoverip_port = pcapoverip_listener.local_addr().unwrap().port();
        let never_bound_listen_port = free_port();

        let controller = RealController {
            pid: 0,
            listen_port: never_bound_listen_port,
        };

        assert!(
            !controller.confirm_listening(PROBE_TIMEOUT),
            "GARDE-FOU: confirm_listening ne doit JAMAIS réussir sur la seule base du port \
             PCAP-over-IP diagnostic ({pcapoverip_port}) — listen_port ({never_bound_listen_port}) \
             n'écoute pas"
        );
    }

    #[test]
    fn reussit_uniquement_quand_listen_port_ecoute_reellement() {
        let listen_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let listen_port = listen_listener.local_addr().unwrap().port();

        let controller = RealController {
            pid: 0,
            listen_port,
        };

        assert!(
            controller.confirm_listening(PROBE_TIMEOUT),
            "confirm_listening doit réussir quand listen_port écoute réellement"
        );
    }
}

/// Backends/handles en mémoire pour les tests — jamais de vrai `PolarProxy` invoqué (absent de
/// cette machine de dev, cf. rapport de livraison). `trigger_abnormal_death()` simule le
/// scénario le plus important de cette passe : le process meurt SANS passer par
/// `PolarProxySubsystem::stop()` pendant que la redirection est active.
#[cfg(test)]
pub mod fake {
    use super::*;
    use std::sync::{Arc, Condvar, Mutex};

    struct Shared {
        dead: Mutex<bool>,
        condvar: Condvar,
    }

    #[derive(Clone)]
    pub struct FakePolarProxyBackend {
        installed: bool,
        listening: bool,
        current: Arc<Mutex<Option<Arc<Shared>>>>,
    }

    impl FakePolarProxyBackend {
        pub fn available_and_listening() -> Self {
            Self {
                installed: true,
                listening: true,
                current: Arc::new(Mutex::new(None)),
            }
        }

        pub fn available_but_never_listens() -> Self {
            Self {
                installed: true,
                listening: false,
                current: Arc::new(Mutex::new(None)),
            }
        }

        pub fn unavailable() -> Self {
            Self {
                installed: false,
                listening: false,
                current: Arc::new(Mutex::new(None)),
            }
        }

        /// Simule la mort anormale du process actuellement "en cours" — no-op si rien n'est
        /// actif (spawn() jamais appelé, ou déjà arrêté).
        pub fn trigger_abnormal_death(&self) {
            if let Some(shared) = self.current.lock().unwrap().clone() {
                *shared.dead.lock().unwrap() = true;
                shared.condvar.notify_all();
            }
        }
    }

    impl PolarProxyBackend for FakePolarProxyBackend {
        fn detect(&self) -> PolarProxyAvailability {
            PolarProxyAvailability {
                installed: self.installed,
                reason: (!self.installed).then(|| "PolarProxy absent (fake)".to_string()),
            }
        }

        fn spawn(&self, _config: &PolarProxyConfig) -> io::Result<SpawnedPolarProxy> {
            let shared = Arc::new(Shared {
                dead: Mutex::new(false),
                condvar: Condvar::new(),
            });
            *self.current.lock().unwrap() = Some(shared.clone());

            Ok(SpawnedPolarProxy {
                controller: Box::new(FakeController {
                    shared: shared.clone(),
                    listening: self.listening,
                }),
                watchdog: Box::new(FakeWatchdog { shared }),
            })
        }
    }

    struct FakeController {
        shared: Arc<Shared>,
        listening: bool,
    }

    impl PolarProxyController for FakeController {
        fn confirm_listening(&self, _timeout: Duration) -> bool {
            self.listening
        }

        fn request_stop(&self) {
            *self.shared.dead.lock().unwrap() = true;
            self.shared.condvar.notify_all();
        }

        fn request_kill(&self) {
            self.request_stop();
        }
    }

    struct FakeWatchdog {
        shared: Arc<Shared>,
    }

    impl PolarProxyWatchdog for FakeWatchdog {
        fn wait_exit(self: Box<Self>) {
            let mut dead = self.shared.dead.lock().unwrap();
            while !*dead {
                dead = self.shared.condvar.wait(dead).unwrap();
            }
        }
    }
}
