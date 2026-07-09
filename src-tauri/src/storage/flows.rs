//! API publique storage pour `correlation/` (EPIC 5) — table `flows` + FTS5 `flows_fts`
//! (colonnes créées vides en EPIC 6, migration 0002 les complète). `insert_flow` prend
//! directement `&shared::Flow` (et non une struct storage-locale comme `events.rs`) : `Flow`
//! est possédé par `crate::shared` précisément pour permettre ça sans faire dépendre
//! `storage/` d'un domaine métier (cf. doc de module `shared/mod.rs`).

use rusqlite::{params, OptionalExtension};

use crate::shared::{CertificateInfo, Flow, FlowVisibility};

use super::connection::{now_unix, StorageHandle};
use super::error::StorageError;

/// Champs `Flow` pré-sérialisés en JSON — évite de repasser `flow` + 4 `String` séparées à
/// travers `insert_flow_row`/`insert_fts_row` (lisibilité + limite 35 lignes/fonction).
struct EncodedFlow {
    request_headers_json: String,
    response_headers_json: String,
    certificate_json: Option<String>,
    sources_json: String,
}

fn encode_flow(flow: &Flow) -> Result<EncodedFlow, StorageError> {
    Ok(EncodedFlow {
        request_headers_json: serde_json::to_string(&flow.request_headers)?,
        response_headers_json: serde_json::to_string(&flow.response_headers)?,
        certificate_json: flow
            .certificate
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?,
        sources_json: serde_json::to_string(&flow.sources)?,
    })
}

/// Insère un `Flow` corrélé dans `flows` ET `flows_fts` (5.4/6.4 — les deux tables sont
/// alimentées ensemble, jamais l'une sans l'autre). `INSERT OR IGNORE` : le moteur de
/// corrélation garantit déjà l'absence de doublon par 5-tuple/fenêtre (5.2), mais un `id`
/// déjà présent (ex: relance après crash) ne doit jamais faire échouer l'insertion.
pub fn insert_flow(storage: &StorageHandle, flow: &Flow) -> Result<(), StorageError> {
    let encoded = encode_flow(flow)?;
    let conn = storage.lock();
    insert_flow_row(&conn, flow, &encoded)?;
    insert_fts_row(&conn, flow, &encoded)?;
    Ok(())
}

const INSERT_FLOW_SQL: &str = "INSERT OR IGNORE INTO flows
    (id, timestamp_unix, process, destination, ip, port, protocol, size_bytes,
     visibility, duration_ms, source_ip, source_port, method, path, status,
     request_headers_json, response_headers_json, body_preview, content_type,
     certificate_json, sources_json)
 VALUES
    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18,
     ?19, ?20, ?21)";

fn insert_flow_row(
    conn: &rusqlite::Connection,
    flow: &Flow,
    encoded: &EncodedFlow,
) -> Result<(), StorageError> {
    conn.execute(
        INSERT_FLOW_SQL,
        params![
            flow.id,
            now_unix(),
            flow.process,
            flow.destination,
            flow.ip,
            flow.port,
            flow.protocol,
            flow.size_bytes as i64,
            visibility_to_str(flow.visibility),
            flow.duration_ms as i64,
            flow.source_ip,
            flow.source_port,
            flow.method,
            flow.path,
            flow.status,
            encoded.request_headers_json,
            encoded.response_headers_json,
            flow.body_preview,
            flow.content_type,
            encoded.certificate_json,
            encoded.sources_json,
        ],
    )?;
    Ok(())
}

fn insert_fts_row(
    conn: &rusqlite::Connection,
    flow: &Flow,
    encoded: &EncodedFlow,
) -> Result<(), StorageError> {
    let headers_text = format!(
        "{} {}",
        encoded.request_headers_json, encoded.response_headers_json
    );
    conn.execute(
        "INSERT INTO flows_fts (flow_id, destination, body_preview, headers, process)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            flow.id,
            flow.destination,
            flow.body_preview,
            headers_text,
            flow.process,
        ],
    )?;
    Ok(())
}

/// Met à jour un flow déjà persisté (enrichissement a posteriori par un fragment déchiffré
/// tardif arrivé après fermeture capture+attribution, 5.2 — cf. `correlation::update`) —
/// ré-écrit `flows` ET `flows_fts` pour le même `id`, jamais un nouvel `INSERT` : c'est
/// précisément ce qui évite le doublon que produirait un second `insert_flow`.
pub fn update_flow(storage: &StorageHandle, flow: &Flow) -> Result<(), StorageError> {
    let encoded = encode_flow(flow)?;
    let conn = storage.lock();
    update_flow_row(&conn, flow, &encoded)?;
    update_fts_row(&conn, flow, &encoded)?;
    Ok(())
}

