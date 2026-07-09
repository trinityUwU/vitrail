-- EPIC 6 (PLAN.md §6sexies) : schéma minimal — remplace les 3 JSONL provisoires
-- (system_events, capture_events, attribution_state) + tables vides pour EPIC 5
-- (flows, processes) + FTS5 créée mais non alimentée (branchement EPIC 5).

CREATE TABLE system_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_unix INTEGER NOT NULL,
    label TEXT NOT NULL,
    snapshot_json TEXT NOT NULL
);
CREATE INDEX idx_system_events_timestamp ON system_events (timestamp_unix);

CREATE TABLE capture_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_unix_ms INTEGER NOT NULL,
    interface TEXT NOT NULL,
    protocol TEXT NOT NULL,
    src_ip TEXT NOT NULL,
    dst_ip TEXT NOT NULL,
    src_port INTEGER,
    dst_port INTEGER,
    bytes INTEGER NOT NULL,
    sni TEXT,
    detected_protocol TEXT
);
CREATE INDEX idx_capture_events_timestamp ON capture_events (timestamp_unix_ms);

-- `pid` reste NULL pour l'usage actuel (sauvegarde de l'adresse socket d'origine) : colonne
-- prête pour un futur enrichissement pid-keyed (hors périmètre EPIC 6), index créé quand même
-- (PLAN.md §6sexies 6.2, coût nul).
CREATE TABLE attribution_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_unix INTEGER NOT NULL,
    pid INTEGER,
    original_address TEXT
);
CREATE INDEX idx_attribution_state_pid ON attribution_state (pid);

-- Vide dès cette passe (EPIC 5 alimentera), colonnes cohérentes avec commands/types.rs::Flow.
CREATE TABLE flows (
    id TEXT PRIMARY KEY,
    timestamp_unix INTEGER NOT NULL,
    process TEXT,
    destination TEXT,
    ip TEXT,
    port INTEGER,
    protocol TEXT,
    size_bytes INTEGER,
    visibility TEXT
);

-- Vide dès cette passe (EPIC 5 alimentera), cache pid->exe au fil du temps.
CREATE TABLE processes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pid INTEGER,
    exe_path TEXT,
    name TEXT,
    first_seen_unix INTEGER,
    last_seen_unix INTEGER
);

-- Créée, non alimentée, non branchée à une commande de recherche (EPIC 6sexies 6.4).
CREATE VIRTUAL TABLE flows_fts USING fts5 (
    flow_id UNINDEXED,
    destination,
    body_preview,
    headers
);
