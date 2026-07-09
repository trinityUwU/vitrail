//! Génération des flux mockés — extrait de `mock_data.rs` pour rester sous la limite de
//! 500 lignes (code-standards.md). Même source de démo (`docs/Mockup.html`), enrichie des
//! champs de contenu déchiffré, certificat et sources de corrélation (EPIC 8.5, partie A) :
//! remplacée à terme par `correlation`/`storage` (EPICs 1 à 7).

use super::mock_data::destinations;
use super::types::{
    CertificateInfo, CorrelationSource, DestinationInfo, Flow, FlowVisibility, HttpHeader,
};

const SOURCE_IP: &str = "192.168.1.42";

struct FlowSeed {
    id: &'static str,
    timestamp: &'static str,
    process: &'static str,
    destination: &'static str,
    ip: &'static str,
    port: u16,
    protocol: &'static str,
    size_bytes: u64,
    duration_ms: u64,
    visibility: FlowVisibility,
    method: Option<&'static str>,
    path: Option<&'static str>,
    status: Option<u16>,
}

fn flow_seeds() -> Vec<FlowSeed> {
    vec![
        FlowSeed {
            id: "f0",
            timestamp: "14:58:44",
            process: "Google Chrome",
            destination: "api.google.com",
            ip: "142.250.74.238",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 45230,
            duration_ms: 820,
            visibility: FlowVisibility::Fully,
            method: Some("GET"),
            path: Some("/search?q=vitrail"),
            status: Some(200),
        },
        FlowSeed {
            id: "f1",
            timestamp: "14:57:01",
            process: "Google Chrome",
            destination: "cdn.jsdelivr.net",
            ip: "151.101.1.229",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 18200,
            duration_ms: 340,
            visibility: FlowVisibility::Fully,
            method: Some("GET"),
            path: Some("/npm/lucide@latest"),
            status: Some(200),
        },
        FlowSeed {
            id: "f2",
            timestamp: "14:58:30",
            process: "Slack",
            destination: "slack.com",
            ip: "34.237.168.12",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 8920,
            duration_ms: 1200,
            visibility: FlowVisibility::Meta,
            method: None,
            path: None,
            status: None,
        },
        FlowSeed {
            id: "f3",
            timestamp: "14:58:50",
            process: "Discord",
            destination: "discord.gg",
            ip: "162.159.135.232",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 23100,
            duration_ms: 980,
            visibility: FlowVisibility::Meta,
            method: None,
            path: None,
            status: None,
        },
        FlowSeed {
            id: "f4",
            timestamp: "14:55:19",
            process: "VS Code",
            destination: "github.com",
            ip: "140.82.121.4",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 8900,
            duration_ms: 410,
            visibility: FlowVisibility::Fully,
            method: Some("GET"),
            path: Some("/api/v3/repos/user/project"),
            status: Some(200),
        },
        FlowSeed {
            id: "f5",
            timestamp: "14:56:33",
            process: "Docker Desktop",
            destination: "registry-1.docker.io",
            ip: "52.5.133.89",
            port: 443,
            protocol: "TLS 1.2",
            size_bytes: 523000,
            duration_ms: 4200,
            visibility: FlowVisibility::Attrib,
            method: None,
            path: None,
            status: None,
        },
        FlowSeed {
            id: "f6",
            timestamp: "14:58:55",
            process: "Spotify",
            destination: "spotify.com",
            ip: "35.186.224.45",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 245000,
            duration_ms: 2100,
            visibility: FlowVisibility::Fully,
            method: Some("GET"),
            path: Some("/audio/track/8f2k3j"),
            status: Some(206),
        },
        FlowSeed {
            id: "f7",
            timestamp: "14:58:40",
            process: "Firefox",
            destination: "play.googleapis.com",
            ip: "142.250.74.174",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 3200,
            duration_ms: 190,
            visibility: FlowVisibility::Fully,
            method: Some("POST"),
            path: Some("/fdls/subscriptions"),
            status: Some(200),
        },
        FlowSeed {
            id: "f8",
            timestamp: "14:45:22",
            process: "Node.js",
            destination: "crates.io",
            ip: "18.214.128.42",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 5600,
            duration_ms: 260,
            visibility: FlowVisibility::Fully,
            method: Some("GET"),
            path: Some("/api/v1/crates/serde"),
            status: Some(200),
        },
        FlowSeed {
            id: "f9",
            timestamp: "14:30:15",
            process: "Google Chrome",
            destination: "fonts.googleapis.com",
            ip: "142.250.74.238",
            port: 443,
            protocol: "TLS 1.3",
            size_bytes: 4200,
            duration_ms: 120,
            visibility: FlowVisibility::Fully,
            method: Some("GET"),
            path: Some("/css2?family=Outfit"),
            status: Some(200),
        },
    ]
}

