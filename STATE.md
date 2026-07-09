# STATE — Vitrail

## Où on en est (2026-07-09)

Phase de planification uniquement. Aucune ligne de code applicatif écrite. Le repo contient :
- L'architecture complète pensée et documentée (`docs/PLAN.md`, `ARCHITECTURE.md`).
- Le plan d'implémentation en epics/stories (`docs/EPICS.md`).
- La spécification fonctionnelle exhaustive de l'UI, sans design (`docs/UI_SPEC.md`) —
  destinée à servir de base à un mockup produit par Chris via GLM 5.2, à intégrer ensuite.

## Décisions actées

- Nom du projet : **Vitrail**.
- Repo public, licence MIT.
- Stack : Tauri (Rust + React/TS), SQLite WAL, orchestration OpenSnitch + PolarProxy.
- Aucune réinvention de la capture/décryptage — Vitrail est une couche de corrélation et
  d'orchestration au-dessus d'outils existants.
- Zéro exposition réseau en v1 (IPC Tauri uniquement).

## Ouvert (cf. `docs/PLAN.md` section 7)

- Confirmation du périmètre réseau exact voulu par Chris ("accessible depuis le réseau" —
  interprété par défaut comme "aucune exposition réseau du tout", à confirmer).
- Discussion à venir sur l'orchestration technique détaillée (prochaine étape annoncée par
  Chris).
- Création effective du repo GitHub distant (`gh repo create`) — pas encore fait, en
  attente de confirmation avant action visible publiquement.

## Prochaine étape

Discussion orchestration technique (comment les sous-processus OpenSnitch/PolarProxy sont
supervisés, IPC exact, séquencement précis du kill switch) — puis démarrage EPIC 0
(scaffold Tauri réel).
