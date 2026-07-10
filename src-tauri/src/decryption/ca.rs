//! CA locale dédiée Vitrail (story 4.1) — `rcgen` (pur Rust, pas de dépendance OpenSSL
//! système), générée dans `$XDG_DATA_HOME/vitrail/ca/` (clé privée 600), jamais réutiliser/
//! modifier une CA existante (PLAN.md §5). `CaSubsystem::start()` génère+installe SEULEMENT
//! si absente (idempotent) ; `stop()` NE désinstalle PAS par défaut — décision non tranchée
//! explicitement dans PLAN.md 4.1 ("documente ce choix... à confirmer avec Chris") : une CA
//! retirée à chaque coupure obligerait à réinstaller (prompt polkit) à chaque réactivation, et
//! casserait la confiance déjà accordée par l'utilisateur entre deux sessions courtes — le
//! kill switch régule la REDIRECTION (nftables/PolarProxy), pas la présence de la CA en soi.
//!
//! DIVERGENCE RÉSOLUE (2026-07-10, vérifié contre le vrai binaire PolarProxy 2.0.1 — la CLI
//! réelle enfin disponible sur cette machine) : `--cacert load:FICHIER:MOTDEPASSE` exige un
//! PKCS12, pas le PEM `rcgen` brut (reproduit manuellement : `PolarProxy --cacert load <pem>`
//! échoue immédiatement avec "Argument Error: Invalid --cacert argument"). `export_pkcs12`
//! shelle vers le binaire `openssl` (même pattern que `sha256sum`/`trust`/`nft` — un binaire
//! externe à surface étroite plutôt qu'une dépendance crate `openssl` qui contredirait la
//! raison même du choix de `rcgen`) pour convertir cert+clé PEM en PKCS12 à la demande, avec
//! un mot de passe aléatoire À USAGE UNIQUE (jamais persisté : régénéré à chaque lancement de
//! PolarProxy par `PolarProxySubsystem::start()`, le fichier `.p12` n'a besoin de survivre que
//! le temps du process PolarProxy en cours).

use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use rcgen::{CertificateParams, DnType, KeyPair};
use sha2::{Digest, Sha256};

use crate::killswitch::{KillSwitchError, Subsystem};
use crate::storage::decryption::{self as storage_decryption, CaMetadata};
use crate::storage::StorageHandle;

use super::helper_backend::HelperBackend;
use super::vitrail_data_dir;

const CERT_FILE: &str = "ca.pem";
const KEY_FILE: &str = "ca.key";
const PKCS12_FILE: &str = "ca.p12";

pub fn ca_dir() -> PathBuf {
    vitrail_data_dir().join("ca")
}

/// Génère une nouvelle CA dédiée (jamais un réemploi) — cert PEM + clé privée PEM 600.
fn generate_ca() -> Result<(PathBuf, PathBuf, String), String> {
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, "Vitrail Local Root CA");
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);

    let key_pair =
        KeyPair::generate().map_err(|error| format!("génération de clé échouée: {error}"))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|error| format!("auto-signature de la CA échouée: {error}"))?;

    let dir = ca_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("création de {dir:?} échouée: {error}"))?;
    let cert_path = dir.join(CERT_FILE);
    let key_path = dir.join(KEY_FILE);

    std::fs::write(&cert_path, cert.pem())
        .map_err(|error| format!("écriture du certificat CA échouée: {error}"))?;
    write_private_key(&key_path, &key_pair.serialize_pem())?;

    let fingerprint = fingerprint_der(&cert.der()[..]);
    Ok((cert_path, key_path, fingerprint))
}

fn write_private_key(path: &PathBuf, pem: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("ouverture de {path:?} échouée: {error}"))?;
    file.write_all(pem.as_bytes())
        .map_err(|error| format!("écriture de {path:?} échouée: {error}"))
}

