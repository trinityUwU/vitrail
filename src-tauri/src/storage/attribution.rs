//! API publique storage pour `attribution/` — remplace `attribution_state.jsonl`
//! (`daemon_config::save_original_address`/`read_last_original_address`).

use rusqlite::params;

use super::connection::{now_unix, StorageHandle};
use super::error::StorageError;

/// Sauvegarde l'adresse socket d'origine du daemon `opensnitchd` AVANT reconfiguration
/// (story 1.1/1.6). Ne remplace jamais une entrée : la restauration relit toujours la
/// dernière ligne valide (même sémantique que l'ancien JSONL append-only).
pub fn save_origin_socket(
    storage: &StorageHandle,
    original_address: &str,
) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT INTO attribution_state (timestamp_unix, pid, original_address)
         VALUES (?1, NULL, ?2)",
        params![now_unix(), original_address],
    )?;
    Ok(())
}

/// Relit la dernière adresse d'origine sauvegardée.
pub fn read_origin_socket(storage: &StorageHandle) -> Result<Option<String>, StorageError> {
    let conn = storage.lock();
    let result = conn.query_row(
        "SELECT original_address FROM attribution_state
         WHERE original_address IS NOT NULL
         ORDER BY id DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    );
    match result {
        Ok(address) => Ok(Some(address)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(StorageError::Sqlite(error)),
    }
}
