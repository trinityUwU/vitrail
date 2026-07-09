-- EPIC 4 (PLAN.md §6nonies) : métadonnées de la CA locale dédiée Vitrail (ligne unique),
-- liste d'exclusions utilisateur (destination/processus), événements de pinning détecté
-- (distincts du contenu déchiffré, jamais mélangés).

CREATE TABLE decryption_ca (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    cert_path TEXT NOT NULL,
    key_path TEXT NOT NULL,
    fingerprint_sha256 TEXT NOT NULL,
    created_at_unix INTEGER NOT NULL
);

CREATE TABLE exclusions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL CHECK (kind IN ('destination', 'processus'))
);

CREATE TABLE pinning_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_unix INTEGER NOT NULL,
    protocol TEXT NOT NULL,
    src_ip TEXT NOT NULL,
    src_port INTEGER NOT NULL,
    dst_ip TEXT NOT NULL,
    dst_port INTEGER NOT NULL,
    host TEXT
);

CREATE INDEX idx_pinning_events_timestamp ON pinning_events(timestamp_unix);