/// Statut par sous-système de corrélation pour un niveau de visibilité donné — reprend la
/// logique de déduction précédemment câblée dans `InspectorSources.tsx::buildSources()`.
fn build_sources(visibility: FlowVisibility) -> Vec<CorrelationSource> {
    let (capture_status, capture_detail) = if matches!(visibility, FlowVisibility::Attrib) {
        ("off", "Non applicable")
    } else {
        ("ok", "nftables redirect")
    };
    let (decrypt_status, decrypt_detail) = match visibility {
        FlowVisibility::Fully => ("ok", "PolarProxy"),
        FlowVisibility::Meta => ("warn", "Échoué (pinning)"),
        FlowVisibility::Attrib | FlowVisibility::Unknown => ("off", "Non applicable"),
    };
    vec![
        CorrelationSource {
            name: "Attribution".into(),
            status: "ok".into(),
            detail: "OpenSnitch".into(),
        },
        CorrelationSource {
            name: "Capture".into(),
            status: capture_status.into(),
            detail: capture_detail.into(),
        },
        CorrelationSource {
            name: "Décryptage".into(),
            status: decrypt_status.into(),
            detail: decrypt_detail.into(),
        },
        CorrelationSource {
            name: "Keylog".into(),
            status: "off".into(),
            detail: "Non utilisé pour ce flux".into(),
        },
    ]
}

/// Requête/réponse HTTP factices pour les flux entièrement déchiffrés — contenu repris de
/// l'ancien template en dur de `InspectorContent.tsx`.
fn http_exchange(
    seed: &FlowSeed,
) -> (
    Vec<HttpHeader>,
    Vec<HttpHeader>,
    Option<String>,
    Option<String>,
) {
    let request_headers = vec![
        HttpHeader {
            name: "Host".into(),
            value: seed.destination.into(),
        },
        HttpHeader {
            name: "Accept".into(),
            value: "application/json, text/plain, */*".into(),
        },
        HttpHeader {
            name: "Connection".into(),
            value: "keep-alive".into(),
        },
    ];
    let response_headers = vec![
        HttpHeader {
            name: "Content-Type".into(),
            value: "application/json; charset=utf-8".into(),
        },
        HttpHeader {
            name: "Content-Length".into(),
            value: seed.size_bytes.to_string(),
        },
        HttpHeader {
            name: "X-Request-Id".into(),
            value: format!("{}-req", seed.id),
        },
    ];
    let body_preview = Some(format!(
        r#"{{"timestamp":"{}","data":[...]}}"#,
        seed.timestamp
    ));
    let content_type = Some("application/json; charset=utf-8".into());
    (
        request_headers,
        response_headers,
        body_preview,
        content_type,
    )
}

/// Certificat vu par le MITM local, présent uniquement quand la destination associée est
/// marquée `tls == true` dans `mock_data::destinations()`.
fn certificate_for(domain: &str, destinations: &[DestinationInfo]) -> Option<CertificateInfo> {
    let dest = destinations.iter().find(|d| d.domain == domain)?;
    if !dest.tls {
        return None;
    }
    let fingerprint_seed: u32 = domain.bytes().map(u32::from).sum();
    Some(CertificateInfo {
        issuer: "Vitrail Local CA".into(),
        subject: domain.into(),
        valid_from: "2026-01-01T00:00:00Z".into(),
        valid_to: "2027-01-01T00:00:00Z".into(),
        fingerprint_sha256: format!(
            "SHA256:{fingerprint_seed:08X}...{:04X}",
            fingerprint_seed ^ 0xBEEF
        ),
    })
}

fn build_flow(seed: FlowSeed, index: u16, destinations: &[DestinationInfo]) -> Flow {
    let (request_headers, response_headers, body_preview, content_type) =
        if matches!(seed.visibility, FlowVisibility::Fully) && seed.method.is_some() {
            http_exchange(&seed)
        } else {
            (Vec::new(), Vec::new(), None, None)
        };

    Flow {
        id: seed.id.into(),
        timestamp: seed.timestamp.into(),
        process: seed.process.into(),
        destination: seed.destination.into(),
        ip: seed.ip.into(),
        port: seed.port,
        protocol: seed.protocol.into(),
        size_bytes: seed.size_bytes,
        duration_ms: seed.duration_ms,
        visibility: seed.visibility,
        method: seed.method.map(Into::into),
        path: seed.path.map(Into::into),
        status: seed.status,
        source_ip: SOURCE_IP.into(),
        source_port: 51000 + index * 7,
        request_headers,
        response_headers,
        body_preview,
        content_type,
        certificate: certificate_for(seed.destination, destinations),
        sources: build_sources(seed.visibility),
    }
}

pub fn flows() -> Vec<Flow> {
    let dests = destinations();
    flow_seeds()
        .into_iter()
        .enumerate()
        .map(|(i, seed)| build_flow(seed, i as u16, &dests))
        .collect()
}