const UPDATE_FLOW_SQL: &str = "UPDATE flows SET
    process = ?2, destination = ?3, visibility = ?4, method = ?5, path = ?6, status = ?7,
    request_headers_json = ?8, response_headers_json = ?9, body_preview = ?10,
    content_type = ?11, certificate_json = ?12, sources_json = ?13
 WHERE id = ?1";

fn update_flow_row(
    conn: &rusqlite::Connection,
    flow: &Flow,
    encoded: &EncodedFlow,
) -> Result<(), StorageError> {
    conn.execute(
        UPDATE_FLOW_SQL,
        params![
            flow.id,
            flow.process,
            flow.destination,
            visibility_to_str(flow.visibility),
            flow.method,
            flow.path,
            flow.status,
            encoded.request_headers_json,
            encoded.response_headers_json,
            flow.body_preview,
            flow.content_type,
            encoded.certificate_json,
            encoded.sources_json,
        ],
    )?;
    Ok(())
}

fn update_fts_row(
    conn: &rusqlite::Connection,
    flow: &Flow,
    encoded: &EncodedFlow,
) -> Result<(), StorageError> {
    let headers_text = format!(
        "{} {}",
        encoded.request_headers_json, encoded.response_headers_json
    );
    conn.execute(
        "UPDATE flows_fts SET destination = ?1, body_preview = ?2, headers = ?3, process = ?4
         WHERE flow_id = ?5",
        params![
            flow.destination,
            flow.body_preview,
            headers_text,
            flow.process,
            flow.id,
        ],
    )?;
    Ok(())
}

/// Recherche le flow le plus récent correspondant à un 4-tuple ip/port/source_ip/source_port
/// dans une fenêtre de `window_secs` secondes — protocole volontairement exclu du filtre :
/// `Flow.protocol` porte le protocole applicatif détecté par `capture/` (ex "TLS 1.3"), pas le
/// protocole transport brut du 5-tuple d'origine, comparer dessus produirait de faux négatifs.
/// Utilisé par `correlation::update` (5.2) pour retrouver un flow déjà émis avant de l'enrichir
/// a posteriori avec un fragment déchiffré tardif, plutôt que d'en persister un second pour la
/// même connexion.
pub fn find_recent_by_five_tuple(
    storage: &StorageHandle,
    ip: &str,
    port: u16,
    source_ip: &str,
    source_port: u16,
    window_secs: i64,
) -> Result<Option<Flow>, StorageError> {
    let conn = storage.lock();
    let earliest = now_unix() - window_secs;
    conn.query_row(
        "SELECT id, timestamp_unix, process, destination, ip, port, protocol, size_bytes,
                visibility, duration_ms, source_ip, source_port, method, path, status,
                request_headers_json, response_headers_json, body_preview, content_type,
                certificate_json, sources_json
         FROM flows
         WHERE ip = ?1 AND port = ?2 AND source_ip = ?3 AND source_port = ?4
               AND timestamp_unix >= ?5
         ORDER BY timestamp_unix DESC LIMIT 1",
        params![ip, port, source_ip, source_port, earliest],
        row_to_flow,
    )
    .optional()
    .map_err(Into::into)
}

/// Les plus récents en premier, bornés à `limit` (Timeline, story 8.1).
pub fn list_flows(storage: &StorageHandle, limit: u32) -> Result<Vec<Flow>, StorageError> {
    let conn = storage.lock();
    let mut stmt = conn.prepare(
        "SELECT id, timestamp_unix, process, destination, ip, port, protocol, size_bytes,
                visibility, duration_ms, source_ip, source_port, method, path, status,
                request_headers_json, response_headers_json, body_preview, content_type,
                certificate_json, sources_json
         FROM flows ORDER BY timestamp_unix DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], row_to_flow)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn get_flow(storage: &StorageHandle, id: &str) -> Result<Option<Flow>, StorageError> {
    let conn = storage.lock();
    conn.query_row(
        "SELECT id, timestamp_unix, process, destination, ip, port, protocol, size_bytes,
                visibility, duration_ms, source_ip, source_port, method, path, status,
                request_headers_json, response_headers_json, body_preview, content_type,
                certificate_json, sources_json
         FROM flows WHERE id = ?1",
        params![id],
        row_to_flow,
    )
    .optional()
    .map_err(Into::into)
}

