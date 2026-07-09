//! Exclusions utilisateur (story 4.5) — périmètre réaliste : seules les exclusions
//! `kind == "destination"` sont appliquées en amont nftables (résolution DNS locale du domaine
//! en IP(s) via `std::net::ToSocketAddrs`, standard, pas de dépendance DNS supplémentaire).
//! `kind == "processus"` est persisté (cohérence du contrat IPC/liste affichée) mais JAMAIS
//! appliqué au niveau nftables — limite documentée, jamais un faux sentiment de protection.

use std::net::{IpAddr, ToSocketAddrs};

use crate::storage::decryption::{self as storage_decryption, ExclusionRow};
use crate::storage::StorageHandle;

use super::helper_backend::HelperBackend;

pub const KIND_DESTINATION: &str = "destination";
/// Valeur `kind` documentée pour la persistance-sans-application (cf. doc de module) — utilisée
/// par les tests ; conservée `pub` comme pendant explicite de `KIND_DESTINATION` dans le
/// contrat de ce module même si le chemin `else` de `add_exclusion` ne la compare pas par
/// valeur (tout ce qui n'est pas `KIND_DESTINATION` est traité comme non-nftables).
#[allow(dead_code)]
pub const KIND_PROCESS: &str = "processus";

pub fn add_exclusion(
    storage: &StorageHandle,
    helper: &dyn HelperBackend,
    name: &str,
    kind: &str,
) -> Result<(), String> {
    storage_decryption::add_exclusion(storage, name, kind)
        .map_err(|error| format!("persistance de l'exclusion échouée: {error}"))?;

    if kind == KIND_DESTINATION {
        apply_destination_exclusions(storage, helper)?;
    } else {
        tracing::warn!(
            name,
            "exclusion de type processus persistée mais NON appliquée au niveau nftables \
             (limite connue, aucun moyen fiable de filtrer par processus à ce niveau réseau)"
        );
    }
    Ok(())
}

pub fn remove_exclusion(
    storage: &StorageHandle,
    helper: &dyn HelperBackend,
    name: &str,
) -> Result<(), String> {
    let was_destination = storage_decryption::list_exclusions(storage)
        .map(|rows| {
            rows.iter()
                .any(|r| r.name == name && r.kind == KIND_DESTINATION)
        })
        .unwrap_or(false);

    storage_decryption::remove_exclusion(storage, name)
        .map_err(|error| format!("suppression de l'exclusion échouée: {error}"))?;

    if was_destination {
        apply_destination_exclusions(storage, helper)?;
    }
    Ok(())
}

/// Recalcule la liste COMPLÈTE des IPs résolues pour toutes les exclusions `destination`
/// connues et la pousse en une seule fois (`nft-set-exclusions` REMPLACE tout le set côté
/// helper — jamais un ajout incrémental côté nftables).
fn apply_destination_exclusions(
    storage: &StorageHandle,
    helper: &dyn HelperBackend,
) -> Result<(), String> {
    let rows = storage_decryption::list_exclusions(storage)
        .map_err(|error| format!("lecture des exclusions échouée: {error}"))?;

    let ips: Vec<String> = rows
        .iter()
        .filter(|row| row.kind == KIND_DESTINATION)
        .flat_map(|row| resolve_domain(&row.name))
        .map(|ip| ip.to_string())
        .collect();

    helper.nft_set_exclusions(&ips)
}

/// Résolution DNS locale best-effort — une résolution échouée est loggée et ignorée (jamais
/// fatale à l'ensemble de la liste, un domaine injoignable ne doit pas bloquer les autres
/// exclusions déjà valides).
fn resolve_domain(domain: &str) -> Vec<IpAddr> {
    match (domain, 443).to_socket_addrs() {
        Ok(addrs) => addrs.map(|addr| addr.ip()).collect(),
        Err(error) => {
            tracing::warn!(domain, error = %error, "decryption: résolution DNS d'une exclusion échouée");
            Vec::new()
        }
    }
}

#[allow(dead_code)] // contrat public consommé par `commands/settings.rs` (liste affichée)
pub fn list_exclusions(storage: &StorageHandle) -> Vec<ExclusionRow> {
    storage_decryption::list_exclusions(storage).unwrap_or_else(|error| {
        tracing::error!(error = %error, "decryption: lecture des exclusions échouée");
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decryption::helper_backend::FakeHelperBackend;

    #[test]
    fn add_exclusion_processus_persiste_sans_toucher_nftables() {
        let storage = StorageHandle::open_in_memory().unwrap();
        let helper = FakeHelperBackend::new();

        add_exclusion(&storage, &helper, "firefox", KIND_PROCESS).unwrap();

        assert_eq!(list_exclusions(&storage).len(), 1);
        assert!(
            helper.redirect_calls().is_empty(),
            "une exclusion processus ne doit jamais toucher nftables"
        );
    }

    #[test]
    fn remove_exclusion_absente_est_un_no_op() {
        let storage = StorageHandle::open_in_memory().unwrap();
        let helper = FakeHelperBackend::new();
        assert!(remove_exclusion(&storage, &helper, "inconnue").is_ok());
    }
}
