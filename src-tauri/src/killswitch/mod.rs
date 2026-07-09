//! Cycle de vie orchestré de tous les sous-systèmes, snapshot/diff (EPIC 7).
//!
//! `KillSwitchState` est l'unique point d'entrée public, géré par Tauri via
//! `.manage(...)` et consommé par `commands/killswitch.rs` via `tauri::State`. Les types IPC
//! (`SystemStatus`/`SubsystemStatus`/`TeardownReport`) sont possédés par `crate::shared` et
//! ré-exportés tels quels par `commands/types.rs` (ARCHITECTURE.md : `commands/` n'est jamais
//! la source de types consommés par un domaine).

mod emergency;
mod nftables;
mod sequence;
mod snapshot;
mod subsystem;
mod verify;

#[cfg(test)]
mod tests;

use std::sync::Mutex;

use nftables::{NftablesBackend, SystemNftablesBackend};
use sequence::Step;
use snapshot::{append_event, SystemSnapshot};
use subsystem::StubSubsystem;
use thiserror::Error;

pub use subsystem::Subsystem;

use crate::attribution::AttributionSubsystem;
use crate::capture::CaptureSubsystem;
use crate::shared::{SubsystemStatus, SystemStatus, TeardownReport};
use crate::storage::StorageHandle;

#[derive(Debug, Error)]
pub enum KillSwitchError {
    #[error("échec d'exécution nftables: {0}")]
    NftablesExec(String),
    #[error("échec de persistance system_events: {0}")]
    Persistence(String),
    /// Erreur générique d'exécution d'un sous-système réel (ex: spawn du binaire privilégié
    /// `vitrail-capture-helper`). Décision non explicitement tranchée dans PLAN.md : variante
    /// ajoutée pour donner à `capture::CaptureSubsystem` un `Result` fidèle plutôt que de
    /// détourner `NftablesExec` — réutilisable par les futurs domaines réels (attribution,
    /// decryption, keylog) quand ils remplaceront leur `StubSubsystem`.
    #[error("échec du sous-système {subsystem}: {reason}")]
    SubsystemExec { subsystem: String, reason: String },
    #[error("activation interrompue à l'étape {step}: {reason}")]
    ActivationFailed { step: String, reason: String },
}

struct Inner {
    ca: StubSubsystem,
    nftables: Box<dyn NftablesBackend>,
    polarproxy: StubSubsystem,
    attribution: Box<dyn Subsystem>,
    capture: Box<dyn Subsystem>,
    keylog: StubSubsystem,
    pre_activation_snapshot: Option<SystemSnapshot>,
    active: bool,
    last_verification: Option<TeardownReport>,
    last_verification_clean: bool,
    storage: StorageHandle,
}

pub struct KillSwitchState {
    inner: Mutex<Inner>,
}

impl KillSwitchState {
    /// Ouvre la connexion storage réelle (fichier XDG, EPIC 6) — fallback en mémoire loggé si
    /// l'ouverture échoue, jamais de panic au démarrage de l'app pour une défaillance de
    /// persistance (même philosophie que les erreurs de journalisation dans le reste du
    /// domaine : dégradé et visible, pas fatal). `correlation` (EPIC 5) est cloné dans
    /// `CaptureSubsystem`/`AttributionSubsystem` : chacun publie ses événements retenus vers
    /// `correlation/` en plus de sa persistance `storage::` existante.
    pub fn new(correlation: crate::correlation::CorrelationSender) -> Self {
        let storage = StorageHandle::open_default().unwrap_or_else(|error| {
            tracing::error!(
                error = %error,
                "ouverture de vitrail.db échouée, fallback en mémoire (persistance non garantie)"
            );
            StorageHandle::open_in_memory()
                .expect("ouverture SQLite en mémoire ne doit jamais échouer")
        });

        Self::with_backend(
            Box::new(SystemNftablesBackend),
            Box::new(CaptureSubsystem::new(storage.clone(), correlation.clone())),
            Box::new(AttributionSubsystem::new(storage.clone(), correlation)),
            storage,
        )
    }

    /// Handle storage partagé (même connexion, même `Mutex`) — exposé pour que `lib.rs` le
    /// `.manage()` séparément à destination de `commands/settings.rs` (EPIC 6), sans ouvrir
    /// une deuxième connexion vers le même fichier.
    pub fn storage_handle(&self) -> StorageHandle {
        self.inner
            .lock()
            .expect("mutex killswitch empoisonné")
            .storage
            .clone()
    }