/// Recherche plein texte via `flows_fts` (6.4) — requête vide = tous les flux (parité avec
/// l'ancien comportement mocké de `commands/flows.rs`). Chaque terme est traité en préfixe
/// (`terme*`) : FTS5 ne fait pas de correspondance "contains" sur sous-chaîne arbitraire.
pub fn search_flows(storage: &StorageHandle, query: &str) -> Result<Vec<Flow>, StorageError> {
    let needle = query.trim();
    if needle.is_empty() {
        return list_flows(storage, 200);
    }
    let match_query = build_prefix_match(needle);

    let conn = storage.lock();
    let mut stmt = conn.prepare(
        "SELECT f.id, f.timestamp_unix, f.process, f.destination, f.ip, f.port, f.protocol,
                f.size_bytes, f.visibility, f.duration_ms, f.source_ip, f.source_port,
                f.method, f.path, f.status, f.request_headers_json, f.response_headers_json,
                f.body_preview, f.content_type, f.certificate_json, f.sources_json
         FROM flows_fts fts
         JOIN flows f ON f.id = fts.flow_id
         WHERE flows_fts MATCH ?1
         ORDER BY f.timestamp_unix DESC",
    )?;
    let rows = stmt.query_map(params![match_query], row_to_flow)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Échappe les guillemets doubles (syntaxe FTS5) puis enveloppe chaque terme en préfixe
/// (`"terme"*`) — reste tolérant à la ponctuation qu'un utilisateur pourrait taper.
fn build_prefix_match(needle: &str) -> String {
    needle
        .split_whitespace()
        .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `pub(super)` — réutilisée par `storage::aggregates` pour rester la source unique de la
/// correspondance `FlowVisibility` <-> texte stocké (pas de second mapping dupliqué).
pub(super) fn visibility_to_str(visibility: FlowVisibility) -> &'static str {
    match visibility {
        FlowVisibility::Fully => "fully",
        FlowVisibility::Meta => "meta",
        FlowVisibility::Attrib => "attrib",
        FlowVisibility::Unknown => "unknown",
    }
}

pub(super) fn visibility_from_str(value: &str) -> FlowVisibility {
    match value {
        "fully" => FlowVisibility::Fully,
        "meta" => FlowVisibility::Meta,
        "attrib" => FlowVisibility::Attrib,
        _ => FlowVisibility::Unknown,
    }
}

/// Reconstruit `Flow.timestamp` (affichage `HH:MM:SS`) depuis `timestamp_unix` — évite de
/// stocker le même instant deux fois (colonne numérique pour le tri, texte pour l'affichage).
fn timestamp_display(timestamp_unix: i64) -> String {
    let secs = timestamp_unix.max(0) as u64;
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{h:02}:{m:02}:{s:02}")
}

fn row_to_flow(row: &rusqlite::Row) -> rusqlite::Result<Flow> {
    let visibility_str: String = row.get(8)?;
    let request_headers_json: String = row.get(15)?;
    let response_headers_json: String = row.get(16)?;
    let certificate_json: Option<String> = row.get(19)?;
    let sources_json: String = row.get(20)?;
    let timestamp_unix: i64 = row.get(1)?;

    Ok(Flow {
        id: row.get(0)?,
        timestamp: timestamp_display(timestamp_unix),
        process: row.get(2)?,
        destination: row.get(3)?,
        ip: row.get(4)?,
        port: row.get(5)?,
        protocol: row.get(6)?,
        size_bytes: row.get::<_, i64>(7)? as u64,
        duration_ms: row.get::<_, i64>(9)? as u64,
        visibility: visibility_from_str(&visibility_str),
        source_ip: row.get(10)?,
        source_port: row.get(11)?,
        method: row.get(12)?,
        path: row.get(13)?,
        status: row.get(14)?,
        request_headers: decode_json_or_default(&request_headers_json),
        response_headers: decode_json_or_default(&response_headers_json),
        body_preview: row.get(17)?,
        content_type: row.get(18)?,
        certificate: certificate_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<CertificateInfo>(json).ok()),
        sources: decode_json_or_default(&sources_json),
    })
}

/// Une ligne JSON corrompue ne doit jamais faire échouer toute la lecture d'un flux — loggée
/// et remplacée par une valeur vide plutôt que de propager l'erreur (données d'affichage,
/// pas de logique métier critique).
fn decode_json_or_default<T: Default + serde::de::DeserializeOwned>(json: &str) -> T {
    serde_json::from_str(json).unwrap_or_else(|error| {
        tracing::warn!(error = %error, "décodage JSON d'un champ flows échoué, valeur vide utilisée");
        T::default()
    })
}
