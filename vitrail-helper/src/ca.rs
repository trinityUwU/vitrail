//! `install-ca`/`remove-ca` (PLAN.md §6nonies 4.1) — détecte `trust` (p11-kit, Arch/Fedora) en
//! priorité, sinon `update-ca-certificates` (Debian/Ubuntu). Le fichier installé est NOMMÉ par
//! l'empreinte SHA-256 exacte (`vitrail-<fingerprint>.pem`) : `remove-ca` retrouve et supprime
//! UNIQUEMENT ce fichier, jamais un retrait générique par nom/chemin (invariant PLAN.md §5).
//! `sha256sum` (coreutils) est invoqué en externe plutôt que d'ajouter une dépendance crate à
//! ce binaire à surface volontairement étroite (même choix que `nft`/`systemctl`/`trust`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Code de sortie dédié : aucun mécanisme de trust store connu trouvé (ni `trust` ni
/// `update-ca-certificates`) — état dégradé explicite, distinct d'un échec générique.
pub const EXIT_NO_TRUST_MECHANISM: u8 = 3;

const TRUST_ANCHOR_DIR: &str = "/etc/ca-certificates/trust-source/anchors";
const DEBIAN_ANCHOR_DIR: &str = "/usr/local/share/ca-certificates";

pub enum InstallError {
    Failed(String),
    NoTrustMechanism(String),
}

pub fn install_ca(cert_path: &str) -> Result<(), InstallError> {
    let fingerprint = sha256_file(cert_path).map_err(InstallError::Failed)?;

    if which("trust") {
        install_via_trust(cert_path, &fingerprint).map_err(InstallError::Failed)
    } else if which("update-ca-certificates") {
        install_via_debian(cert_path, &fingerprint).map_err(InstallError::Failed)
    } else {
        Err(InstallError::NoTrustMechanism(
            "aucun mécanisme de trust store trouvé (ni `trust` ni `update-ca-certificates`)"
                .to_string(),
        ))
    }
}

pub fn remove_ca(fingerprint: &str) -> Result<(), String> {
    let trust_path = anchor_path(TRUST_ANCHOR_DIR, fingerprint);
    let debian_path = anchor_path(DEBIAN_ANCHOR_DIR, fingerprint);

    let mut removed = false;
    if trust_path.exists() {
        remove_verified(&trust_path, fingerprint)?;
        if which("trust") {
            run(Command::new("trust")
                .args(["anchor", "--remove"])
                .arg(&trust_path))?;
        }
        removed = true;
    }
    if debian_path.exists() {
        remove_verified(&debian_path, fingerprint)?;
        if which("update-ca-certificates") {
            run(&mut Command::new("update-ca-certificates"))?;
        }
        removed = true;
    }

    if !removed {
        eprintln!("vitrail-helper: aucune CA installée ne correspond à cette empreinte (no-op)");
    }
    Ok(())
}

/// Vérifie que le fichier trouvé au chemin dérivé de l'empreinte a bien CETTE empreinte exacte
/// avant suppression (défense en profondeur contre une collision de nommage) — jamais une
/// suppression basée sur le seul nom de fichier.
fn remove_verified(path: &Path, expected_fingerprint: &str) -> Result<(), String> {
    let actual = sha256_file(&path.to_string_lossy())?;
    if actual != expected_fingerprint {
        return Err(format!(
            "empreinte du fichier {} ({actual}) ne correspond pas à celle demandée \
             ({expected_fingerprint}), suppression refusée",
            path.display()
        ));
    }
    std::fs::remove_file(path)
        .map_err(|error| format!("suppression de {} échouée: {error}", path.display()))
}

fn anchor_path(dir: &str, fingerprint: &str) -> PathBuf {
    PathBuf::from(dir).join(format!("vitrail-{fingerprint}.pem"))
}

fn install_via_trust(cert_path: &str, fingerprint: &str) -> Result<(), String> {
    let dest = anchor_path(TRUST_ANCHOR_DIR, fingerprint);
    std::fs::copy(cert_path, &dest)
        .map_err(|error| format!("copie vers {} échouée: {error}", dest.display()))?;
    run(Command::new("trust").args(["anchor", "--store"]).arg(&dest))
}

fn install_via_debian(cert_path: &str, fingerprint: &str) -> Result<(), String> {
    let dest = anchor_path(DEBIAN_ANCHOR_DIR, fingerprint).with_extension("crt");
    std::fs::copy(cert_path, &dest)
        .map_err(|error| format!("copie vers {} échouée: {error}", dest.display()))?;
    run(&mut Command::new("update-ca-certificates"))
}

fn sha256_file(path: &str) -> Result<String, String> {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .map_err(|error| format!("échec d'exécution de `sha256sum`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`sha256sum {path}` a échoué: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .next()
        .map(|hash| hash.to_lowercase())
        .ok_or_else(|| "sortie de sha256sum vide ou inattendue".to_string())
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("échec d'exécution de la commande: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "commande échouée (code {}): {}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).trim()
    ))
}
