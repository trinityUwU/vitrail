//! API publique storage pour `decryption/` (EPIC 4, PLAN.md §6nonies) — métadonnées de la CA
//! locale (ligne unique), exclusions utilisateur, événements de pinning détecté (table dédiée,
//! jamais mélangés au contenu déchiffré des `flows`).

use rusqlite::{params, OptionalExtension};

use super::connection::{now_unix, StorageHandle};
use super::error::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaMetadata {
    pub cert_path: String,
    pub key_path: String,
    pub fingerprint_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExclusionRow {
    pub name: String,
    pub kind: String,
}

pub struct PinningEvent<'a> {
    pub timestamp_unix: i64,
    pub protocol: &'a str,
    pub src_ip: &'a str,
    pub src_port: u16,
    pub dst_ip: &'a str,
    pub dst_port: u16,
    pub host: Option<&'a str>,
}

/// Lit les métadonnées de la CA actuellement installée — `None` si jamais générée
/// (`CaSubsystem::start()` la génère alors, story 4.1).
pub fn get_ca(storage: &StorageHandle) -> Result<Option<CaMetadata>, StorageError> {
    storage
        .lock()
        .query_row(
            "SELECT cert_path, key_path, fingerprint_sha256 FROM decryption_ca WHERE id = 1",
            [],
            |row| {
                Ok(CaMetadata {
                    cert_path: row.get(0)?,
                    key_path: row.get(1)?,
                    fingerprint_sha256: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

/// Remplace la ligne unique de métadonnées CA (rotation, story `rotate_ca`) — `INSERT OR
/// REPLACE` sur la clé fixe `id = 1`.
pub fn save_ca(storage: &StorageHandle, metadata: &CaMetadata) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT OR REPLACE INTO decryption_ca (id, cert_path, key_path, fingerprint_sha256, created_at_unix)
         VALUES (1, ?1, ?2, ?3, ?4)",
        params![metadata.cert_path, metadata.key_path, metadata.fingerprint_sha256, now_unix()],
    )?;
    Ok(())
}

pub fn list_exclusions(storage: &StorageHandle) -> Result<Vec<ExclusionRow>, StorageError> {
    let conn = storage.lock();
    let mut stmt = conn.prepare("SELECT name, kind FROM exclusions ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(ExclusionRow {
            name: row.get(0)?,
            kind: row.get(1)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn add_exclusion(storage: &StorageHandle, name: &str, kind: &str) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT OR IGNORE INTO exclusions (name, kind) VALUES (?1, ?2)",
        params![name, kind],
    )?;
    Ok(())
}

pub fn remove_exclusion(storage: &StorageHandle, name: &str) -> Result<(), StorageError> {
    storage
        .lock()
        .execute("DELETE FROM exclusions WHERE name = ?1", params![name])?;
    Ok(())
}

/// Insère un événement de pinning détecté (story 4.4) — table dédiée, distincte du contenu
/// déchiffré des `flows` (jamais mélangés, PLAN.md §6nonies).
pub fn record_pinning_event(
    storage: &StorageHandle,
    event: PinningEvent<'_>,
) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT INTO pinning_events
            (timestamp_unix, protocol, src_ip, src_port, dst_ip, dst_port, host)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event.timestamp_unix,
            event.protocol,
            event.src_ip,
            event.src_port,
            event.dst_ip,
            event.dst_port,
            event.host,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_ca_absente_puis_sauvegardee_puis_remplacee() {
        let storage = StorageHandle::open_in_memory().unwrap();
        assert_eq!(get_ca(&storage).unwrap(), None);

        let first = CaMetadata {
            cert_path: "/a/ca.pem".into(),
            key_path: "/a/ca.key".into(),
            fingerprint_sha256: "a".repeat(64),
        };
        save_ca(&storage, &first).unwrap();
        assert_eq!(get_ca(&storage).unwrap(), Some(first));

        let rotated = CaMetadata {
            cert_path: "/b/ca.pem".into(),
            key_path: "/b/ca.key".into(),
            fingerprint_sha256: "b".repeat(64),
        };
        save_ca(&storage, &rotated).unwrap();
        assert_eq!(get_ca(&storage).unwrap(), Some(rotated));
    }

    #[test]
    fn cycle_add_list_remove_exclusion() {
        let storage = StorageHandle::open_in_memory().unwrap();
        add_exclusion(&storage, "example.com", "destination").unwrap();
        add_exclusion(&storage, "example.com", "destination").unwrap(); // idempotent

        let rows = list_exclusions(&storage).unwrap();
        assert_eq!(rows.len(), 1, "add_exclusion ne doit jamais dupliquer");

        remove_exclusion(&storage, "example.com").unwrap();
        assert!(list_exclusions(&storage).unwrap().is_empty());
    }

    #[test]
    fn record_pinning_event_persiste() {
        let storage = StorageHandle::open_in_memory().unwrap();
        record_pinning_event(
            &storage,
            PinningEvent {
                timestamp_unix: 1000,
                protocol: "tcp",
                src_ip: "10.0.0.5",
                src_port: 51000,
                dst_ip: "93.184.216.34",
                dst_port: 443,
                host: Some("example.com"),
            },
        )
        .unwrap();

        let count: i64 = storage
            .lock()
            .query_row("SELECT COUNT(*) FROM pinning_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
