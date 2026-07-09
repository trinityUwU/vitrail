# STATE — Vitrail

## Où on en est (2026-07-09)

Le scaffold Tauri réel existe et build. Le repo contient :
- L'architecture complète pensée et documentée (`docs/PLAN.md`, `ARCHITECTURE.md`).
- Le plan d'implémentation en epics/stories (`docs/EPICS.md`).
- La spécification fonctionnelle exhaustive de l'UI (`docs/UI_SPEC.md`).
- Un mockup statique GLM 5.2 (`docs/Mockup.html`), revu (`docs/MOCKUP_REVIEW.md`, 3 défauts
  identifiés : données macOS, nom de chaîne nftables incohérent, bug de texte FR).
- **EPIC 0 (fondations)** et **EPIC 8 (contrat UI/IPC)** livrés : projet Tauri (Rust +
  React/TS + Bun) scaffoldé, frontend porté du mockup en 13 vertical slices + `shared/`
  (cf. `ARBORESCENCE.md`), 8 modules de domaine stubs (`mod.rs` = une ligne de
  responsabilité), 24 commandes IPC dans `src-tauri/src/commands/` retournant des données
  de démo explicitement commentées comme temporaires. Les 3 défauts du mockup sont corrigés
  et vérifiés (grep) dans le code source. `cargo build`/`clippy`/`fmt` et `bun run build`
  passent sans erreur ni warning.
- Streaming temps réel simulé par un émetteur factice (thread Rust + événements Tauri),
  documenté comme placeholder EPIC 8.4 — à remplacer par le vrai flux de `correlation/`.

## Décisions actées

- Nom du projet : **Vitrail**.
- Repo public, licence MIT.
- Stack : Tauri (Rust + React/TS), SQLite WAL, orchestration OpenSnitch + PolarProxy.
- Aucune réinvention de la capture/décryptage — Vitrail est une couche de corrélation et
  d'orchestration au-dessus d'outils existants.
- Zéro exposition réseau en v1 (IPC Tauri uniquement) — appliqué concrètement lors du
  portage : polices Google Fonts et script CDN Lucide du mockup retirés (contradiction avec
  la garantie "zéro appel réseau" affichée sur l'écran Confidentialité), remplacés par
  `lucide-react` en dépendance npm et des variables de police avec fallback système.

## Ouvert

- **Polices exactes** (`DM Serif Display`, `Outfit`) : gardées en variables CSS avec
  fallback système, pas encore self-hébergées. Décision à prendre : `@fontsource` (bundlé,
  zéro réseau) ou fichiers de police locaux fournis par Chris.
- Confirmation du périmètre réseau exact voulu par Chris ("accessible depuis le réseau" —
  interprété par défaut comme "aucune exposition réseau du tout", confirmé implicitement
  par la décision ci-dessus, mais pas encore acté explicitement).
- Discussion à venir sur l'orchestration technique détaillée (supervision des sous-process
  OpenSnitch/PolarProxy, séquencement précis du kill switch) — EPICs 1 à 7, non commencés.
- Création effective du repo GitHub distant (`gh repo create`) — pas encore fait, en
  attente de confirmation avant action visible publiquement.
- Icônes app Tauri : encore celles du template par défaut, à remplacer.

## Prochaine étape

Audit de complétude en cours (agent dédié à la recherche des manques, lancé
2026-07-09) — cf. son rapport une fois disponible. Puis discussion orchestration technique
et démarrage des EPICs 1-7 (logique réelle des domaines).
