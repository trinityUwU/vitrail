//! Migrations `.sql` embarquées (`include_str!`), numérotées, exécutées dans l'ordre au
//! démarrage, version courante trackée dans `schema_migrations` — mécanisme volontairement
//! simple, pas de dépendance externe de migration (PLAN.md §6sexies, story 6.1).

use rusqlite::{params, Connection};

use super::connection::now_unix;
use super::error::StorageError;

const MIGRATIONS: &[(i64, &str, &str)] = &[
    (
        1,
        "0001_init",
        include_str!("../../migrations/0001_init.sql"),
    ),
    (
        2,
        "0002_flows_detail",
        include_str!("../../migrations/0002_flows_detail.sql"),
    ),
    (
        3,
        "0003_keylog_apps",
        include_str!("../../migrations/0003_keylog_apps.sql"),
    ),
    (
        4,
        "0004_flows_five_tuple_index",
        include_str!("../../migrations/0004_flows_five_tuple_index.sql"),
    ),
];

pub(super) fn apply_migrations(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at_unix INTEGER NOT NULL
        );",
    )?;

    for (version, name, sql) in MIGRATIONS {
        if is_applied(conn, *version)? {
            continue;
        }
        conn.execute_batch(sql).map_err(|error| {
            tracing::error!(error = %error, version, name, "échec d'application d'une migration");
            error
        })?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at_unix) VALUES (?1, ?2, ?3)",
            params![version, name, now_unix()],
        )?;
        tracing::info!(version, name, "migration storage appliquée");
    }
    Ok(())
}

fn is_applied(conn: &Connection, version: i64) -> Result<bool, StorageError> {
    let applied: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
        params![version],
        |row| row.get(0),
    )?;
    Ok(applied)
}
