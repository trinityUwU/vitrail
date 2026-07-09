//! Validateurs stricts partagés par les sous-commandes de `main.rs` — jamais d'action
//! privilégiée sur une entrée non validée (EPIC 4, même discipline que
//! `main.rs::validate_socket_address` existant, EPIC 1).

/// Chemin de fichier absolu, sans `..`, caractères restreints — utilisé pour les chemins de
/// certificat passés à `install-ca`/`remove-ca`.
pub fn validate_file_path(path: &str) -> Result<(), String> {
    let invalid = || format!("chemin de fichier invalide, refusé: {path}");
    if !path.starts_with('/') || path.contains("..") || path.is_empty() || path.len() > 4096 {
        return Err(invalid());
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.');
    if !path.chars().all(allowed) {
        return Err(invalid());
    }
    Ok(())
}

/// Empreinte SHA-256 exacte (64 caractères hexadécimaux) — jamais de retrait de CA par nom/
/// chemin générique (PLAN.md §6nonies 4.1).
pub fn validate_fingerprint(fingerprint: &str) -> Result<(), String> {
    if fingerprint.len() != 64 || !fingerprint.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "empreinte SHA-256 invalide, refusée (attendu 64 hex): {fingerprint}"
        ));
    }
    Ok(())
}

/// Port local non privilégié (`> 1024`) — la redirection nftables ne doit jamais cibler un
/// port système (PLAN.md §6nonies 4.3).
pub fn validate_local_port(port: &str) -> Result<u16, String> {
    let parsed: u16 = port
        .parse()
        .map_err(|_| format!("port invalide, refusé: {port}"))?;
    if parsed <= 1024 {
        return Err(format!("port privilégié refusé (attendu > 1024): {parsed}"));
    }
    Ok(parsed)
}

/// Liste d'IPs (v4/v6) séparées par des virgules — chaque élément doit parser comme une
/// `IpAddr` exacte, jamais un motif partiel (PLAN.md §6nonies 4.5).
pub fn validate_ip_list(raw: &str) -> Result<Vec<std::net::IpAddr>, String> {
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    raw.split(',')
        .map(|item| {
            item.trim()
                .parse::<std::net::IpAddr>()
                .map_err(|_| format!("adresse IP invalide dans la liste d'exclusions: {item}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_file_path_rejette_traversee() {
        assert!(validate_file_path("/etc/vitrail/../shadow").is_err());
        assert!(validate_file_path("relative/path").is_err());
        assert!(validate_file_path("/etc/vitrail/ca/ca.pem").is_ok());
    }

    #[test]
    fn validate_fingerprint_exige_64_hex() {
        assert!(validate_fingerprint("abc").is_err());
        assert!(validate_fingerprint(&"a".repeat(64)).is_ok());
        assert!(validate_fingerprint(&"z".repeat(64)).is_err());
    }

    #[test]
    fn validate_local_port_refuse_privilegie() {
        assert!(validate_local_port("443").is_err());
        assert!(validate_local_port("1024").is_err());
        assert!(validate_local_port("8443").is_ok());
        assert!(validate_local_port("abc").is_err());
    }

    #[test]
    fn validate_ip_list_rejette_entree_non_ip() {
        assert!(validate_ip_list("1.2.3.4,not-an-ip").is_err());
        assert_eq!(validate_ip_list("").unwrap().len(), 0);
        assert_eq!(validate_ip_list("1.2.3.4,::1").unwrap().len(), 2);
    }
}
