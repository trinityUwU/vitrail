//! Erreur unifiée du domaine `storage/` — jamais de `rusqlite::Error`/`std::io::Error` exposée
//! en dehors de ce domaine (frontière stricte, ARCHITECTURE.md).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("erreur SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("erreur d'E/S: {0}")]
    Io(#[from] std::io::Error),
    #[error("erreur de sérialisation JSON: {0}")]
    Serde(#[from] serde_json::Error),
}
