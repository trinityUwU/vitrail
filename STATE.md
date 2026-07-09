# STATE — Vitrail

## Où on en est (2026-07-09)

Scaffold Tauri complet et cohérent. Le repo contient :
- L'architecture complète pensée et documentée (`docs/PLAN.md`, `ARCHITECTURE.md`).
- Le plan d'implémentation en epics/stories, à jour avec le contrat IPC réel
  (`docs/EPICS.md`).
- La spécification fonctionnelle exhaustive de l'UI (`docs/UI_SPEC.md`).
- Un mockup statique GLM 5.2 (`docs/Mockup.html`), revu (`docs/MOCKUP_REVIEW.md`, 3 défauts
  identifiés et corrigés au portage).
- **EPIC 0** et **EPIC 8** livrés et audités : projet Tauri (Rust + React/TS + Bun), 8
  modules de domaine stubs, frontend en 13 vertical slices + `shared/`. Un premier audit de
  complétude (agent dédié) a trouvé 3 manques bloquants et un pattern de boutons factices
  non documentés ; une seconde passe les a tous corrigés :
  - Contrat `Flow` désormais complet (headers, corps, content-type, IP/port source,
    certificat, sources de corrélation) — l'Inspecteur de flux lit ces champs au lieu de
    les fabriquer en JSX.
  - Désync des exclusions entre écrans corrigée via `ExclusionsProvider`/
    `useExclusionsContext` (Context React, même pattern que `KillSwitchProvider`) — un
    ajout depuis Processus/Destinations apparaît maintenant immédiatement dans
    Paramètres > Exclusions.
  - Commandes IPC manquantes ajoutées et branchées : tag de destination, CRUD complet des
    règles d'alerte + historique des déclenchements, requêtes de recherche sauvegardées,
    purge de données/logs, détail et suppression de session, persistance réelle des
    paramètres Notifications/Keylog, export/import de config. Plus aucun bouton
    "fonctionnalité disponible dans la version complète" (sauf régénération de CA, qui
    appelle déjà `rotate_ca` — la vraie logique système reste EPIC 4/9).
  - Deux résidus mineurs corrigés directement (sans agent) : copie de l'empreinte CA
    (`CaTab.tsx`) qui était encore un faux toast, et un bloc `try/catch` mort dans
    `ProcessDetailPanel`/`DestinationDetailPanel` qui affichait un toast de succès même en
    cas d'échec silencieux de l'exclusion (`addExclusion` retourne maintenant un booléen).
- `cargo build`/`clippy -D warnings`/`fmt --check` et `bun run build` passent tous, sans
  erreur ni warning, vérifiés après chaque passe.

## Décisions actées

- Nom du projet : **Vitrail**.
- Repo public, licence MIT.
- Stack : Tauri (Rust + React/TS), SQLite WAL, orchestration OpenSnitch + PolarProxy.
- Aucune réinvention de la capture/décryptage — Vitrail est une couche de corrélation et
  d'orchestration au-dessus d'outils existants.
- Zéro exposition réseau en v1 (IPC Tauri uniquement) — polices Google Fonts et CDN Lucide
  du mockup retirés au profit de dépendances locales, cohérent avec l'écran Confidentialité.
- Les commandes CRUD mockées (alertes, tags, requêtes sauvegardées) ne persistent pas entre
  rechargements de l'app — comportement accepté jusqu'à EPIC 6/7 (vraie persistance SQLite),
  cohérent avec le reste du contrat mocké.

## Ouvert

- **Polices exactes** (`DM Serif Display`, `Outfit`) : variables CSS avec fallback système,
  pas encore self-hébergées (`@fontsource` ou fichiers fournis par Chris — décision à
  prendre).
- Confirmation explicite du périmètre réseau ("accessible depuis le réseau" — interprété
  comme aucune exposition réseau, cohérent avec les choix déjà faits, mais jamais confirmé
  mot pour mot par Chris).
- Discussion orchestration technique (supervision des sous-process OpenSnitch/PolarProxy,
  séquencement précis du kill switch) — EPICs 1 à 7, non commencés.
- Création du repo GitHub distant (`gh repo create`) — pas encore fait, attente de
  confirmation avant action visible publiquement.
- Icônes app Tauri : encore celles du template par défaut.

## Prochaine étape

Discussion orchestration technique avec Chris, puis démarrage des EPICs 1-7 (logique réelle
des domaines : OpenSnitch, capture AF_PACKET, PolarProxy, SSLKEYLOGFILE, corrélation,
SQLite, kill switch).
