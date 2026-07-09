//! Détection honnête de `tshark` (story 3.1/3.5) — Vitrail ne gère pas l'élévation de
//! `tshark` (divergence de privilège assumée, PLAN.md §6octies) : `tshark --version` puis test
//! réel de permission de capture (`tshark -D`, liste les interfaces sans capturer). Absent ou
//! sans permission = état dégradé explicite, jamais une supposition.
//!
//! `detect_tshark()` invoque le vrai binaire — n'est appelée qu'en production, via
//! `SystemTsharkBackend::detect()` (`tshark_process.rs`), jamais depuis un test. Toute la
//! logique testable vit dans `interpret_dash_d`/`capturable_interfaces`, qui ne spawnent
//! jamais de process (hard rule : `tshark` est absent de cette machine de dev).

use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsharkAvailability {
    pub installed: bool,
    pub can_capture: bool,
    /// Identifiants d'interface utilisables tels quels avec `-i` (numéros retournés par
    /// `tshark -D`), loopback/`any` exclues.
    pub interfaces: Vec<String>,
    pub reason: Option<String>,
}

impl TsharkAvailability {
    fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            installed: false,
            can_capture: false,
            interfaces: Vec::new(),
            reason: Some(reason.into()),
        }
    }
}

pub fn detect_tshark() -> TsharkAvailability {
    let which_ok = Command::new("tshark")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if !which_ok {
        return TsharkAvailability::unavailable(
            "tshark introuvable (tshark --version a échoué ou le binaire est absent)",
        );
    }

    match Command::new("tshark").arg("-D").output() {
        Ok(out) => interpret_dash_d(
            out.status.success(),
            &String::from_utf8_lossy(&out.stdout),
            &String::from_utf8_lossy(&out.stderr),
        ),
        Err(error) => TsharkAvailability::unavailable(format!("tshark -D injouable: {error}")),
    }
}

/// Cœur testable de la détection (story 3.1/3.5) — ne spawn jamais `tshark`, prend en entrée le
/// résultat déjà obtenu par `detect_tshark()`. `success=false` ou une liste d'interfaces
/// vide/loopback-only = permission de capture absente, état dégradé explicite.
fn interpret_dash_d(success: bool, stdout: &str, stderr: &str) -> TsharkAvailability {
    if !success {
        return TsharkAvailability {
            installed: true,
            can_capture: false,
            interfaces: Vec::new(),
            reason: Some(format!("tshark -D a échoué: {}", stderr.trim())),
        };
    }

    let interfaces = capturable_interfaces(stdout);
    if interfaces.is_empty() {
        return TsharkAvailability {
            installed: true,
            can_capture: false,
            interfaces,
            reason: Some(
                "tshark -D n'a retourné aucune interface capturable \
                 (permission de capture probablement absente, cf. groupe wireshark)"
                    .to_string(),
            ),
        };
    }

    TsharkAvailability {
        installed: true,
        can_capture: true,
        interfaces,
        reason: None,
    }
}

/// Parse la sortie `tshark -D` (`"1. wlan0\n2. lo (Loopback)\n3. any\n"`) — retient le numéro
/// d'interface (utilisable directement avec `-i`), exclut la loopback et l'interface `any`
/// (capture agrégée non voulue ici, chaque interface active est déjà listée séparément).
fn capturable_interfaces(dash_d_output: &str) -> Vec<String> {
    dash_d_output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (number, name) = line.split_once('.')?;
            let name = name.trim();
            if name.is_empty() || name.contains("(Loopback)") || name.eq_ignore_ascii_case("any") {
                return None;
            }
            Some(number.trim().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_d_reussi_avec_interfaces_donne_can_capture() {
        let availability =
            interpret_dash_d(true, "1. wlan0\n2. lo (Loopback)\n3. any\n4. wg0\n", "");
        assert!(availability.installed);
        assert!(availability.can_capture);
        assert_eq!(availability.interfaces, vec!["1", "4"]);
        assert!(availability.reason.is_none());
    }

    #[test]
    fn dash_d_sans_interface_capturable_donne_degrade() {
        let availability = interpret_dash_d(true, "1. lo (Loopback)\n2. any\n", "");
        assert!(availability.installed);
        assert!(!availability.can_capture);
        assert!(availability.interfaces.is_empty());
        assert!(availability.reason.is_some());
    }

    #[test]
    fn dash_d_en_echec_donne_degrade_avec_stderr_dans_la_raison() {
        let availability = interpret_dash_d(false, "", "permission denied");
        assert!(availability.installed);
        assert!(!availability.can_capture);
        assert!(availability.reason.unwrap().contains("permission denied"));
    }

    #[test]
    fn sortie_vide_donne_degrade() {
        let availability = interpret_dash_d(true, "", "");
        assert!(!availability.can_capture);
        assert!(availability.interfaces.is_empty());
    }
}
