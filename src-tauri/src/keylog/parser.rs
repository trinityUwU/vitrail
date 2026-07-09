//! Parsing du flux JSON `-T ek` de tshark (story 3.3/3.4) — délègue tout le déchiffrement/la
//! reconstruction HTTP à tshark (PLAN.md §6octies), ce module se contente d'interpréter son
//! JSON. Format basé sur la documentation publique de Wireshark (`-T ek` = un doc Elasticsearch
//! Bulk par paquet : une ligne "index" `{"index":{}}` suivie d'une ligne de données
//! `{"timestamp":...,"layers":{...}}`) — PAS vérifié contre une sortie réelle (`tshark` absent
//! de cette machine de dev, cf. rapport de livraison). Noms de champs candidats gérés en best
//! effort avec repli sur `None`, jamais de panic sur une ligne inattendue (données externes non
//! fiables, même discipline que `capture::packet`/`capture::tls_sni`).

use serde::Deserialize;
use serde_json::Value;

use crate::attribution::FiveTuple;
use crate::shared::{CertificateInfo, HttpHeader};

const BODY_PREVIEW_MAX_BYTES: usize = 2000;

#[derive(Debug, Clone)]
pub struct DecryptedFragment {
    pub five_tuple: FiveTuple,
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

/// `{"index":{...}}` — ligne d'action Elasticsearch Bulk précédant chaque doc, sans contenu
/// utile pour Vitrail.
#[derive(Deserialize)]
struct IndexAction {
    #[allow(dead_code)]
    index: Value,
}

/// Parse une ligne du flux `-T ek` — `None` pour une ligne "index", malformée, ou sans 5-tuple
/// exploitable (jamais de panic, story 3.4).
pub fn parse_ek_line(line: &str) -> Option<DecryptedFragment> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    if serde_json::from_str::<IndexAction>(trimmed).is_ok() {
        return None;
    }

    let value: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(error) => {
            tracing::warn!(error = %error, line = %trimmed, "ligne JSON tshark -T ek invalide, ignorée");
            return None;
        }
    };

    let layers = value.get("layers")?;
    let five_tuple = extract_five_tuple(layers)?;

    Some(DecryptedFragment {
        five_tuple,
        host: text(layers, &["http_http_host", "http_http_request_line_host"]),
        method: text(layers, &["http_http_request_method"]),
        path: text(
            layers,
            &["http_http_request_uri", "http_http_request_full_uri"],
        ),
        status: text(layers, &["http_http_response_code"]).and_then(|s| s.parse().ok()),
        request_headers: extract_headers(layers, "http_http_request_line"),
        response_headers: extract_headers(layers, "http_http_response_line"),
        body_preview: extract_body_preview(layers),
        content_type: text(layers, &["http_http_content_type"]),
        certificate: extract_certificate(layers),
    })
}

fn extract_five_tuple(layers: &Value) -> Option<FiveTuple> {
    let src_ip = text(layers, &["ip_ip_src", "ipv6_ipv6_src"])?;
    let dst_ip = text(layers, &["ip_ip_dst", "ipv6_ipv6_dst"])?;
    let (protocol, src_port, dst_port) = if let Some(sp) = text(layers, &["tcp_tcp_srcport"]) {
        ("tcp".to_string(), sp, text(layers, &["tcp_tcp_dstport"])?)
    } else {
        (
            "udp".to_string(),
            text(layers, &["udp_udp_srcport"])?,
            text(layers, &["udp_udp_dstport"])?,
        )
    };
    Some(FiveTuple {
        protocol,
        src_ip: crate::attribution::normalize_ip(&src_ip),
        src_port: src_port.parse().ok()?,
        dst_ip: crate::attribution::normalize_ip(&dst_ip),
        dst_port: dst_port.parse().ok()?,
    })
}

/// Cherche le premier champ existant parmi plusieurs noms candidats — le format `-T ek` varie
/// selon la version de Wireshark, jamais une seule clé supposée fixe.
fn text(layers: &Value, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find_map(|key| layers.get(key).and_then(value_as_string))
}

/// Un champ `-T ek` peut être une chaîne seule ou un tableau à un élément (répétition de
/// dissecteur Wireshark) — gère les deux formes sans paniquer.
fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items.first().and_then(value_as_string),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn extract_headers(layers: &Value, key: &str) -> Vec<HttpHeader> {
    let Some(value) = layers.get(key) else {
        return Vec::new();
    };
    let lines: Vec<String> = match value {
        Value::Array(items) => items.iter().filter_map(value_as_string).collect(),
        Value::String(s) => vec![s.clone()],
        _ => return Vec::new(),
    };
    lines
        .iter()
        .filter_map(|line| line.split_once(": "))
        .map(|(name, value)| HttpHeader {
            name: name.trim().to_string(),
            value: value.trim().to_string(),
        })
        .collect()
}

