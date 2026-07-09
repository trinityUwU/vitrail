//! Connexion SQLite unique, protégée par `Mutex`, mode WAL — chemin
//! `$XDG_DATA_HOME/vitrail/vitrail.db` (créé 600). `StorageHandle` est `Clone` (Arc interne) :
//! une seule connexion réelle partagée entre `KillSwitchState` et l'état `tauri::State` dédié
//! (voir `lib.rs`), jamais deux connexions séparées vers le même fichier (PLAN.md §6sexies).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::error::StorageError;
use super::migrations;

#[derive(Clone)]
pub struct StorageHandle {
    conn: Arc<Mutex<Connection>>,
}

impl StorageHandle {
    /// Connexion réelle sur le fichier XDG par défaut — utilisée par l'app (`lib.rs`).
    pub fn open_default() -> Result<Self, StorageError> {
        let path = default_db_path();
        ensure_parent_dir(&path)?;
        precreate_with_restricted_permissions(&path)?;
        let conn = Connection::open(&path)?;
        Self::initialize(conn)
    }

    /// Connexion en mémoire — utilisée par les tests des domaines appelants (killswitch,
    /// capture, attribution) pour ne jamais toucher au vrai fichier `vitrail.db`.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        Self::initialize(conn)
    }

    fn initialize(conn: Connection) -> Result<Self, StorageError> {
        // Ignoré silencieusement par SQLite sur `:memory:` (retombe sur le mode "memory") —
        // comportement documenté, pas une erreur à traiter côté appelant.
        let _: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        migrations::apply_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Accès à la connexion réservé aux modules de `storage/` (`pub(super)` = visible dans
    /// `storage` et ses descendants) — jamais exposé en dehors du domaine (frontière stricte).
    pub(super) fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("mutex storage empoisonné")
    }
}

pub(super) fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn default_db_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local/share")
        });
    base.join("vitrail").join("vitrail.db")
}

fn ensure_parent_dir(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|error| {
        tracing::error!(error = %error, path = %parent.display(), "création du dossier storage échouée");
        StorageError::Io(error)
    })
}

/// Pré-crée `vitrail.db` en mode 600 dès la création (jamais un `set_permissions` a posteriori,
/// qui laisserait une fenêtre TOCTOU où le fichier existe avec l'umask par défaut du process —
/// même raisonnement que le fix JSONL de `killswitch/snapshot.rs`, git show 61a7f05).
/// Si le fichier existe déjà (lancement précédent), `create_new` échoue normalement : on saute
/// la précréation, ses permissions étant déjà correctes depuis sa première création.
fn precreate_with_restricted_permissions(path: &Path) -> Result<(), StorageError> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => {
            tracing::error!(error = %error, path = %path.display(), "précréation de vitrail.db échouée");
            Err(StorageError::Io(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn unique_test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vitrail-test-{name}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ))
    }

    #[test]
    fn precreate_donne_des_permissions_600_des_la_creation() {
        let path = unique_test_path("perms");
        let _ = std::fs::remove_file(&path);

        precreate_with_restricted_permissions(&path).expect("précréation");
        let mode = std::fs::metadata(&path)
            .expect("métadonnées du fichier précréé")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            mode, 0o600,
            "le fichier doit être 600 dès sa création, jamais plus large"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn precreate_est_un_no_op_si_le_fichier_existe_deja() {
        let path = unique_test_path("existing");
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, b"deja-la").expect("création préalable");

        let result = precreate_with_restricted_permissions(&path);
        assert!(
            result.is_ok(),
            "un fichier déjà existant ne doit pas faire échouer la précréation"
        );
        assert_eq!(
            std::fs::read(&path).expect("relecture"),
            b"deja-la",
            "le contenu existant ne doit pas être écrasé"
        );

        let _ = std::fs::remove_file(&path);
    }
}
