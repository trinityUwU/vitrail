//! Construction du `Flow` IPC (story 5.2/5.3) à partir des fragments accumulés par
//! `engine.rs` — champs non couverts par capture/attribution (headers, body, certificat,
//! réservés EPIC 3/4) restent `None`/vides, jamais fabriqués (PLAN.md §6septies).

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::attribution::{AttributionEvent, FiveTuple};
use crate::capture::CapturedPacket;
use crate::keylog::DecryptedFragment;
use crate::shared::{CertificateInfo, CorrelationSource, Flow, HttpHeader};

use super::visibility::determine_visibility;

/// Processus inconnu quand `attribution` n'a pas contribué au flux (visibilité `Meta` ou
/// `Unknown`) — placeholder honnête, pas une valeur fabriquée.
const UNKNOWN_PROCESS: &str = "Processus inconnu";

pub struct PendingFlow {
    pub five_tuple: FiveTuple,
    pub capture: Option<CapturedPacket>,
    pub attribution: Option<AttributionEvent>,
    /// EPIC 3 (PLAN.md §6octies) : fragment déchiffré via `tshark`/SSLKEYLOGFILE — première
    /// source de contenu réelle (`request_headers`/`response_headers`/`body_preview`/
    /// `content_type`/`certificate` restaient vides jusqu'ici, cf. doc de module d'origine).
    pub decryption: Option<DecryptedFragment>,
    pub first_seen: Instant,
}

impl PendingFlow {
    pub fn new(five_tuple: FiveTuple) -> Self {
        Self {
            five_tuple,
            capture: None,
            attribution: None,
            decryption: None,
            first_seen: Instant::now(),
        }
    }
}

/// Contenu extrait d'un fragment déchiffré (EPIC 3) — regroupe les champs
/// method/path/status/headers/body/certificat pour éviter de répéter cinq fois
/// `pending.decryption.as_ref().and_then(...)` dans `build_flow`. Réutilisé tel quel par
/// `correlation::update` (fix audit 5.2) pour enrichir a posteriori un flow déjà émis avec un
/// fragment déchiffré tardif, sans dupliquer cette extraction.
pub(super) struct DecryptionContent {
    pub host: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
    pub request_headers: Vec<HttpHeader>,
    pub response_headers: Vec<HttpHeader>,
    pub body_preview: Option<String>,
    pub content_type: Option<String>,
    pub certificate: Option<CertificateInfo>,
}

impl DecryptionContent {
    pub(super) fn from_fragment(fragment: &DecryptedFragment) -> Self {
        Self {
            host: fragment.host.clone(),
            method: fragment.method.clone(),
            path: fragment.path.clone(),
            status: fragment.status,
            request_headers: fragment.request_headers.clone(),
            response_headers: fragment.response_headers.clone(),
            body_preview: fragment.body_preview.clone(),
            content_type: fragment.content_type.clone(),
            certificate: fragment.certificate.clone(),
        }
    }

    fn empty() -> Self {
        Self {
            host: None,
            method: None,
            path: None,
            status: None,
            request_headers: Vec::new(),
            response_headers: Vec::new(),
            body_preview: None,
            content_type: None,
            certificate: None,
        }
    }
}

pub fn build_flow(pending: &PendingFlow, sequence: u64) -> Flow {
    let has_capture = pending.capture.is_some();
    let has_attribution = pending.attribution.is_some();
    let has_decryption = pending.decryption.is_some();
    // 4ᵉ paramètre = `keylog` (pas `decryption`/EPIC 4-PolarProxy) : cette passe alimente le
    // pipeline SSLKEYLOGFILE, décision explicitée au rapport de livraison EPIC 3.
    let visibility = determine_visibility(has_capture, has_attribution, false, has_decryption);
    let content = pending
        .decryption
        .as_ref()
        .map(DecryptionContent::from_fragment)
        .unwrap_or_else(DecryptionContent::empty);

    Flow {
        id: flow_id(&pending.five_tuple, sequence),
        timestamp: timestamp_hms(),
        process: process_name(pending),
        destination: destination(pending, &content),
        ip: pending.five_tuple.dst_ip.clone(),
        port: pending.five_tuple.dst_port,
        protocol: protocol(pending),
        size_bytes: pending
            .capture
            .as_ref()
            .map(|c| c.bytes as u64)
            .unwrap_or(0),
        // Non mesurable avec capture+attribution+keylog (pas de timing requête/réponse tant que
        // decryption/EPIC 4 n'apporte pas un vrai début/fin d'échange côté proxy) — décision non
        // explicite dans PLAN.md, cf. rapport EPIC 5/EPIC 3.
        duration_ms: 0,
        visibility,
        method: content.method,
        path: content.path,
        status: content.status,
        source_ip: pending.five_tuple.src_ip.clone(),
        source_port: pending.five_tuple.src_port,
        request_headers: content.request_headers,
        response_headers: content.response_headers,
        body_preview: content.body_preview,
        content_type: content.content_type,
        certificate: content.certificate,
        sources: build_sources(has_capture, has_attribution, has_decryption),
    }
}

