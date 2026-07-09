//! API publique storage pour `commands::destinations::tag_destination` — table
//! `destination_tags` (migration 0006, PLAN.md §6decies) : tag utilisateur posé sur une
//! destination, indépendant de `flows` (pas de table `destinations` dédiée). Même pattern
//! d'écriture minimale que `storage::keylog::add_app`.

use std::collections::HashMap;

use rusqlite::{params, OptionalExtension};

use super::connection::StorageHandle;
use super::error::StorageError;

/// Pose ou remplace le tag d'une destination — idempotent (`INSERT OR REPLACE`), la
/// destination n'a pas besoin d'être déjà apparue dans `flows` pour être taguée.
pub fn set_tag(storage: &StorageHandle, domain: &str, tag: &str) -> Result<(), StorageError> {
    storage.lock().execute(
        "INSERT INTO destination_tags (domain, tag) VALUES (?1, ?2)
         ON CONFLICT(domain) DO UPDATE SET tag = excluded.tag",
        params![domain, tag],
    )?;
    Ok(())
}

pub fn get_tag(storage: &StorageHandle, domain: &str) -> Result<Option<String>, StorageError> {
    let conn = storage.lock();
    conn.query_row(
        "SELECT tag FROM destination_tags WHERE domain = ?1",
        params![domain],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

/// Tous les tags en une seule requête — évite un aller-retour SQL par ligne dans
/// `list_destinations_aggregated` (N+1) alors que `get_destination_aggregated` (une seule
/// destination) reste sur `get_tag`.
pub fn get_all_tags(storage: &StorageHandle) -> Result<HashMap<String, String>, StorageError> {
    let conn = storage.lock();
    let mut stmt = conn.prepare("SELECT domain, tag FROM destination_tags")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect::<Result<HashMap<_, _>, _>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_tag_puis_get_tag_relit_la_derniere_valeur() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        assert_eq!(get_tag(&storage, "example.com").unwrap(), None);

        set_tag(&storage, "example.com", "surveillé").unwrap();
        assert_eq!(
            get_tag(&storage, "example.com").unwrap(),
            Some("surveillé".to_string())
        );

        set_tag(&storage, "example.com", "de confiance").unwrap();
        assert_eq!(
            get_tag(&storage, "example.com").unwrap(),
            Some("de confiance".to_string()),
            "un second set_tag doit remplacer, jamais dupliquer"
        );
    }

    #[test]
    fn get_all_tags_renvoie_toutes_les_destinations_taguees() {
        let storage = StorageHandle::open_in_memory().expect("storage mémoire");
        set_tag(&storage, "a.example.com", "un").unwrap();
        set_tag(&storage, "b.example.com", "deux").unwrap();

        let tags = get_all_tags(&storage).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags.get("a.example.com"), Some(&"un".to_string()));
        assert_eq!(tags.get("b.example.com"), Some(&"deux".to_string()));
    }
}
