# STATE — Vitrail

## Où on en est (2026-07-09)

Repo public poussé : https://github.com/trinityUwU/vitrail. Démarrage de l'implémentation
réelle des EPICs 1-7, dans l'ordre décidé (7 → 2 → 1 → 6 → 5 → 3 → 4).

**EPIC 7 (squelette kill switch) livré, audité, corrigé** :
- Workspace Cargo (`/Cargo.toml`) avec un second membre `vitrail-helper/` — binaire
  privilégié minimal (allowlist stricte `nft-apply`/`nft-flush`, aucune interpolation
  shell), invoqué via `pkexec` depuis l'app, policy polkit `re.vitrail.helper.policy`
  cohérente avec le chemin en dur côté Rust (`/usr/local/bin/vitrail-helper`, à ajuster
  au packaging EPIC 10).
- Domaine `killswitch/` réel : trait `Subsystem` (stub pour CA/PolarProxy/attribution/
  capture/keylog — chaque EPIC branchera sa vraie implémentation sans toucher
  l'orchestration), trait `NftablesBackend` (`SystemNftablesBackend` réel +
  `FakeNftablesBackend` pour les tests), snapshot système horodaté persisté en JSONL
  append-only (`$XDG_DATA_HOME/vitrail/system_events.jsonl`, 600 dès l'ouverture),
  séquence d'activation stricte CA→nftables→PolarProxy→attribution→capture→keylog
  (arrêt au premier échec), séquence de désactivation en ordre inverse avec retry
  (3 tentatives, timeout 5s par étape) et best-effort (jamais bloquée), diff de
  vérification pré/post avec divergences lisibles, arrêt d'urgence distinct
  (flush nftables prioritaire, best-effort, hors séquence ordonnée).
- Les 5 commandes IPC du panneau kill switch (`activate_vitrail`, `deactivate_vitrail`,
  `emergency_stop`, `get_system_status`, `verify_teardown`) appellent désormais la vraie
  logique via `tauri::State<KillSwitchState>` — contrat IPC/types TS inchangé.
- Test des 100 cycles start/stop (7.6) vert, `FakeNftablesBackend` uniquement, jamais de
  `pkexec` réel en test.
- Audit séparé a trouvé et fait corriger : une inversion de frontière de domaine
  (`killswitch` importait des types depuis `commands` — les trois types `SystemStatus`/
  `SubsystemStatus`/`TeardownReport` vivent maintenant dans `shared/`, `commands/types.rs`
  les ré-exporte), le retry/timeout manquant en 7.3, une fonction >35 lignes, un TOCTOU
  mineur sur les permissions du JSONL, et un faux rapport "propre" si `verify_teardown()`
  est appelé sans activation préalable.
- `cargo build --workspace`/`clippy --workspace -- -D warnings`/`fmt --check`/
  `test --workspace` et `bun run build` tous verts, vérifiés indépendamment après chaque
  passe (build agent + audit agent + fix agent).

## Historique (scaffold initial)

Scaffold Tauri complet et cohérent. Le repo contenait déjà à ce stade :
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
- Élévation de privilèges : polkit par action via `vitrail-helper`, zéro daemon root
  persistant (`docs/PLAN.md` §6bis). `system_events` en JSONL transitoire jusqu'à EPIC 6
  (`docs/PLAN.md` §6ter).

## Ouvert

- **Polices exactes** (`DM Serif Display`, `Outfit`) : variables CSS avec fallback système,
  pas encore self-hébergées (`@fontsource` ou fichiers fournis par Chris — décision à
  prendre).
- Confirmation explicite du périmètre réseau ("accessible depuis le réseau" — interprété
  comme aucune exposition réseau, cohérent avec les choix déjà faits, mais jamais confirmé
  mot pour mot par Chris).
- Icônes app Tauri : encore celles du template par défaut.
- Chemin en dur `/usr/local/bin/vitrail-helper` (Rust + `.policy` polkit) à ajuster au vrai
  chemin d'installation choisi en EPIC 10.

## Prochaine étape

EPIC 2 — capture réseau brute (AF_PACKET), en autonomie via le pattern build → audit → fix
agents ([[vitrail-workflow]] côté mémoire). EPIC 7 reste un squelette tant que les domaines
qu'il orchestre (CA/PolarProxy/attribution/capture/keylog) n'ont pas de vraie implémentation
— chaque EPIC suivant remplace son `StubSubsystem` par une implémentation réelle du trait
`Subsystem` sans toucher à l'orchestration.
