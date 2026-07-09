//! Extraction du SNI depuis un ClientHello TLS en clair (`tls-parser`) — aucun déchiffrement,
//! lecture d'un champ non chiffré du handshake (PLAN.md §6quater, ARCHITECTURE.md `capture/`).

use tls_parser::{
    parse_tls_extensions, parse_tls_plaintext, TlsExtension, TlsMessage, TlsMessageHandshake,
};

use crate::packet::FiveTuple;

pub fn extract_sni(tuple: &FiveTuple) -> Option<String> {
    if tuple.protocol != "tcp" || tuple.payload.is_empty() {
        return None;
    }
    let (_, plaintext) = parse_tls_plaintext(&tuple.payload).ok()?;
    plaintext.msg.iter().find_map(sni_from_message)
}

fn sni_from_message(message: &TlsMessage) -> Option<String> {
    let TlsMessage::Handshake(TlsMessageHandshake::ClientHello(client_hello)) = message else {
        return None;
    };
    let extensions_data = client_hello.ext?;
    let (_, extensions) = parse_tls_extensions(extensions_data).ok()?;
    extensions.into_iter().find_map(sni_from_extension)
}

fn sni_from_extension(extension: TlsExtension) -> Option<String> {
    let TlsExtension::SNI(sni_list) = extension else {
        return None;
    };
    let (_, name) = sni_list.into_iter().next()?;
    std::str::from_utf8(name).ok().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construit un enregistrement TLS `ClientHello` minimal avec extension SNI pour le nom
    /// de domaine donné — bytes bruts assemblés à la main pour tester l'extraction sans
    /// dépendre d'une capture réelle.
    fn client_hello_with_sni(hostname: &str) -> Vec<u8> {
        let server_name = hostname.as_bytes();

        // server_name_list: name_type(1) + name_length(2) + name
        let mut server_name_list = vec![0x00];
        server_name_list.extend((server_name.len() as u16).to_be_bytes());
        server_name_list.extend(server_name);

        // extension SNI: type(2)=0x0000 + length(2) + server_name_list_length(2) + list
        let mut sni_extension = vec![0x00, 0x00];
        let sni_ext_body_len = 2 + server_name_list.len();
        sni_extension.extend((sni_ext_body_len as u16).to_be_bytes());
        sni_extension.extend((server_name_list.len() as u16).to_be_bytes());
        sni_extension.extend(server_name_list);

        let extensions = sni_extension;

        let mut body = Vec::new();
        body.extend([0x03, 0x03]); // client_version TLS 1.2
        body.extend([0x00; 32]); // random
        body.push(0x00); // session_id_length = 0
        body.extend([0x00, 0x02]); // cipher_suites_length
        body.extend([0x00, 0x2F]); // un cipher suite
        body.push(0x01); // compression_methods_length
        body.push(0x00); // compression method null
        body.extend((extensions.len() as u16).to_be_bytes());
        body.extend(extensions);

        let mut handshake = vec![0x01]; // ClientHello
        let body_len = body.len() as u32;
        handshake.extend(&body_len.to_be_bytes()[1..]); // 3 octets de longueur
        handshake.extend(body);

        let mut record = vec![0x16, 0x03, 0x01]; // handshake, TLS 1.0 record version
        record.extend((handshake.len() as u16).to_be_bytes());
        record.extend(handshake);
        record
    }

    fn tuple_with_payload(payload: Vec<u8>) -> FiveTuple {
        FiveTuple {
            protocol: "tcp",
            src_ip: "192.168.1.10".to_string(),
            dst_ip: "192.168.1.20".to_string(),
            src_port: Some(51000),
            dst_port: Some(443),
            payload,
        }
    }

    #[test]
    fn extracts_sni_from_valid_client_hello() {
        let payload = client_hello_with_sni("example.com");
        let tuple = tuple_with_payload(payload);
        assert_eq!(extract_sni(&tuple), Some("example.com".to_string()));
    }

    #[test]
    fn empty_payload_returns_none_without_panicking() {
        let tuple = tuple_with_payload(Vec::new());
        assert_eq!(extract_sni(&tuple), None);
    }

    #[test]
    fn non_tcp_protocol_returns_none() {
        let mut tuple = tuple_with_payload(client_hello_with_sni("example.com"));
        tuple.protocol = "udp";
        assert_eq!(extract_sni(&tuple), None);
    }

    #[test]
    fn truncated_client_hello_returns_none_without_panicking() {
        let full = client_hello_with_sni("example.com");
        for cut in [1, 5, 10, full.len() / 2] {
            let tuple = tuple_with_payload(full[..cut].to_vec());
            assert_eq!(
                extract_sni(&tuple),
                None,
                "troncature à {cut} octets ne doit pas paniquer"
            );
        }
    }

    #[test]
    fn random_garbage_bytes_return_none_without_panicking() {
        let garbage_samples: [&[u8]; 4] = [
            &[0xFF, 0x00, 0x00, 0x00, 0x00],
            &[0x16, 0x03, 0x01, 0xFF, 0xFF, 0x01, 0x02, 0x03],
            &[0x00; 10],
            &[0x16, 0x03, 0x01, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00],
        ];
        for garbage in garbage_samples {
            let tuple = tuple_with_payload(garbage.to_vec());
            assert_eq!(extract_sni(&tuple), None);
        }
    }
}
