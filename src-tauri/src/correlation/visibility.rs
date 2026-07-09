//! Détermination du niveau de visibilité d'un flux (story 5.3, PLAN.md §6septies) — fonction
//! pure, testée pour toutes les combinaisons de sources disponibles. `decryption`/`keylog`
//! sont toujours `false` tant qu'EPIC 3/4 ne sont pas livrés, mais le paramètre existe déjà
//! pour ne jamais avoir à retoucher cette fonction (seulement ses appelants) quand ils le
//! seront.

use crate::shared::FlowVisibility;

/// Mapping exact PLAN.md §6septies 5.3 :
/// - contenu déchiffré présent (`decryption` OU `keylog`) → `Fully`.
/// - `capture` présente, pas de contenu → `Meta`.
/// - `attribution` présente, `capture` absente → `Attrib`.
/// - rien → `Unknown`.
pub fn determine_visibility(
    capture: bool,
    attribution: bool,
    decryption: bool,
    keylog: bool,
) -> FlowVisibility {
    if decryption || keylog {
        FlowVisibility::Fully
    } else if capture {
        FlowVisibility::Meta
    } else if attribution {
        FlowVisibility::Attrib
    } else {
        FlowVisibility::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exhaustif sur les 16 combinaisons des 4 booléens — table de vérité écrite en dur
    /// (jamais dérivée de la même logique if/else que `determine_visibility`, sinon un bug
    /// partagé entre le code et le test ne serait jamais détecté).
    #[test]
    fn couvre_toutes_les_combinaisons_de_sources() {
        let cases: [(bool, bool, bool, bool, FlowVisibility); 16] = [
            (false, false, false, false, FlowVisibility::Unknown),
            (false, false, false, true, FlowVisibility::Fully),
            (false, false, true, false, FlowVisibility::Fully),
            (false, false, true, true, FlowVisibility::Fully),
            (false, true, false, false, FlowVisibility::Attrib),
            (false, true, false, true, FlowVisibility::Fully),
            (false, true, true, false, FlowVisibility::Fully),
            (false, true, true, true, FlowVisibility::Fully),
            (true, false, false, false, FlowVisibility::Meta),
            (true, false, false, true, FlowVisibility::Fully),
            (true, false, true, false, FlowVisibility::Fully),
            (true, false, true, true, FlowVisibility::Fully),
            (true, true, false, false, FlowVisibility::Meta),
            (true, true, false, true, FlowVisibility::Fully),
            (true, true, true, false, FlowVisibility::Fully),
            (true, true, true, true, FlowVisibility::Fully),
        ];

        for (capture, attribution, decryption, keylog, expected) in cases {
            let visibility = determine_visibility(capture, attribution, decryption, keylog);
            assert_eq!(
                visibility, expected,
                "capture={capture} attribution={attribution} decryption={decryption} keylog={keylog}"
            );
        }
    }

    #[test]
    fn capture_seule_donne_meta() {
        assert_eq!(
            determine_visibility(true, false, false, false),
            FlowVisibility::Meta
        );
    }

    #[test]
    fn attribution_seule_donne_attrib() {
        assert_eq!(
            determine_visibility(false, true, false, false),
            FlowVisibility::Attrib
        );
    }

    #[test]
    fn rien_donne_unknown() {
        assert_eq!(
            determine_visibility(false, false, false, false),
            FlowVisibility::Unknown
        );
    }

    #[test]
    fn decryption_prime_sur_tout_le_reste() {
        assert_eq!(
            determine_visibility(true, true, true, false),
            FlowVisibility::Fully
        );
    }

    #[test]
    fn keylog_seul_donne_fully_meme_sans_capture_ni_attribution() {
        assert_eq!(
            determine_visibility(false, false, false, true),
            FlowVisibility::Fully
        );
    }
}
