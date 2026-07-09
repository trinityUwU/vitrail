//! Recherche/session (story 6.7) — une "session" est dérivée d'une paire d'événements
//! `system_events` (`pre-activation` / `post-deactivation`), cohérent avec ce que le mock de
//! `commands/settings.rs::list_sessions` représentait (regroupement par plage d'activation du
//! kill switch). `flows`/`processes` restent vides jusqu'à EPIC 5 : le détail de session est
//! donc partiel (volume dérivé de `capture_events`, flux vides) — attendu, signalé au rapport.

use rusqlite::params;

use super::connection::StorageHandle;
use super::error::StorageError;

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub started_at_unix: i64,
    pub ended_at_unix: i64,
}

/// Reconstruit les sessions en pairant chaque `pre-activation` avec le `post-deactivation`
/// suivant, dans l'ordre chronologique. Un `pre-activation` sans `post-deactivation` associé
/// (kill switch encore actif) ne produit pas de session — cohérent avec le mock qui ne
/// listait que des sessions terminées.
pub fn list_sessions(storage: &StorageHandle) -> Result<Vec<SessionRow>, StorageError> {
    let conn = storage.lock();
    let mut stmt = conn.prepare(
        "SELECT id, timestamp_unix, label FROM system_events
         WHERE label IN ('pre-activation', 'post-deactivation')
         ORDER BY timestamp_unix ASC, id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut sessions = Vec::new();
    let mut pending_start: Option<(i64, i64)> = None;
    for row in rows {
        let (event_id, timestamp, label) = row?;
        match label.as_str() {
            "pre-activation" => pending_start = Some((event_id, timestamp)),
            "post-deactivation" => {
                if let Some((start_id, started_at)) = pending_start.take() {
                    sessions.push(SessionRow {
                        id: format!("{start_id}-{event_id}"),
                        started_at_unix: started_at,
                        ended_at_unix: timestamp,
                    });
                }
            }
            _ => {}
        }
    }
    Ok(sessions)
}

/// Détail d'une session par son id composite (`"<start_event_id>-<end_event_id>"`).
pub fn get_session(storage: &StorageHandle, id: &str) -> Result<Option<SessionRow>, StorageError> {
    Ok(list_sessions(storage)?.into_iter().find(|s| s.id == id))
}

/// Supprime les deux événements `system_events` bornant la session (pas de table `sessions`
/// dédiée : une session est une vue dérivée, la supprimer supprime ses bornes).
pub fn delete_session(storage: &StorageHandle, id: &str) -> Result<(), StorageError> {
    let Some((start_id, end_id)) = parse_session_id(id) else {
        return Ok(());
    };
    storage.lock().execute(
        "DELETE FROM system_events WHERE id IN (?1, ?2)",
        params![start_id, end_id],
    )?;
    Ok(())
}

/// Volume total (octets) capturé pendant la fenêtre temporelle de la session — seule donnée
/// réellement disponible tant que `flows`/`processes` ne sont pas alimentées (EPIC 5).
pub fn session_volume_bytes(
    storage: &StorageHandle,
    started_at_unix: i64,
    ended_at_unix: i64,
) -> Result<i64, StorageError> {
    let conn = storage.lock();
    let bytes: i64 = conn.query_row(
        "SELECT COALESCE(SUM(bytes), 0) FROM capture_events
         WHERE timestamp_unix_ms >= ?1 AND timestamp_unix_ms <= ?2",
        params![started_at_unix * 1000, ended_at_unix * 1000],
        |row| row.get(0),
    )?;
    Ok(bytes)
}

fn parse_session_id(id: &str) -> Option<(i64, i64)> {
    let (start, end) = id.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}