    /// Constructeur pour les tests (7.6/EPIC 2/EPIC 1) : jamais de `SystemNftablesBackend` ni
    /// de vrais `CaptureSubsystem`/`AttributionSubsystem` en test — injecter des variantes en
    /// mémoire (`FakeNftablesBackend`, `capture::FakeCaptureSubsystem`,
    /// `attribution::FakeAttributionSubsystem`) garantit qu'aucun `pkexec` ni aucun process
    /// privilégié réel n'est déclenché par un test. `storage` doit être une connexion en
    /// mémoire (`StorageHandle::open_in_memory()`) en test, jamais le vrai fichier.
    pub fn with_backend(
        nftables: Box<dyn NftablesBackend>,
        capture: Box<dyn Subsystem>,
        attribution: Box<dyn Subsystem>,
        storage: StorageHandle,
    ) -> Self {
        Self {
            inner: Mutex::new(Inner {
                ca: StubSubsystem::new("ca"),
                nftables,
                polarproxy: StubSubsystem::new("polarproxy"),
                attribution,
                capture,
                keylog: StubSubsystem::new("keylog"),
                pre_activation_snapshot: None,
                active: false,
                last_verification: None,
                last_verification_clean: true,
                storage,
            }),
        }
    }

    /// 7.1 (snapshot pré) + 7.2 (séquence CA → nftables → PolarProxy → attribution →
    /// capture → keylog, arrêt au premier échec réel).
    pub fn activate(&self) -> SystemStatus {
        let mut inner = self.inner.lock().expect("mutex killswitch empoisonné");

        let pre = capture_snapshot(&inner);
        let _ = append_event(&inner.storage, "pre-activation", &pre);
        inner.pre_activation_snapshot = Some(pre);

        let outcome = {
            let steps = build_steps(&inner);
            sequence::activate(&steps)
        };
        let failed_step = match &outcome {
            Err(KillSwitchError::ActivationFailed { step, .. }) => Some(step.clone()),
            _ => None,
        };
        inner.active = outcome.is_ok();

        let post = capture_snapshot(&inner);
        let _ = append_event(&inner.storage, "post-activation", &post);

        let label = if outcome.is_ok() {
            "active"
        } else {
            "degraded"
        };
        build_status(&inner, label, failed_step.as_deref())
    }

    /// 7.3 (ordre inverse, best-effort) + 7.4 (diff pré/post-activation systématique).
    pub fn deactivate(&self) -> SystemStatus {
        let mut inner = self.inner.lock().expect("mutex killswitch empoisonné");

        let report = {
            let steps = build_steps(&inner);
            sequence::deactivate(&steps)
        };
        inner.active = false;

        let post = capture_snapshot(&inner);
        let _ = append_event(&inner.storage, "post-deactivation", &post);

        let verification = match &inner.pre_activation_snapshot {
            Some(pre) => verify::diff(pre, &post),
            None => verify::no_prior_activation(),
        };
        inner.last_verification_clean = verification.clean;
        inner.last_verification = Some(verification.clone());

        let label = if report.failed_steps.is_empty() && verification.clean {
            "inactive"
        } else {
            "degraded"
        };
        let failed_step = report.failed_steps.first().map(|(name, _)| name.as_str());
        build_status(&inner, label, failed_step)
    }

    /// 7.5 — hors séquence, best-effort, priorité à la restauration réseau.
    pub fn emergency_stop(&self) -> SystemStatus {
        let mut inner = self.inner.lock().expect("mutex killswitch empoisonné");

        let report = {
            let refs = build_subsystem_refs(&inner);
            emergency::emergency_stop(&*inner.nftables, &refs)
        };
        inner.active = false;
        inner.last_verification = None;
        inner.last_verification_clean = false;

        let post = capture_snapshot(&inner);
        let _ = append_event(&inner.storage, "emergency-stop", &post);

        let label = if report.unconfirmed_steps.is_empty() {
            "inactive"
        } else {
            "degraded"
        };
        let failed_step = report.unconfirmed_steps.first().map(|s| s.as_str());
        build_status(&inner, label, failed_step)
    }

