-- EPIC 5 fix (audit doublon Flow, story 5.2) : `correlation::update` recherche le flow déjà
-- émis par 4-tuple ip/port/source_ip/source_port (protocole exclu, cf. storage/flows.rs) pour
-- l'enrichir a posteriori avec un fragment déchiffré tardif plutôt que d'en créer un second.
-- Index composite pour que cette recherche (déclenchée à chaque fragment `Decryption` sans
-- entrée active dans le buffer de corrélation) ne fasse jamais un scan complet de `flows`.

CREATE INDEX idx_flows_five_tuple ON flows (ip, port, source_ip, source_port, timestamp_unix);
