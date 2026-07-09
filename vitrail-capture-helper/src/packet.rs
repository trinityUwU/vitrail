//! Parsing d'une trame Ethernet en enregistrement de capture : 5-tuple, timestamp, taille,
//! SNI (si ClientHello TLS détecté), protocole best-effort (stories 2.2/2.3/2.4).

use std::time::{SystemTime, UNIX_EPOCH};

use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;
use serde::Serialize;

use crate::tls_sni::extract_sni;

#[derive(Debug, Serialize)]
pub struct CapturedPacket {
    pub timestamp_unix_ms: u128,
    pub interface: String,
    pub protocol: &'static str,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub bytes: usize,
    pub sni: Option<String>,
    pub detected_protocol: Option<&'static str>,
}

/// 5-tuple intermédiaire, pas encore sérialisé — porte aussi le payload L4 nécessaire à
/// l'extraction SNI et à la détection de protocole best-effort.
pub(crate) struct FiveTuple {
    pub(crate) protocol: &'static str,
    pub(crate) src_ip: String,
    pub(crate) dst_ip: String,
    pub(crate) src_port: Option<u16>,
    pub(crate) dst_port: Option<u16>,
    pub(crate) payload: Vec<u8>,
}

pub fn parse_ethernet_frame(raw_frame: &[u8], interface_name: &str) -> Option<CapturedPacket> {
    let ethernet = EthernetPacket::new(raw_frame)?;
    let tuple = match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => parse_ipv4(&ethernet)?,
        EtherTypes::Ipv6 => parse_ipv6(&ethernet)?,
        _ => return None,
    };

    let sni = extract_sni(&tuple);
    let detected_protocol = detect_protocol(&tuple, sni.is_some());

    Some(CapturedPacket {
        timestamp_unix_ms: now_unix_ms(),
        interface: interface_name.to_string(),
        protocol: tuple.protocol,
        src_ip: tuple.src_ip,
        dst_ip: tuple.dst_ip,
        src_port: tuple.src_port,
        dst_port: tuple.dst_port,
        bytes: raw_frame.len(),
        sni,
        detected_protocol,
    })
}

fn parse_ipv4(ethernet: &EthernetPacket) -> Option<FiveTuple> {
    let ipv4 = Ipv4Packet::new(ethernet.payload())?;
    let src_ip = ipv4.get_source().to_string();
    let dst_ip = ipv4.get_destination().to_string();
    parse_l4(
        ipv4.get_next_level_protocol(),
        ipv4.payload(),
        src_ip,
        dst_ip,
    )
}

fn parse_ipv6(ethernet: &EthernetPacket) -> Option<FiveTuple> {
    let ipv6 = Ipv6Packet::new(ethernet.payload())?;
    let src_ip = ipv6.get_source().to_string();
    let dst_ip = ipv6.get_destination().to_string();
    parse_l4(ipv6.get_next_header(), ipv6.payload(), src_ip, dst_ip)
}

fn parse_l4(
    next_header: pnet::packet::ip::IpNextHeaderProtocol,
    payload: &[u8],
    src_ip: String,
    dst_ip: String,
) -> Option<FiveTuple> {
    match next_header {
        IpNextHeaderProtocols::Tcp => parse_tcp(payload, src_ip, dst_ip),
        IpNextHeaderProtocols::Udp => parse_udp(payload, src_ip, dst_ip),
        _ => Some(FiveTuple {
            protocol: "other",
            src_ip,
            dst_ip,
            src_port: None,
            dst_port: None,
            payload: Vec::new(),
        }),
    }
}

fn parse_tcp(payload: &[u8], src_ip: String, dst_ip: String) -> Option<FiveTuple> {
    let tcp = TcpPacket::new(payload)?;
    Some(FiveTuple {
        protocol: "tcp",
        src_ip,
        dst_ip,
        src_port: Some(tcp.get_source()),
        dst_port: Some(tcp.get_destination()),
        payload: tcp.payload().to_vec(),
    })
}