    pub fn current_status(&self) -> SystemStatus {
        let inner = self.inner.lock().expect("mutex killswitch empoisonné");
        let label = if inner.active { "active" } else { "inactive" };
        build_status(&inner, label, None)
    }

    /// 7.4 à la demande (hors désactivation) : re-diff l'état courant contre le dernier
    /// snapshot pré-activation connu.
    pub fn verify_teardown(&self) -> TeardownReport {
        let mut inner = self.inner.lock().expect("mutex killswitch empoisonné");
        let post = capture_snapshot(&inner);
        let report = match &inner.pre_activation_snapshot {
            Some(pre) => verify::diff(pre, &post),
            None => verify::no_prior_activation(),
        };
        inner.last_verification_clean = report.clean;
        inner.last_verification = Some(report.clone());
        report
    }
}

impl Default for KillSwitchState {
    /// `Default` ne peut pas prendre de paramètre : construit un canal de corrélation
    /// jetable (récepteur immédiatement abandonné, `correlation/` reste inactive) — jamais
    /// utilisé par `lib.rs` (qui appelle `new(correlation)` avec le vrai récepteur), présent
    /// uniquement pour satisfaire l'idiome Rust `Default`/`new()` sans argument caché.
    fn default() -> Self {
        Self::new(crate::correlation::channel().0)
    }
}

fn capture_snapshot(inner: &Inner) -> SystemSnapshot {
    let refs = build_subsystem_refs(inner);
    SystemSnapshot::capture(&*inner.nftables, &refs)
}

fn build_subsystem_refs(inner: &Inner) -> Vec<&dyn Subsystem> {
    vec![
        &inner.ca,
        &inner.polarproxy,
        &*inner.attribution,
        &*inner.capture,
        &inner.keylog,
    ]
}

/// Ordre CA → nftables → PolarProxy → attribution → capture → keylog câblé une fois pour
/// toutes ici (PLAN.md §6ter) — `sequence.rs` ne fait qu'itérer ce vecteur.
fn build_steps(inner: &Inner) -> Vec<Step<'_>> {
    vec![
        Step::Subsystem(&inner.ca),
        Step::Nftables(&*inner.nftables),
        Step::Subsystem(&inner.polarproxy),
        Step::Subsystem(&*inner.attribution),
        Step::Subsystem(&*inner.capture),
        Step::Subsystem(&inner.keylog),
    ]
}

fn build_status(inner: &Inner, label: &str, failed_step: Option<&str>) -> SystemStatus {
    SystemStatus {
        kill_switch_state: label.to_string(),
        subsystems: subsystem_statuses(inner, failed_step),
        last_verification: inner
            .last_verification
            .as_ref()
            .map(|r| r.checked_at.clone()),
        last_verification_clean: inner.last_verification_clean,
    }
}

fn subsystem_entries(inner: &Inner) -> [(&'static str, &'static str, &'static str, bool); 6] {
    [
        (
            "ca",
            "CA installée",
            "Certificat racine dans le trust store",
            inner.ca.is_active(),
        ),
        (
            "nftables",
            "Règles nftables",
            "Chaîne VITRAIL_REDIRECT active",
            inner.nftables.is_applied(),
        ),
        (
            "polarproxy",
            "Décryptage (PolarProxy)",
            "Intercepteur TLS/MITM local",
            inner.polarproxy.is_active(),
        ),
        (
            "attribution",
            "Attribution (OpenSnitch)",
            "Daemon de corrélation processus/flux",
            inner.attribution.is_active(),
        ),
        (
            "capture",
            "Capture réseau",
            "Capture AF_PACKET brute",
            inner.capture.is_active(),
        ),
        (
            "keylog",
            "Keylog SSLKEYLOGFILE",
            "Export de clés par les applications",
            inner.keylog.is_active(),
        ),
    ]
}

fn subsystem_statuses(inner: &Inner, failed_step: Option<&str>) -> Vec<SubsystemStatus> {
    subsystem_entries(inner)
        .into_iter()
        .map(|(id, name, detail, active)| SubsystemStatus {
            id: id.to_string(),
            name: name.to_string(),
            detail: detail.to_string(),
            status: status_label(id, active, failed_step).to_string(),
        })
        .collect()
}

fn status_label(id: &str, active: bool, failed_step: Option<&str>) -> &'static str {
    if failed_step == Some(id) {
        "err"
    } else if active {
        "ok"
    } else {
        "off"
    }
}
