-- EPIC 3 (PLAN.md §6octies) : persiste la liste des apps ciblées par l'injection
-- SSLKEYLOGFILE (remplace le mock en mémoire de commands/settings.rs::list_keylog_apps) +
-- l'état d'injection courant (chemin .desktop réécrit, sauvegarde de la surcharge
-- préexistante si elle existait avant l'écriture de Vitrail).

CREATE TABLE keylog_apps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    binary_path TEXT NOT NULL UNIQUE,
    desktop_path TEXT,
    backup_path TEXT
);