fn parse_udp(payload: &[u8], src_ip: String, dst_ip: String) -> Option<FiveTuple> {
    let udp = UdpPacket::new(payload)?;
    Some(FiveTuple {
        protocol: "udp",
        src_ip,
        dst_ip,
        src_port: Some(udp.get_source()),
        dst_port: Some(udp.get_destination()),
        payload: udp.payload().to_vec(),
    })
}

/// Détection best-effort (story 2.4) : pas d'exhaustivité recherchée. Le SNI déjà extrait
/// prime (implique TLS) ; sinon on retombe sur les ports connus, puis sur des heuristiques de
/// contenu pour QUIC et le HTTP en clair.
fn detect_protocol(tuple: &FiveTuple, has_sni: bool) -> Option<&'static str> {
    if has_sni {
        return Some("tls");
    }
    match (tuple.protocol, tuple.src_port, tuple.dst_port) {
        ("udp", Some(53), _) | ("udp", _, Some(53)) => Some("dns"),
        ("tcp", Some(53), _) | ("tcp", _, Some(53)) => Some("dns"),
        ("udp", Some(443), _) | ("udp", _, Some(443)) => detect_quic(&tuple.payload),
        ("tcp", Some(443), _) | ("tcp", _, Some(443)) => Some("tls"),
        ("tcp", Some(80), _) | ("tcp", _, Some(80)) => Some("http"),
        _ => detect_plaintext_http(&tuple.payload),
    }
}

/// En-tête long QUIC reconnaissable au bit de poids fort du premier octet (RFC 9000 §17.2).
/// Heuristique volontairement simple — pas de parsing complet du protocole.
fn detect_quic(payload: &[u8]) -> Option<&'static str> {
    if payload.first().is_some_and(|byte| byte & 0x80 != 0) {
        Some("quic")
    } else {
        None
    }
}

