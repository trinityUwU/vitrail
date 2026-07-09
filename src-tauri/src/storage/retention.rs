//! Purge/rétention (story 6.3/6.6) — `DELETE` sur les 3 tables d'événements + `VACUUM`.
//! Consommé par `commands/settings.rs` (`purge_data`/`purge_logs`), jamais de SQL direct
//! côté `commands/` (ARCHITECTURE.md).

use super::connection::StorageHandle;
use super::error::StorageError;

#[derive(Debug, Clone, Copy)]
pub struct PurgeStats {
    pub deleted_rows: u64,
    pub freed_bytes: i64,
}

/// Purge totale (`before_unix = None`) ou ciblée (lignes strictement antérieures au seuil)
/// des 3 tables d'événements, suivie d'un `VACUUM`. Utilisée par `purge_data` (6.6) et par la
/// politique de rétention (6.3, seuil dérivé de `Settings.retention_days`).
///
/// Le `VACUUM` (potentiellement plusieurs centaines de ms à quelques secondes sur une base
/// volumineuse) est exécuté sous un verrou séparé des `DELETE` : le `Mutex<Connection>` est
/// relâché entre les deux étapes pour ne pas bloquer `capture/` (jusqu'à 2000 paquets/s) ni
/// `attribution/` (start/stop) pendant toute sa durée. Reste malgré tout une opération
/// bloquante rare, déclenchée uniquement par une action manuelle utilisateur — jamais un
/// chemin chaud — donc pas de refonte en pool de connexions dans cette passe (connexion
/// unique partagée, PLAN.md §6sexies).
pub fn purge_data_before(
    storage: &StorageHandle,
    before_unix: Option<i64>,
) -> Result<PurgeStats, StorageError> {
    let before_bytes = {
        let conn = storage.lock();
        db_size_bytes(&conn)?
    };

    let deleted_rows = {
        let conn = storage.lock();
        match before_unix {
            Some(threshold) => {
                let system = conn.execute(
                    "DELETE FROM system_events WHERE timestamp_unix < ?1",
                    [threshold],
                )?;
                let capture = conn.execute(
                    "DELETE FROM capture_events WHERE timestamp_unix_ms < ?1",
                    [threshold.saturating_mul(1000)],
                )?;
                let attribution = conn.execute(
                    "DELETE FROM attribution_state WHERE timestamp_unix < ?1",
                    [threshold],
                )?;
                system + capture + attribution
            }
            None => {
                let system = conn.execute("DELETE FROM system_events", [])?;
                let capture = conn.execute("DELETE FROM capture_events", [])?;
                let attribution = conn.execute("DELETE FROM attribution_state", [])?;
                system + capture + attribution
            }
        }
    };

    let after_bytes = {
        let conn = storage.lock();
        conn.execute_batch("VACUUM")?;
        db_size_bytes(&conn)?
    };

    Ok(PurgeStats {
        deleted_rows: deleted_rows as u64,
        freed_bytes: (before_bytes - after_bytes).max(0),
    })
}

/// Purge du journal système seul (`purge_logs`, 6.6) — cible `system_events` uniquement, qui
/// est la table dont `get_log_entries` (actuellement mocké) représente le contenu. Même
/// raisonnement que `purge_data_before` : `VACUUM` sous un verrou séparé du `DELETE`.
pub fn purge_logs(storage: &StorageHandle) -> Result<u64, StorageError> {
    let deleted = {
        let conn = storage.lock();
        conn.execute("DELETE FROM system_events", [])?
    };
    {
        let conn = storage.lock();
        conn.execute_batch("VACUUM")?;
    }
    Ok(deleted as u64)
}

fn db_size_bytes(conn: &rusqlite::Connection) -> Result<i64, StorageError> {
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
    Ok(page_count * page_size)
}
