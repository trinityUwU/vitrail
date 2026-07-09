-- EPIC 5 (PLAN.md §6septies) : `flows` créée vide en EPIC 6 ne portait que les colonnes de
-- liste (id, timestamp_unix, process, destination, ip, port, protocol, size_bytes,
-- visibility) — complète ici avec le reste du contrat `Flow` (commands/types.rs) pour que
-- `storage::flows::insert_flow` persiste un enregistrement complet.

ALTER TABLE flows ADD COLUMN duration_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE flows ADD COLUMN source_ip TEXT NOT NULL DEFAULT '';
ALTER TABLE flows ADD COLUMN source_port INTEGER NOT NULL DEFAULT 0;
ALTER TABLE flows ADD COLUMN method TEXT;
ALTER TABLE flows ADD COLUMN path TEXT;
ALTER TABLE flows ADD COLUMN status INTEGER;
ALTER TABLE flows ADD COLUMN request_headers_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE flows ADD COLUMN response_headers_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE flows ADD COLUMN body_preview TEXT;
ALTER TABLE flows ADD COLUMN content_type TEXT;
ALTER TABLE flows ADD COLUMN certificate_json TEXT;
ALTER TABLE flows ADD COLUMN sources_json TEXT NOT NULL DEFAULT '[]';

CREATE INDEX idx_flows_timestamp ON flows (timestamp_unix);

-- `flows_fts` (FTS5) ne supporte pas `ALTER TABLE ADD COLUMN` ("virtual tables may not be
-- altered") — recréée avec la colonne `process` en plus (absente d'EPIC 6) pour que la
-- recherche plein texte (6.4) porte aussi sur le process, pas seulement
-- destination/body/headers. Sans risque : la table est encore vide à ce stade (EPIC 6 l'a
-- créée sans jamais l'alimenter, EPIC 5 est la première passe qui la peuple réellement).
DROP TABLE flows_fts;

CREATE VIRTUAL TABLE flows_fts USING fts5 (
    flow_id UNINDEXED,
    destination,
    body_preview,
    headers,
    process
);
