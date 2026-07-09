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

**EPIC 2 (capture réseau brute) livré, audité, corrigé** :
- Troisième membre du workspace, `vitrail-capture-helper/` — binaire strictement mono-
  fonction (capture passive uniquement), reçoit les capacités `cap_net_raw,cap_net_admin`
  via `setcap` (divergence assumée par rapport au polkit-par-action de EPIC 7 : la capture
  est un processus continu, pas une action ponctuelle — documenté PLAN.md §6quater et
  `CONTRIBUTING.md`).
- Détection dynamique des interfaces actives (`pnet`, aucune interface en dur), un thread
  de capture AF_PACKET par interface, parsing 5-tuple + timestamp + volumétrie
  IPv4/IPv6/TCP/UDP, extraction SNI en clair depuis le ClientHello (`tls-parser`, aucun
  déchiffrement), détection de protocole best-effort (DNS/QUIC/TLS/HTTP), rate limiting
  token-bucket (2000 pps par défaut, drops agrégés et loggés périodiquement, jamais un log
  par paquet).
- `CaptureSubsystem` (`src-tauri/src/capture/`) implémente le trait `Subsystem` et remplace
  le stub "capture" dans la séquence du kill switch sans aucune autre modification de
  `killswitch/` — exactement la garantie que le squelette EPIC 7 devait fournir. Persistance
  provisoire en JSONL (`capture_events.jsonl`, même pattern 600 que `system_events.jsonl`).
- Audit séparé a fait corriger : diagnostics stderr du helper (dont l'avertissement de
  rate-limit) qui partaient vers `/dev/null` — relayés via `tracing::warn!` par un thread
  dédié ; documentation `setcap` manquante ajoutée dans `CONTRIBUTING.md`/`README.md` ;
  zéro test sur le parsing de paquets/ClientHello exposé à des données réseau non fiables —
  11 tests unitaires ajoutés (`packet.rs`, `tls_sni.rs`, y compris troncatures et bytes
  aléatoires, aucune panique) ; une ligne >120 caractères reformatée.
- `cargo build --workspace`/`clippy -D warnings`/`fmt --check`/`test --workspace` (12 tests,
  y compris les 100 cycles kill switch avec le vrai `CaptureSubsystem`) et `bun run build`
  tous verts, vérifiés indépendamment à chaque passe.

**EPIC 1 (attribution processus) livré, audité, corrigé** :
- Serveur gRPC `tonic` implémentant `ui.proto` réel d'OpenSnitch (`.proto` récupéré tel quel
  depuis `evilsocket/opensnitch`, copié en `src-tauri/proto/ui.proto`), socket dédié
  `$XDG_RUNTIME_DIR/vitrail/ui.sock` (dossier 700).
- **Découverte importante non anticipée** : `AskRule(Connection) returns (Rule)` est une RPC
  synchrone et bloquante — `opensnitchd` l'appelle pour chaque connexion sans règle connue et
  attend la réponse avant de laisser passer le paquet. Vitrail répond systématiquement
  "allow/once" (laissez-passer technique, jamais une décision de sécurité durable — conforme
  à "Vitrail ne décide jamais de blocage"), mais ça place `attribution/` sur un chemin
  critique réseau : un bug ici peut geler la connectivité de toute la machine tant que
  Vitrail est actif. Deux garde-fous ajoutés suite à l'audit : cache non-bloquant pour la
  résolution `.desktop` (I/O disque déplacée en `spawn_blocking`, jamais dans le chemin
  `AskRule`) + timeout gRPC 500ms + `AbnormalExitGuard` (Drop-based) qui restaure
  automatiquement l'adresse `ui_socket` d'origine du daemon même si le thread serveur meurt
  anormalement (pas seulement via l'arrêt normal `stop()`).
- Cache pid→exe sur clé composite `(pid, start_time)` (lu dans `/proc/<pid>/stat` champ 22),
  jamais le pid seul — évite toute confusion sur un pid recyclé. Résolution `.desktop`
  uniquement pour l'affichage, jamais pour la logique de corrélation.
- `vitrail-helper` gagne une troisième sous-commande allowlistée `opensnitch-set-socket`
  (édite `Server.Address` dans `/etc/opensnitchd/default-config.json`, redémarre le service,
  validation stricte du chemin socket), nouvelle action polkit dédiée
  `re.vitrail.helper.opensnitch`, code de sortie distinct si la config est écrite mais le
  restart échoue (état incohérent fichier/runtime signalé explicitement, pas un échec
  générique).
- `AttributionSubsystem` remplace le stub "attribution" dans le kill switch, la restauration
  ratée à la désactivation remonte bien comme divergence visible dans `SystemStatus`
  (`DeactivationReport.failed_steps`), pas un no-op silencieux.
- Tests d'intégration gRPC réels (vrai client `tonic` contre vrai serveur, socket temporaire
  dédié au test) + test de collision `(pid, start_time)`.
