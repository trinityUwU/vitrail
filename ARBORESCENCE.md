# ARBORESCENCE — Vitrail

Une ligne par fichier. Régénérer après tout ajout/suppression significatif.

```
.env.example              — variables d'environnement requises (template)
.gitignore                — exclusions Rust/Tauri/Bun + données sensibles
ARCHITECTURE.md            — carte des domaines, frontières de module, décisions figées
CODE_OF_CONDUCT.md         — règles de contribution liées à la confiance/sécurité du projet
CONTRIBUTING.md            — setup dev, invariants non négociables, style de code
LICENSE                    — MIT
README.md                  — présentation publique, positionnement face aux outils existants
STATE.md                   — état courant du projet, décisions actées, ouvert, prochaine étape
TODO.md                    — vue résumée des epics + backlog non structuré
docs/EPICS.md              — plan d'implémentation détaillé (12 epics, stories actionnables)
docs/PLAN.md                — architecture technique complète, état de l'art, réversibilité
docs/UI_SPEC.md             — spécification fonctionnelle exhaustive de l'UI (sans design)
logs/.gitkeep               — placeholder pour dossier logs (reset par start.sh, jamais versionné)
restart.sh                  — stop.sh puis start.sh
start.sh                    — lance Vitrail (mode dev Tauri), gestion PID + reset logs
stop.sh                     — arrête Vitrail proprement via PID file

# À venir (EPIC 0 — scaffold Tauri non encore fait)
src-tauri/src/attribution/  — client OpenSnitch, cache pid→exe
src-tauri/src/capture/      — capture AF_PACKET, parsing 5-tuple, SNI
src-tauri/src/decryption/   — orchestration PolarProxy, fail-open
src-tauri/src/keylog/       — pipeline SSLKEYLOGFILE, tshark
src-tauri/src/correlation/  — fusion des sources en timeline unique
src-tauri/src/storage/      — SQLite WAL, rétention, recherche
src-tauri/src/killswitch/   — cycle de vie orchestré, snapshot/diff
src-tauri/src/shared/       — types communs, config, logging
src-tauri/src/commands/     — surface IPC exposée au frontend
src/                         — frontend React/TS (attend intégration du mockup GLM 5.2)
```
