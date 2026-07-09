//! Diff structurel entre deux `SystemSnapshot` (7.4) — compare l'état post-désactivation à
//! l'état pré-activation (PLAN.md §4 "killswitch"), retourne le `TeardownReport` déjà défini
//! dans `commands/types.rs` (réutilisé tel quel, cf. brief EPIC 7).

use crate::shared::TeardownReport;

use super::snapshot::SystemSnapshot;

/// Aucune activation préalable connue (`pre_activation_snapshot` absent) : comparer le
/// snapshot post à lui-même donnerait toujours `clean: true`, ce qui est trompeur — retourne
/// explicitement un rapport non significatif plutôt qu'un faux "propre".
pub fn no_prior_activation() -> TeardownReport {
    TeardownReport {
        clean: false,
        divergences: vec![
            "aucune activation préalable enregistrée, vérification non significative".to_string(),
        ],
        checked_at: checked_at_string(),
    }
}

pub fn diff(pre: &SystemSnapshot, post: &SystemSnapshot) -> TeardownReport {
    let mut divergences = Vec::new();

    if post.nftables_chain_present != pre.nftables_chain_present {
        divergences.push(
            "la chaîne VITRAIL_REDIRECT est toujours présente après désactivation".to_string(),
        );
    }

    divergences.extend(subsystem_divergences(pre, post));

    let clean = divergences.is_empty();
    TeardownReport {
        clean,
        divergences,
        checked_at: checked_at_string(),
    }
}

fn subsystem_divergences(pre: &SystemSnapshot, post: &SystemSnapshot) -> Vec<String> {
    post.subsystems
        .iter()
        .filter_map(|post_sub| {
            let was_active_before = pre
                .subsystems
                .iter()
                .find(|s| s.name == post_sub.name)
                .map(|s| s.active)
                .unwrap_or(false);
            (post_sub.active != was_active_before).then(|| {
                format!(
                    "le sous-système {} est toujours actif après désactivation",
                    post_sub.name
                )
            })
        })
        .collect()
}

fn checked_at_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}