- `cargo build --workspace`/`clippy -D warnings`/`fmt --check`/`test --workspace` (14 tests
  attribution+killswitch) et `bun run build` tous verts, vérifiés indépendamment.

**EPIC 6 (storage SQLite WAL) livré, audité, corrigé** :
- `rusqlite` (feature `bundled`, aucune dépendance système), WAL, `vitrail.db` en
  `$XDG_DATA_HOME/vitrail/`. Migrations SQL embarquées, schéma `system_events`/
  `capture_events`/`attribution_state`/`flows`/`processes` (2 dernières vides, alimentées
  par EPIC 5), `flows_fts` (FTS5) créée mais pas encore branchée à une commande.
- Migre les 3 persistances JSONL provisoires posées en EPIC 7/2/1
  (`system_events.jsonl`/`capture_events.jsonl`/état socket attribution) vers de vraies
  tables, via une API `storage::` publique par domaine appelant — aucun accès SQLite direct
  en dehors de `storage/`, comportement observable des 3 domaines inchangé (mêmes tests
  verts : 100 cycles kill switch, tests gRPC attribution, tests capture).
- `purge_logs`/`purge_data`/`get_session_detail`/`delete_session`/`list_sessions`
  (`commands/settings.rs`) deviennent de vraies requêtes SQLite (au lieu de mocks
  plausibles) — `purge_data` retourne maintenant `Result<PurgeResult, String>` (une date
  fournie mais illisible est une erreur explicite, plus jamais une purge totale silencieuse
  — bug trouvé par l'audit et corrigé).
- Audit a fait corriger : bug de purge totale silencieuse sur date invalide (sévérité
  haute — risque de perte de données) ; régression TOCTOU sur les permissions de
  `vitrail.db` (même classe de bug déjà corrigée une fois sur les JSONL en EPIC 7,
  réintroduite différemment ici — fichier pré-créé en 600 avant ouverture, plus de
  `set_permissions` a posteriori) ; contention du Mutex storage partagé pendant `VACUUM`
  (DELETE et VACUUM sous des acquisitions de lock séparées, pas un seul verrou englobant qui
  aurait pu retarder capture/attribution) ; validation jour/mois insuffisante dans le
  parsing de date (`2026-02-30` rejeté proprement désormais).
- `cargo build --workspace`/`clippy -D warnings`/`fmt --check`/`test --workspace` (27 tests
  storage/killswitch/attribution + 11 capture-helper) et `bun run build` tous verts.

**EPIC 5 (moteur de corrélation) livré, audité, corrigé** :
- `correlation/` fusionne les événements `capture/` et `attribution/` par 5-tuple (protocole,
  IP/port src/dst — déjà transporté par le message `Connection` de `ui.proto`) avec une
  fenêtre tolérante de 5s : fusion immédiate si les 2 sources sont réunies pour une clé, sinon
  émission à expiration avec les sources déjà disponibles — jamais de doublon par 5-tuple.
- Visibilité (`FlowVisibility`) déterminée par une fonction pure testée sur les 16
  combinaisons de sources (capture/attribution/decryption/keylog) : `Fully` si contenu
  déchiffré (aucune source encore réelle, prêt pour EPIC 3/4), `Meta` si capture sans contenu,
  `Attrib` si attribution sans capture, `Unknown` sinon.
- Chaque `Flow` produit est persisté dans `storage::flows`/`flows_fts` (tables restées vides
  depuis EPIC 6, alimentées pour de vrai ici, recherche FTS5 réellement branchée) ET émis via
  l'événement Tauri `vitrail://flow`, qui remplace l'émetteur factice de dev (EPIC 8.4) —
  aucun changement côté frontend (`useTimelineFlows.ts` inchangé, même contrat d'événement).
  `commands/flows.rs` (`list_flows`/`get_flow_detail`/`search_flows`) sert désormais de vraies
  données au lieu de `mock_flows::flows()`.
- Câblage minimal ajouté dans `capture/`/`attribution/` (déjà durcis) : un canal `mpsc`
  supplémentaire (`try_send` non-bloquant) envoie chaque événement retenu vers
  `correlation/`, en plus de leur écriture `storage::events` existante — n'a rien changé à
  leur logique interne. Audit a vérifié en priorité que ça ne casse pas le chemin `AskRule`
  non-bloquant ni `AbnormalExitGuard` d'attribution (EPIC 1) : confirmé sans régression.
- Audit a trouvé et fait corriger un vrai risque de fusion silencieusement cassée sur IPv6 :
  `capture/` (Rust `IpAddr::to_string()`) et `attribution/` (chaîne brute du daemon Go
  `opensnitchd`) pouvaient représenter la même adresse différemment (IPv4-mappée
  `::ffff:a.b.c.d`, forme compressée/non compressée) — la clé de fusion `Eq`/`Hash` stricte
  aurait alors produit deux `Flow` distincts (`Meta` + `Attrib`) au lieu d'un seul fusionné.
  Corrigé par une normalisation via `std::net::IpAddr` des deux côtés avant construction du
  5-tuple (jamais un impact réseau, `AskRule` répond toujours "allow" indépendamment de la
  fusion). Une fonction >35 lignes également corrigée (extraction SQL en constante).
- `cargo build --workspace`/`clippy -D warnings`/`fmt --check`/`test --workspace` (42 tests
  lib + 11 capture-helper) et `bun run build` tous verts.

**EPIC 3 (SSLKEYLOGFILE) livré, audité, corrigé** :
- `keylog/` ne réinvente aucun parsing TLS/HTTP — délègue entièrement à `tshark` en
  sous-processus (`-o tls.keylog_file:...`, sortie `-T ek` JSON Lines), même rigueur que
  `vitrail-capture-helper` (SIGTERM→SIGKILL, stderr relayé via tracing). Détection honnête
  (`tshark -D`, pas une supposition) — état dégradé explicite si absent/sans permission,
  jamais un faux sentiment de couverture.
- Fichier de clés `$XDG_DATA_HOME/vitrail/tls_keylog.log` (600 dès l'ouverture, tronqué à
  chaque activation). Injection réversible : script wrapper + copie utilisateur des
  `.desktop` ciblés (jamais le fichier système), snapshot d'une surcharge préexistante avant
  écrasement pour restauration exacte à la désactivation.
- `list_keylog_apps`/`add_keylog_app`/`remove_keylog_app` (déjà dans le contrat IPC) sont
  désormais réels via `storage::keylog`.
- Première source de CONTENU réelle : `correlation/` étendue (`CorrelationEvent::Decryption`)
  pour remplir `request_headers`/`response_headers`/`body_preview`/`content_type`/
  `certificate` du `Flow` et produire `FlowVisibility::Fully` pour de vrai — modification
  ciblée du moteur EPIC 5 déjà audité 2 fois, sans régression sur le chemin `AskRule`.
- Audit a trouvé et fait corriger un vrai doublon (violation 5.2) : un fragment déchiffré
  arrivant APRÈS que capture+attribution aient déjà fermé/émis un `Flow` `Meta`/`Attrib`
  recréait un second `Flow` `Fully` pour la même connexion. Corrigé en enrichissant le flow
  déjà persisté (`storage::flows::find_recent_by_five_tuple`/`update_flow`) au lieu d'en
  émettre un second — ré-émet `vitrail://flow` avec la version mise à jour. Deux dépassements
  de taille (fonction/fichier) également corrigés.
- `cargo build --workspace`/`clippy -D warnings`/`fmt --check`/`test --workspace` (stable sur
  plusieurs runs consécutifs) et `bun run build` tous verts.

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

EPIC 4 — PolarProxy (décryptage TLS actif, CA locale, fail-open sur certificate pinning) —
dernier de l'ordre décidé, le plus risqué (MITM, CA système, jamais casser une app qui pin).
Après EPIC 4, tous les `StubSubsystem` du kill switch auront été remplacés par du réel.
En autonomie via le pattern build → audit → fix agents ([[vitrail-workflow]] côté mémoire).

**Points à surveiller** :
- Mutex storage partagé entre `killswitch/`/`capture/`/`attribution/`/`correlation/`/
  `keylog/` — couplage de robustesse identifié en EPIC 6, toujours pas bloquant mais la
  charge augmente à chaque EPIC.
- `find_recent_by_five_tuple` (fix doublon EPIC 3) cherche sur une fenêtre de 30s plus large
  que la fenêtre de corrélation (5s) et exclut le protocole du filtre — risque théorique
  d'enrichir le mauvais flow en cas de reconnexion rapide sur le même 4-tuple sous fort
  trafic, compromis assumé par l'audit, à garder en tête si un bug de mauvais matching
  apparaît en usage réel.
- Aucun filet de sécurité type `AbnormalExitGuard` si Vitrail crashe pendant qu'une app est
  injectée en keylog (EPIC 3) — un `.desktop` modifié pourrait rester en l'état si le process
  meurt anormalement avant `stop()`. Pas encore traité, à surveiller.

**Point à surveiller/valider avec Chris à l'usage réel** : `attribution/` répond
systématiquement "allow/once" à `AskRule` (laissez-passer technique, pas une décision de
sécurité — cf. ci-dessus), ce qui signifie qu'activer Vitrail avec `opensnitchd` installé
prend la main sur le filtrage réseau de la machine tant que Vitrail tourne (règles
persistantes déjà en place chez l'utilisateur non ré-appliquées automatiquement par ce
"allow once" — comportement cohérent avec le scope "Vitrail ne bloque rien" mais à tester
sur une vraie install `opensnitchd` avec des règles existantes avant la première activation
en dehors d'un environnement de dev).
