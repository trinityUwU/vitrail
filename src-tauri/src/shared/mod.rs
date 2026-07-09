//! Types communs, config, logging (tracing). Pas de logique métier.
//!
//! `SystemStatus`/`SubsystemStatus`/`TeardownReport` vivent ici (et non dans
//! `commands/types.rs`) car ils sont produits par `killswitch/` (EPIC 7) : `commands/`
//! n'agrège/délègue jamais l'inverse de ce sens (ARCHITECTURE.md). `commands/types.rs` les
//! ré-exporte tels quels pour préserver le contrat IPC existant.
//!
//! `Flow`/`FlowVisibility`/`HttpHeader`/`CertificateInfo`/`CorrelationSource` suivent le même
//! principe (EPIC 5) : produits par `correlation/`, mais consommés aussi par `storage/`
//! (`storage::flows::insert_flow`) — les poser ici plutôt que dans `correlation/` évite que
//! `storage/` dépende d'un domaine métier (frontière stricte, ARCHITECTURE.md : "storage ne
//! contient aucune logique métier de corrélation").

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsystemStatus {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub kill_switch_state: String,
    pub subsystems: Vec<SubsystemStatus>,
    pub last_verification: Option<String>,
    pub last_verification_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeardownReport {
    pub clean: bool,
    pub divergences: Vec<String>,
    pub checked_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FlowVisibility {
    Fully,
    Meta,
    Attrib,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    pub issuer: String,
    pub subject: String,
    pub valid_from: String,
    pub valid_to: String,
    pub fingerprint_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationSource {
    pub name: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub id: String,
    pub timestamp: String,
    pub process: String,
    pub destination: String,
    pub ip: String,
    pub port: u16,
    pub protocol: String,
    pub size_bytes: u64,
    pub duration_ms: u64,
    pub visibility: FlowVisibility,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
    pub source_ip: String,
    pub source_port: u16,
    pub request_headers: Vec<HttpHeader>,
    pub response_headers: Vec<HttpHeader>,
    pub body_preview: Option<String>,
    pub content_type: Option<String>,
    pub certificate: Option<CertificateInfo>,
    pub sources: Vec<CorrelationSource>,
}

/// Verrou de test partagé pour tout module dont les tests manipulent des variables
/// d'environnement globales au process (`XDG_DATA_HOME`/`XDG_DATA_DIRS`/`XDG_RUNTIME_DIR`).
/// Chaque module ayant historiquement défini son propre `static ENV_GUARD` local
/// (`attribution::desktop_resolver`, `attribution::subsystem`), deux tests de modules
/// DIFFÉRENTS pouvaient modifier ces variables en même temps sans exclusion mutuelle réelle
/// (chacun protégé par un mutex distinct) — flaky par construction dès que `cargo test`
/// parallélise plusieurs modules qui les touchent (constaté à l'ajout des tests `keylog::*` en
/// EPIC 3). Centralisé ici : tout module de test touchant ces variables doit utiliser CE verrou,
/// jamais en redéfinir un local.
#[cfg(test)]
pub(crate) static ENV_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
