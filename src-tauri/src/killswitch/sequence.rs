//! Séquences d'activation (7.2) et de désactivation (7.3) orchestrées.
//!
//! Ordre strict décidé en PLAN.md §6ter : CA → nftables → PolarProxy → attribution →
//! capture → keylog. `mod.rs` construit le `Vec<Step>` dans cet ordre exact ; ce module ne
//! réordonne jamais rien lui-même, il se contente d'itérer (avant pour l'activation, en
//! ordre inverse pour la désactivation).

use std::sync::mpsc;
use std::time::Duration;

use super::nftables::NftablesBackend;
use super::subsystem::Subsystem;
use super::KillSwitchError;

/// Nombre maximal de tentatives par étape de désactivation (7.3) — un échec transitoire
/// (ex: `pkexec`/`vitrail-helper` lent) ne doit pas suffire à marquer l'étape en échec.
const STEP_MAX_ATTEMPTS: u32 = 3;

/// Timeout par tentative : au-delà, la tentative est traitée comme un échec et retentée.
const STEP_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);

/// Backoff fixe entre deux tentatives ratées — volontairement court, jamais déclenché par un
/// `StubSubsystem`/`FakeNftablesBackend` de test puisque leur `stop()` ne peut pas échouer.
const STEP_RETRY_BACKOFF: Duration = Duration::from_millis(200);

pub enum Step<'a> {
    Subsystem(&'a dyn Subsystem),
    Nftables(&'a dyn NftablesBackend),
}

impl Step<'_> {
    fn name(&self) -> &'static str {
        match self {
            Step::Subsystem(s) => s.name(),
            Step::Nftables(_) => "nftables",
        }
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        match self {
            Step::Subsystem(s) => s.start(),
            Step::Nftables(n) => n.apply(),
        }
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        match self {
            Step::Subsystem(s) => s.stop(),
            Step::Nftables(n) => n.flush(),
        }
    }
}

/// Activation : arrête la séquence au premier échec réel, log clairement à quelle étape.
pub fn activate(steps: &[Step]) -> Result<(), KillSwitchError> {
    for step in steps {
        match step.start() {
            Ok(()) => tracing::info!(step = step.name(), "étape d'activation réussie"),
            Err(error) => {
                tracing::error!(step = step.name(), error = %error, "activation interrompue");
                return Err(KillSwitchError::ActivationFailed {
                    step: step.name().to_string(),
                    reason: error.to_string(),
                });
            }
        }
    }
    Ok(())
}

pub struct DeactivationReport {
    pub failed_steps: Vec<(String, String)>,
}

/// Désactivation en ordre inverse strict, best-effort : continue même si une étape échoue,
/// ne reste jamais bloquée à mi-séquence. Chaque étape est retentée (7.3) avec un timeout par
/// tentative ; si toutes les tentatives échouent, l'étape est collectée en échec (jamais
/// propagée) et la séquence continue vers l'étape suivante.
pub fn deactivate(steps: &[Step]) -> DeactivationReport {
    let mut failed_steps = Vec::new();

    for step in steps.iter().rev() {
        if let Err(error) = stop_with_retry(step) {
            failed_steps.push((step.name().to_string(), error));
        }
    }

    DeactivationReport { failed_steps }
}

/// Retente `step.stop()` jusqu'à `STEP_MAX_ATTEMPTS` fois, avec un timeout `STEP_ATTEMPT_TIMEOUT`
/// par tentative. Retourne la dernière erreur rencontrée si toutes les tentatives échouent.
fn stop_with_retry(step: &Step) -> Result<(), String> {
    let mut last_error = String::new();

    for attempt in 1..=STEP_MAX_ATTEMPTS {
        match run_stop_with_timeout(step, STEP_ATTEMPT_TIMEOUT) {
            Ok(()) => {
                tracing::info!(
                    step = step.name(),
                    attempt,
                    "étape de désactivation réussie"
                );
                return Ok(());
            }
            Err(error) => {
                last_error = error;
                if attempt < STEP_MAX_ATTEMPTS {
                    tracing::warn!(
                        step = step.name(),
                        attempt,
                        error = %last_error,
                        "tentative de désactivation échouée, nouvelle tentative"
                    );
                    std::thread::sleep(STEP_RETRY_BACKOFF);
                }
            }
        }
    }

    tracing::error!(
        step = step.name(),
        attempts = STEP_MAX_ATTEMPTS,
        error = %last_error,
        "échec best-effort à la désactivation après épuisement des tentatives"
    );
    Err(last_error)
}

/// Exécute `step.stop()` dans un thread scoped et applique un timeout dessus — un
/// `Subsystem`/`NftablesBackend` réel peut bloquer (process externe via `pkexec`), un stub
/// de test répond instantanément et ne déclenche jamais ce chemin.
fn run_stop_with_timeout(step: &Step, timeout: Duration) -> Result<(), String> {
    let (tx, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let result = step.stop().map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
        rx.recv_timeout(timeout)
            .unwrap_or_else(|_| Err(format!("timeout après {}s", timeout.as_secs())))
    })
}