fn extract_body_preview(layers: &Value) -> Option<String> {
    let raw = text(layers, &["http_http_file_data", "http_data_data"])?;
    Some(truncate_utf8(&raw, BODY_PREVIEW_MAX_BYTES))
}

/// Tronque à `max_bytes` sans couper un caractère UTF-8 en deux (`body_preview` provient d'un
/// contenu externe non fiable, même discipline que le reste du projet).
fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn extract_certificate(layers: &Value) -> Option<CertificateInfo> {
    let issuer = text(layers, &["tls_tls_handshake_certificate_issuer"])?;
    let subject = text(layers, &["tls_tls_handshake_certificate_subject"]).unwrap_or_default();
    let valid_from = text(layers, &["tls_tls_handshake_certificate_notbefore"]).unwrap_or_default();
    let valid_to = text(layers, &["tls_tls_handshake_certificate_notafter"]).unwrap_or_default();
    let fingerprint =
        text(layers, &["tls_tls_handshake_certificate_fingerprint"]).unwrap_or_default();
    Some(CertificateInfo {
        issuer,
        subject,
        valid_from,
        valid_to,
        fingerprint_sha256: fingerprint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture construite à la main d'après la doc publique `-T ek` — pas une sortie réelle
    /// vérifiée (cf. doc de module).
    fn http_doc_line() -> String {
        r#"{"timestamp":"1720000000000","layers":{
            "ip_ip_src":"192.168.1.42","ip_ip_dst":"93.184.216.34",
            "tcp_tcp_srcport":"51000","tcp_tcp_dstport":"443",
            "http_http_host":"example.com","http_http_request_method":"GET",
            "http_http_request_uri":"/api/ping","http_http_response_code":"200",
            "http_http_content_type":"application/json",
            "http_http_request_line":["User-Agent: curl/8.0","Host: example.com"],
            "http_http_response_line":["Content-Type: application/json"],
            "http_http_file_data":"{\"ok\":true}"
        }}"#
        .to_string()
    }

    #[test]
    fn ignore_la_ligne_index_elasticsearch_bulk() {
        assert!(parse_ek_line(r#"{"index":{}}"#).is_none());
    }

    #[test]
    fn ignore_une_ligne_vide_ou_malformee_sans_paniquer() {
        assert!(parse_ek_line("").is_none());
        assert!(parse_ek_line("pas du json").is_none());
        assert!(parse_ek_line(r#"{"layers": "pas un objet attendu ici"}"#).is_none());
    }

    #[test]
    fn parse_un_fragment_http_complet() {
        let fragment = parse_ek_line(&http_doc_line()).expect("doit parser la fixture HTTP");
        assert_eq!(fragment.five_tuple.protocol, "tcp");
        assert_eq!(fragment.five_tuple.src_port, 51000);
        assert_eq!(fragment.five_tuple.dst_port, 443);
        assert_eq!(fragment.host.as_deref(), Some("example.com"));
        assert_eq!(fragment.method.as_deref(), Some("GET"));
        assert_eq!(fragment.path.as_deref(), Some("/api/ping"));
        assert_eq!(fragment.status, Some(200));
        assert_eq!(fragment.content_type.as_deref(), Some("application/json"));
        assert_eq!(fragment.request_headers.len(), 2);
        assert_eq!(fragment.request_headers[0].name, "User-Agent");
        assert_eq!(fragment.body_preview.as_deref(), Some("{\"ok\":true}"));
        assert!(
            fragment.certificate.is_none(),
            "aucun champ certificat dans la fixture"
        );
    }

    #[test]
    fn sans_cinq_tuple_exploitable_renvoie_none() {
        let line = r#"{"layers":{"http_http_host":"example.com"}}"#;
        assert!(parse_ek_line(line).is_none());
    }

    #[test]
    fn tronque_le_body_preview_a_2000_octets_sans_couper_un_caractere_utf8() {
        let long_body = "é".repeat(1500); // 3000 octets en UTF-8
        let truncated = truncate_utf8(&long_body, BODY_PREVIEW_MAX_BYTES);
        assert!(truncated.len() <= BODY_PREVIEW_MAX_BYTES);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }
}
