//! Fichier de clés `$XDG_DATA_HOME/vitrail/tls_keylog.log` (story 3.1) — créé/tronqué en 600 à
//! chaque `start()`, jamais purgé à `stop()` (les clés ne réapparaissent qu'au prochain
//! démarrage, cohérent avec la discipline de réversibilité déjà posée pour les autres
//! domaines). Permissions posées atomiquement à la création (`OpenOptions::mode`), jamais de
//! `set_permissions` a posteriori (TOCTOU déjà corrigé ailleurs dans ce projet, cf.
//! `storage::connection::precreate_with_restricted_permissions`).

use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use super::vitrail_data_dir;

const KEYFILE_NAME: &str = "tls_keylog.log";

pub fn keylog_path() -> PathBuf {
    vitrail_data_dir().join(KEYFILE_NAME)
}

/// Crée (ou tronque si déjà présente) le fichier de clés en 600 — appelé à chaque
/// `KeylogSubsystem::start()` (jamais à `stop()`, story 3.1).
pub fn truncate_keyfile() -> io::Result<PathBuf> {
    let path = keylog_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    use crate::shared::ENV_GUARD;

    fn isolated_env(tag: &str) -> PathBuf {
        let base =
            std::env::temp_dir().join(format!("vitrail-keyfile-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("XDG_DATA_HOME", &base);
        base
    }

    #[test]
    fn truncate_keyfile_cree_en_600_et_vide_le_contenu_precedent() {
        let _guard = ENV_GUARD.lock().unwrap();
        let base = isolated_env("truncate");

        let path = truncate_keyfile().expect("première création");
        std::fs::write(&path, b"ancienne-cle-de-session").expect("écriture de test");

        let path2 = truncate_keyfile().expect("second appel doit tronquer, pas échouer");
        assert_eq!(path, path2);
        assert_eq!(std::fs::read(&path).expect("relecture"), b"");

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "le fichier doit être 600 dès sa création");

        let _ = std::fs::remove_dir_all(&base);
        std::env::remove_var("XDG_DATA_HOME");
    }
}
