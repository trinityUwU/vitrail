//! CA locale dédiée Vitrail (story 4.1) — `rcgen` (pur Rust, pas de dépendance OpenSSL
//! système), générée dans `$XDG_DATA_HOME/vitrail/ca/` (clé privée 600), jamais réutiliser/
//! modifier une CA existante (PLAN.md §5). `CaSubsystem::start()` génère+installe SEULEMENT
//! si absente (idempotent) ; `stop()` NE désinstalle PAS par défaut — décision non tranchée
//! explicitement dans PLAN.md 4.1 ("documente ce choix... à confirmer avec Chris") : une CA
//! retirée à chaque coupure obligerait à réinstaller (prompt polkit) à chaque réactivation, et
//! casserait la confiance déjà accordée par l'utilisateur entre deux sessions courtes — le
//! kill switch régule la REDIRECTION (nftables/PolarProxy), pas la présence de la CA en soi.
//!
//! DIVERGENCE SIGNALÉE (recherche CLI PolarProxy, EPIC 4) : PolarProxy ne charge une CA
//! externe que via `--cacert load <fichier PKCS12>`. `rcgen` ne produit pas de PKCS12 (aucune
//! dépendance OpenSSL disponible sans contredire la raison même de son choix ici). Le cert PEM
//! généré ci-dessous est passé tel quel à `--cacert load` par `polarproxy_process.rs` — ce
//! point précis (format exact accepté par ce flag) N'A PAS pu être vérifié contre un vrai
//! binaire PolarProxy (absent de cette machine, cf. rapport de livraison), à valider par Chris.

use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
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