fn fingerprint_der(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Convertit la CA PEM courante en PKCS12 pour PolarProxy (`--cacert load:FICHIER:MOTDEPASSE`,
/// cf. doc de module). Mot de passe aléatoire à usage unique — jamais persisté nulle part,
/// jamais réutilisé d'un lancement à l'autre. Écrase `ca.p12` s'il existe déjà (résidu d'un
/// lancement précédent de PolarProxy, jamais nettoyé automatiquement).
pub fn export_pkcs12(cert_path: &str, key_path: &str) -> Result<(PathBuf, String), String> {
    let password = random_password()?;
    let dest = ca_dir().join(PKCS12_FILE);

    // Pré-créer le fichier avec 600 AVANT que `openssl` n'écrive dedans (même discipline
    // TOCTOU-safe que `write_private_key`) : `openssl -out` sur un fichier déjà existant
    // réutilise l'inode et ses permissions, ne les réinitialise jamais à la création.
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&dest)
        .map_err(|error| format!("pré-création de {dest:?} échouée: {error}"))?;

    let output = Command::new("openssl")
        .args(["pkcs12", "-export", "-in", cert_path, "-inkey", key_path])
        .arg("-out")
        .arg(&dest)
        .arg("-passout")
        .arg(format!("pass:{password}"))
        .output()
        .map_err(|error| format!("échec d'exécution d'`openssl pkcs12`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`openssl pkcs12 -export` a échoué: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok((dest, password))
}

/// Mot de passe hex 32 caractères (16 octets d'entropie) — lu directement depuis `/dev/urandom`
/// avec `read_exact` sur un buffer fixe (jamais `fs::read`, qui bouclerait indéfiniment sur un
/// périphérique caractère sans fin de fichier).
fn random_password() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|error| format!("lecture de /dev/urandom échouée: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub struct CaSubsystem {
    active: AtomicBool,
    storage: StorageHandle,
    helper: Box<dyn HelperBackend>,
}

impl CaSubsystem {
    pub fn new(storage: StorageHandle, helper: Box<dyn HelperBackend>) -> Self {
        Self {
            active: AtomicBool::new(false),
            storage,
            helper,
        }
    }

    fn exec_error(reason: impl ToString) -> KillSwitchError {
        KillSwitchError::SubsystemExec {
            subsystem: "ca".to_string(),
            reason: reason.to_string(),
        }
    }
}

/// Génère+installe une CA neuve et persiste ses métadonnées — fonction libre (pas une méthode
/// de `CaSubsystem`) pour rester appelable depuis `commands/settings.rs::rotate_ca` SANS accès
/// à l'instance privée détenue par `KillSwitchState` (frontière IPC : `commands/` délègue à
/// `decryption::`, jamais aux internes de `killswitch::`). `CaSubsystem::start()` appelle
/// EXACTEMENT cette même fonction (pas de logique dupliquée).
pub fn generate_and_install(
    storage: &StorageHandle,
    helper: &dyn HelperBackend,
) -> Result<CaMetadata, String> {
    let (cert_path, key_path, fingerprint) = generate_ca()?;
    helper.install_ca(&cert_path.to_string_lossy())?;

    let metadata = CaMetadata {
        cert_path: cert_path.to_string_lossy().to_string(),
        key_path: key_path.to_string_lossy().to_string(),
        fingerprint_sha256: fingerprint,
    };
    storage_decryption::save_ca(storage, &metadata).map_err(|error| {
        tracing::error!(error = %error, "persistance des métadonnées CA échouée");
        error.to_string()
    })?;
    Ok(metadata)
}

/// Régénère et réinstalle une CA totalement neuve (retire l'ancienne par empreinte exacte PUIS
/// génère/installe — `commands/settings.rs::rotate_ca`, story 4.1).
pub fn rotate_ca(
    storage: &StorageHandle,
    helper: &dyn HelperBackend,
) -> Result<CaMetadata, String> {
    if let Ok(Some(existing)) = storage_decryption::get_ca(storage) {
        if let Err(error) = helper.remove_ca(&existing.fingerprint_sha256) {
            tracing::warn!(error = %error, "rotate_ca: retrait de l'ancienne CA échoué, on continue");
        }
    }
    generate_and_install(storage, helper)
}

impl Subsystem for CaSubsystem {
    fn name(&self) -> &'static str {
        "ca"
    }

    /// Idempotent : génère+installe SEULEMENT si aucune CA n'est déjà connue en storage ET que
    /// le fichier cert existe encore réellement sur disque (résidu partiel = régénération).
    fn start(&self) -> Result<(), KillSwitchError> {
        let existing = storage_decryption::get_ca(&self.storage).ok().flatten();
        let needs_generation = match &existing {
            Some(meta) => !std::path::Path::new(&meta.cert_path).exists(),
            None => true,
        };

        if needs_generation {
            generate_and_install(&self.storage, self.helper.as_ref()).map_err(Self::exec_error)?;
            tracing::info!("decryption: CA locale générée et installée");
        } else {
            tracing::info!("decryption: CA locale déjà présente, réutilisée (idempotent)");
        }

        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Ne désinstalle PAS la CA par défaut (cf. doc de module) — seul `is_active` reflète que
    /// le sous-système n'est plus "en service" pour ce cycle, la CA reste dans le trust store.
    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        tracing::info!("decryption: CA locale conservée dans le trust store (choix documenté)");
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

/// Variante testable — jamais de vrai `vitrail-helper`/`pkexec` déclenché.
#[cfg(test)]
pub struct FakeCaSubsystem {
    active: AtomicBool,
}

#[cfg(test)]
impl FakeCaSubsystem {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
        }
    }
}

#[cfg(test)]
impl Default for FakeCaSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Subsystem for FakeCaSubsystem {
    fn name(&self) -> &'static str {
        "ca"
    }

    fn start(&self) -> Result<(), KillSwitchError> {
        self.active.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&self) -> Result<(), KillSwitchError> {
        self.active.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decryption::helper_backend::FakeHelperBackend;
    use crate::shared::ENV_GUARD;

    fn isolated_env(tag: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("vitrail-ca-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("XDG_DATA_HOME", &base);
        base
    }

    fn cleanup(base: &PathBuf) {
        let _ = std::fs::remove_dir_all(base);
        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn start_genere_une_ca_et_persiste_ses_metadonnees() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("generate");
        let storage = StorageHandle::open_in_memory().unwrap();
        let helper = FakeHelperBackend::new();
        let subsystem = CaSubsystem::new(storage.clone(), Box::new(helper.clone()));

        subsystem.start().expect("génération CA doit réussir");
        assert!(subsystem.is_active());

        let metadata = storage_decryption::get_ca(&storage)
            .unwrap()
            .expect("CA persistée");
        assert_eq!(metadata.fingerprint_sha256.len(), 64);
        assert!(std::path::Path::new(&metadata.cert_path).exists());
        assert_eq!(helper.install_calls(), 1);

        cleanup(&base);
    }

    #[test]
    fn start_est_idempotent_si_ca_deja_presente() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("idempotent");
        let storage = StorageHandle::open_in_memory().unwrap();
        let helper = FakeHelperBackend::new();
        let subsystem = CaSubsystem::new(storage, Box::new(helper.clone()));

        subsystem.start().unwrap();
        subsystem.stop().unwrap();
        subsystem.start().unwrap();

        assert_eq!(
            helper.install_calls(),
            1,
            "une deuxième activation ne doit jamais régénérer/réinstaller la CA existante"
        );

        cleanup(&base);
    }

    #[test]
    fn stop_ne_desinstalle_jamais_la_ca() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("stop-keeps-ca");
        let storage = StorageHandle::open_in_memory().unwrap();
        let helper = FakeHelperBackend::new();
        let subsystem = CaSubsystem::new(storage, Box::new(helper.clone()));

        subsystem.start().unwrap();
        subsystem.stop().unwrap();

        assert_eq!(
            helper.remove_calls(),
            0,
            "stop() ne doit jamais retirer la CA"
        );

        cleanup(&base);
    }
}
