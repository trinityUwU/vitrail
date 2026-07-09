//! API publique storage pour `keylog::app_injection`/`commands/settings.rs` — table
//! `keylog_apps` (migration 0003, PLAN.md §6octies) : liste des apps ciblées par l'injection
//! `SSLKEYLOGFILE` + état d'injection courant (chemin `.desktop` réécrit, chemin de sauvegarde
//! de la surcharge préexistante si elle existait).

use rusqlite::{params, OptionalExtension};

use super::connection::StorageHandle;
use super::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeylogAppRow {
    pub binary_path: String,
    pub desktop_path: Option<String>,
    pub backup_path: Option<String>,
}

/// Liste les apps ciblées (config persistée, story 3.2/3.5) — ordre stable par `binary_path`
/// pour un affichage déterministe.
pub fn list_apps(storage: &StorageHandle) -> Result<Vec<KeylogAppRow>, StorageError> {
    let conn = storage.lock();
    let mut stmt = conn.prepare(
        "SELECT binary_path, desktop_path, backup_path FROM keylog_apps ORDER BY binary_path",
    )?;
    let rows = stmt.query_map([], row_to_app)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Ajoute une app ciblée — idempotent (`INSERT OR IGNORE`), jamais de doublon.
pub fn add_app(storage: &StorageHandle, binary_path: &str) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT OR IGNORE INTO keylog_apps (binary_path) VALUES (?1)",
        params![binary_path],
    )?;
    Ok(())
}

/// Retire une app ciblée — supprime aussi son éventuel état d'injection : elle n'est plus
/// couverte, jamais restaurée par un futur `stop()` qui ne la connaîtra plus.
pub fn remove_app(storage: &StorageHandle, binary_path: &str) -> Result<(), StorageError> {
    storage.lock().execute(
        "DELETE FROM keylog_apps WHERE binary_path = ?1",
        params![binary_path],
    )?;
    Ok(())
}

/// Enregistre l'état d'injection courant d'une app (appelé par `app_injection::inject_app`).
pub fn record_injection(
    storage: &StorageHandle,
    binary_path: &str,
    desktop_path: &str,
    backup_path: Option<&str>,
) -> Result<(), StorageError> {
    storage.lock().execute(
        "UPDATE keylog_apps SET desktop_path = ?2, backup_path = ?3 WHERE binary_path = ?1",
        params![binary_path, desktop_path, backup_path],
    )?;
    Ok(())
}

/// Efface l'état d'injection après restauration (`app_injection::restore_app`) — l'app reste
/// dans la liste ciblée, seule sa trace d'injection active est effacée.
pub fn clear_injection(storage: &StorageHandle, binary_path: &str) -> Result<(), StorageError> {
    storage.lock().execute(
        "UPDATE keylog_apps SET desktop_path = NULL, backup_path = NULL WHERE binary_path = ?1",
        params![binary_path],
    )?;
    Ok(())
}

fn row_to_app(row: &rusqlite::Row) -> rusqlite::Result<KeylogAppRow> {
    Ok(KeylogAppRow {
        binary_path: row.get(0)?,
        desktop_path: row.get(1)?,
        backup_path: row.get(2)?,
    })
}

/// Consommé par `keylog::restore_app_injection` (filet de sécurité `commands/settings.rs::
/// remove_keylog_app`, EPIC 3) pour vérifier si une app a une injection active avant suppression.
pub fn get_app(
    storage: &StorageHandle,
    binary_path: &str,
) -> Result<Option<KeylogAppRow>, StorageError> {
    let conn = storage.lock();
    conn.query_row(
        "SELECT binary_path, desktop_path, backup_path FROM keylog_apps WHERE binary_path = ?1",
        params![binary_path],
        row_to_app,
    )
    .optional()
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_add_list_record_clear_remove() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        add_app(&storage, "/usr/bin/firefox").unwrap();
        add_app(&storage, "/usr/bin/firefox").unwrap(); // idempotent

        let apps = list_apps(&storage).unwrap();
        assert_eq!(apps.len(), 1, "add_app ne doit jamais dupliquer");
        assert_eq!(apps[0].desktop_path, None);

        record_injection(
            &storage,
            "/usr/bin/firefox",
            "/home/x/.local/share/applications/firefox.desktop",
            Some("/home/x/.local/share/vitrail/keylog-backups/firefox.desktop"),
        )
        .unwrap();
        let apps = list_apps(&storage).unwrap();
        assert!(apps[0].desktop_path.is_some());
        assert!(apps[0].backup_path.is_some());

        clear_injection(&storage, "/usr/bin/firefox").unwrap();
        let apps = list_apps(&storage).unwrap();
        assert_eq!(apps[0].desktop_path, None);
        assert_eq!(apps[0].backup_path, None);

        remove_app(&storage, "/usr/bin/firefox").unwrap();
        assert!(list_apps(&storage).unwrap().is_empty());
    }

    #[test]
    fn get_app_absent_renvoie_none() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        assert_eq!(get_app(&storage, "/usr/bin/inconnu").unwrap(), None);
    }
}