fn detect_plaintext_http(payload: &[u8]) -> Option<&'static str> {
    const PATTERNS: [&[u8]; 4] = [b"GET /", b"POST /", b"PUT /", b"HTTP/1."];
    if PATTERNS.iter().any(|pattern| payload.starts_with(pattern)) {
        Some("http")
    } else {
        None
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ETHERTYPE_IPV4: [u8; 2] = [0x08, 0x00];
    const ETHERTYPE_IPV6: [u8; 2] = [0x86, 0xDD];
    const ETHERTYPE_ARP: [u8; 2] = [0x08, 0x06];

    fn ethernet_header(ethertype: [u8; 2]) -> Vec<u8> {
        let mut frame = vec![0xAA; 6]; // dst mac
        frame.extend(vec![0xBB; 6]); // src mac
        frame.extend(ethertype);
        frame
    }

    /// En-tête IPv4 minimal (20 octets, pas d'options), checksum non calculée — sans
    /// importance ici, `Ipv4Packet::new` ne la valide pas.
    fn ipv4_header(protocol: u8, payload_len: usize) -> Vec<u8> {
        let total_len = (20 + payload_len) as u16;
        let mut header = vec![
            0x45, // version 4, IHL 5 (20 octets)
            0x00, // DSCP/ECN
        ];
        header.extend(total_len.to_be_bytes());
        header.extend([0x00, 0x00]); // identification
        header.extend([0x00, 0x00]); // flags/fragment offset
        header.push(64); // TTL
        header.push(protocol);
        header.extend([0x00, 0x00]); // checksum (non calculée)
        header.extend([192, 168, 1, 10]); // src
        header.extend([192, 168, 1, 20]); // dst
        header
    }

    fn ipv6_header(next_header: u8, payload_len: usize) -> Vec<u8> {
        let mut header = vec![0x60, 0x00, 0x00, 0x00]; // version 6, traffic class, flow label
        header.extend((payload_len as u16).to_be_bytes());
        header.push(next_header);
        header.push(64); // hop limit
        header.extend([0xFE; 16]); // src
        header.extend([0xFD; 16]); // dst
        header
    }

    fn tcp_header(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend(src_port.to_be_bytes());
        header.extend(dst_port.to_be_bytes());
        header.extend([0x00; 4]); // seq
        header.extend([0x00; 4]); // ack
        header.push(0x50); // data offset 5 (20 octets), reserved
        header.push(0x18); // flags (PSH|ACK)
        header.extend([0xFF, 0xFF]); // window
        header.extend([0x00, 0x00]); // checksum
        header.extend([0x00, 0x00]); // urgent pointer
        header.extend(payload);
        header
    }

    fn udp_header(src_port: u16, dst_port: u16, payload: &[u8]) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend(src_port.to_be_bytes());
        header.extend(dst_port.to_be_bytes());
        let len = (8 + payload.len()) as u16;
        header.extend(len.to_be_bytes());
        header.extend([0x00, 0x00]); // checksum
        header.extend(payload);
        header
    }

    #[test]
    fn parses_valid_ipv4_tcp_frame() {
        let tcp = tcp_header(51000, 443, b"payload");
        let ipv4 = ipv4_header(6, tcp.len()); // 6 = TCP
        let mut frame = ethernet_header(ETHERTYPE_IPV4);
        frame.extend(ipv4);
        frame.extend(tcp);

        let record = parse_ethernet_frame(&frame, "eth0").expect("frame TCP valide attendue");
        assert_eq!(record.protocol, "tcp");
        assert_eq!(record.src_ip, "192.168.1.10");
        assert_eq!(record.dst_ip, "192.168.1.20");
        assert_eq!(record.src_port, Some(51000));
        assert_eq!(record.dst_port, Some(443));
        assert_eq!(record.interface, "eth0");
    }

    #[test]
    fn parses_valid_ipv4_udp_frame() {
        let udp = udp_header(60000, 53, b"query");
        let ipv4 = ipv4_header(17, udp.len()); // 17 = UDP
        let mut frame = ethernet_header(ETHERTYPE_IPV4);
        frame.extend(ipv4);
        frame.extend(udp);

        let record = parse_ethernet_frame(&frame, "eth0").expect("frame UDP valide attendue");
        assert_eq!(record.protocol, "udp");
        assert_eq!(record.src_port, Some(60000));
        assert_eq!(record.dst_port, Some(53));
        assert_eq!(record.detected_protocol, Some("dns"));
    }

    #[test]
    fn parses_valid_ipv6_tcp_frame() {
        let tcp = tcp_header(51000, 443, b"payload");
        let ipv6 = ipv6_header(6, tcp.len());
        let mut frame = ethernet_header(ETHERTYPE_IPV6);
        frame.extend(ipv6);
        frame.extend(tcp);

        let record = parse_ethernet_frame(&frame, "eth0").expect("frame IPv6/TCP valide attendue");
        assert_eq!(record.protocol, "tcp");
        assert_eq!(record.dst_port, Some(443));
    }

    #[test]
    fn truncated_frame_returns_none_without_panicking() {
        // Ethernet header seul, aucune charge utile.
        let frame = ethernet_header(ETHERTYPE_IPV4);
        assert!(parse_ethernet_frame(&frame, "eth0").is_none());

        // Ethernet + IPv4 déclarant un total_len supérieur aux octets réellement présents.
        let mut frame = ethernet_header(ETHERTYPE_IPV4);
        frame.extend(ipv4_header(6, 20));
        frame.truncate(frame.len() - 5); // coupe en plein milieu de l'en-tête IPv4
        assert!(parse_ethernet_frame(&frame, "eth0").is_none());

        // Frame vide.
        assert!(parse_ethernet_frame(&[], "eth0").is_none());
    }

    #[test]
    fn unknown_ethertype_returns_none() {
        let frame = ethernet_header(ETHERTYPE_ARP);
        assert!(parse_ethernet_frame(&frame, "eth0").is_none());
    }
}