fn process_name(pending: &PendingFlow) -> String {
    pending
        .attribution
        .as_ref()
        .map(|event| event.app_name.clone())
        .unwrap_or_else(|| UNKNOWN_PROCESS.to_string())
}

/// Hôte déchiffré (keylog) en priorité si présent — sinon SNI (capture), sinon l'IP de
/// destination, toujours disponible via le 5-tuple.
fn destination(pending: &PendingFlow, content: &DecryptionContent) -> String {
    if let Some(host) = &content.host {
        return host.clone();
    }
    pending
        .capture
        .as_ref()
        .and_then(|packet| packet.sni.clone())
        .unwrap_or_else(|| pending.five_tuple.dst_ip.clone())
}

/// Protocole applicatif détecté (capture, ex: "TLS 1.3") si présent, sinon le protocole
/// transport brut (capture ou, à défaut, celui vu côté attribution/OpenSnitch).
fn protocol(pending: &PendingFlow) -> String {
    if let Some(packet) = &pending.capture {
        if let Some(detected) = &packet.detected_protocol {
            return detected.clone();
        }
        return packet.protocol.clone();
    }
    pending.five_tuple.protocol.clone()
}

fn build_sources(
    has_capture: bool,
    has_attribution: bool,
    has_decryption: bool,
) -> Vec<CorrelationSource> {
    vec![
        CorrelationSource {
            name: "Capture".into(),
            status: if has_capture { "ok" } else { "absent" }.into(),
            detail: if has_capture {
                "Paquet vu dans la fenêtre de corrélation"
            } else {
                "Aucun paquet vu dans la fenêtre de corrélation"
            }
            .into(),
        },
        CorrelationSource {
            name: "Attribution".into(),
            status: if has_attribution { "ok" } else { "absent" }.into(),
            detail: if has_attribution {
                "AskRule reçu (OpenSnitch)"
            } else {
                "AskRule non reçu dans la fenêtre de corrélation"
            }
            .into(),
        },
        CorrelationSource {
            name: "Décryptage".into(),
            status: "absent".into(),
            detail: "EPIC 4 non livré".into(),
        },
        CorrelationSource {
            name: "Keylog".into(),
            status: if has_decryption { "ok" } else { "absent" }.into(),
            detail: if has_decryption {
                "Fragment déchiffré reçu via tshark (SSLKEYLOGFILE)"
            } else {
                "Aucun fragment déchiffré dans la fenêtre de corrélation"
            }
            .into(),
        },
    ]
}

/// Id stable et unique par flux : 5-tuple + numéro de séquence global (jamais réutilisé,
/// `AtomicU64` côté `engine.rs`) — pas de dépendance `uuid` ajoutée pour ça (PLAN.md ne
/// tranche rien ici, décision minimale : réutiliser ce qu'on a déjà).
fn flow_id(tuple: &FiveTuple, sequence: u64) -> String {
    format!(
        "{}-{}-{}-{}-{}-{sequence}",
        tuple.protocol, tuple.src_ip, tuple.src_port, tuple.dst_ip, tuple.dst_port
    )
}

/// Même convention que `killswitch::verify::checked_at_string()` (aucune dépendance
/// `chrono`) — affichage `HH:MM:SS` UTC dérivé de l'horloge système.
fn timestamp_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}
