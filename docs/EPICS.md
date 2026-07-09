# Vitrail — Epics & Stories

Plan d'implémentation complet. Chaque epic correspond à un domaine ou une préoccupation
transverse définie dans `PLAN.md`. Les stories sont écrites pour être directement
actionnables (critère de fin explicite), pas des titres vagues.

Statut : `TODO` / `IN PROGRESS` / `DONE` / `BLOCKED`. Mis à jour au fil de l'implémentation,
reflété en résumé dans `TODO.md` à la racine.

---

## EPIC 0 — Fondations projet
Objectif : un dépôt public sain, buildable, avec les scripts et docs obligatoires.

- **0.1** Scaffold Tauri (`bun create tauri-app`), structure `src-tauri/src/` par domaine
  (dossiers vides avec `mod.rs` stub), frontend placeholder minimal (une page "Vitrail —
  en construction").
- **0.2** `start.sh` / `stop.sh` / `restart.sh` fonctionnels (gestion PID, reset logs),
  cohérents avec le mode dev Tauri (`bun run tauri dev`) et un mode prod packagé.
- **0.3** CI GitHub Actions minimal : `cargo check`, `cargo clippy -- -D warnings`,
  `cargo fmt --check`, build frontend (`bun run build`).
- **0.4** `LICENSE` (MIT), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (repo public, communauté
  visée explicitement par Chris).
- **0.5** `.env.example`, `.gitignore` Rust+Tauri+Bun complet (target/, node_modules/,
  dist/, *.db, logs/).

## EPIC 1 — Attribution processus (OpenSnitch)
Objectif : savoir, pour chaque connexion, quel processus l'a ouverte.

- **1.1** Détection au démarrage : OpenSnitch est-il installé/lancé ? Si non, état dégradé
  explicite dans l'UI (pas de crash, pas de faux positif silencieux).
- **1.2** Client gRPC vers le daemon OpenSnitch (ou lecture de son socket d'événements
  selon la version packagée) — décodage en `AttributionEvent`.
- **1.3** Cache pid→exe_path avec gestion du recyclage de pid (un pid réutilisé après la
  mort d'un process ne doit jamais attribuer une connexion au mauvais binaire).
- **1.4** Résolution du chemin binaire vers un nom d'application lisible (heuristique :
  `.desktop` associé, sinon nom du binaire brut) — pour affichage humain uniquement, jamais
  pour la logique de corrélation (qui reste sur le pid/exe_path exact).
- **1.5** Tests : simulation d'événements OpenSnitch rejoués depuis un fixture, vérification
  du cache pid et de la résolution.

## EPIC 2 — Capture réseau brute
Objectif : visibilité de base indépendante du TLS, base commune de vérité des flux.

- **2.1** Capture AF_PACKET sur les interfaces actives (détection dynamique, pas
  d'interface hardcodée).
- **2.2** Parsing 5-tuple + volumétrie + timestamps, écriture continue en `capture/`.
- **2.3** Extraction du SNI depuis le ClientHello TLS en clair (pas de déchiffrement ici,
  juste le champ visible du handshake) — donne un nom de domaine même sans décryptage.
- **2.4** Détection de protocole best-effort (DNS, QUIC/HTTP3, TLS, plaintext HTTP) pour
  affichage et pour orienter la corrélation.
- **2.5** Gestion de charge : throttling/sampling configurable si le volume de paquets
  dépasse un seuil (éviter de saturer CPU/disque sur une machine sous forte charge réseau).

## EPIC 3 — Décryptage TLS coopératif (SSLKEYLOGFILE)
Objectif : déchiffrement propre, sans interception, pour les apps qui exportent leurs clés.

- **3.1** Génération et gestion du fichier de clés (emplacement, permissions restrictives
  600, rotation/purge).
- **3.2** Injection de la variable d'environnement pour les apps ciblées : wrapper de
  lancement pour les `.desktop` (Firefox, Chrome/Chromium, VSCode/Electron), export
  utilisateur pour les sessions shell.
- **3.3** Tail du fichier de clés en continu, association aux flux capturés (`capture/`)
  par correspondance de session TLS.
- **3.4** Intégration `tshark` en sous-processus pointé sur le pcap live + le fichier de
  clés, extraction du texte en clair (HTTP/1.1, HTTP/2 headers+body, HTTP/3 si supporté).
- **3.5** UI de statut : quelles apps sont actuellement couvertes par le keylog, lesquelles
  ne le sont pas (documentation claire de la limite, pas de faux sentiment de couverture
  totale).

## EPIC 4 — Décryptage TLS actif (PolarProxy)
Objectif : couverture des apps qui n'exportent pas leurs clés, avec repli automatique sur
le pinning.

- **4.1** Génération de la CA locale dédiée Vitrail (jamais réutiliser/modifier une CA
  existante sur le système).
- **4.2** Gestion du cycle de vie du sous-processus PolarProxy : démarrage, configuration
  du mode fail-open, arrêt propre, redémarrage sur crash.
- **4.3** Règle nftables dédiée (`VITRAIL_REDIRECT`) redirigeant le trafic 443/80 vers
  PolarProxy — chaîne isolée, jamais de règle en dehors de ce nom.
- **4.4** Lecture de la sortie PolarProxy (PCAP live ou export structuré) → production de
  `DecryptedFlow` ou `PinningDetected`.
- **4.5** Liste d'exclusion utilisateur : domaines/process jamais interceptés même en
  MITM (ex. banque), appliquée en amont de la redirection nftables, pas juste côté
  affichage.
- **4.6** Tests d'intégration : vérifier concrètement qu'une app à pinning connu (ex. un
  client mobile via scrcpy/waydroid si dispo, sinon un binaire de test avec pinning
  simulé) continue de fonctionner avec fail-open actif.

## EPIC 5 — Moteur de corrélation
Objectif : fusionner attribution + capture + décryptage(x2) en une timeline unique et
cohérente.

- **5.1** Modèle de fusion par 5-tuple + fenêtre temporelle tolérante (les timestamps des
  différentes sources ne sont jamais parfaitement synchrones).
- **5.2** Résolution de conflits : une même connexion vue par plusieurs sources doit
  produire **un seul** enregistrement final, pas un doublon par source.
- **5.3** Détermination du niveau de visibilité par flux : `FullyDecrypted` /
  `MetadataOnly` / `AttributedOnly` / `Unknown` — logique explicite et testée pour chaque
  combinaison de sources disponibles.
- **5.4** Emission d'événements temps réel vers `storage/` et vers l'UI (streaming, pas de
  polling côté frontend).
- **5.5** Tests de fusion avec fixtures combinant les 4 sources dans des ordres et délais
  variés.

## EPIC 6 — Stockage & requêtes
Objectif : persistance locale, recherche performante, rétention maîtrisée.

- **6.1** Schéma SQLite (`flows`, `processes`, `system_events`), migrations versionnées.
- **6.2** Mode WAL, index sur (timestamp, pid, domaine, port) pour les filtres UI courants.
- **6.3** Politique de rétention configurable (défaut 30 jours), tâche de purge planifiée.
- **6.4** Recherche plein texte sur le contenu déchiffré stocké (body preview, headers) —
  évaluer FTS5 SQLite natif.
- **6.5** Export (JSON/CSV) d'une plage de données pour analyse externe ou rapport.

## EPIC 7 — Kill switch & réversibilité
Objectif : la garantie centrale du projet — activer/désactiver sans résidu.

- **7.1** Snapshot d'état système avant activation (règles nftables existantes, CA de
  confiance, daemons actifs) — sérialisé et horodaté dans `system_events`.
- **7.2** Séquence d'activation orchestrée (ordre : CA → nftables → PolarProxy →
  attribution → capture → keylog), chaque étape loggée avec succès/échec explicite.
- **7.3** Séquence de désactivation orchestrée en ordre inverse strict, avec retry/timeout
  par étape (ne jamais rester bloqué en état intermédiaire).
- **7.4** Diff de vérification post-désactivation : comparaison snapshot pré/post, rapport
  affiché à l'utilisateur, toute divergence = alerte visible (pas juste un log).
- **7.5** Bouton d'urgence ("tout couper immédiatement") distinct du arrêt normal — best
  effort, priorité à la restauration réseau même si le nettoyage est incomplet (avec
  rapport des étapes non confirmées).
- **7.6** Tests de bascule répétée (100x start/stop) pour détecter toute fuite d'état.

## EPIC 8 — Contrat UI / IPC
Objectif : tout ce que le frontend (mockup GLM à intégrer ensuite) doit pouvoir afficher et
piloter. Détail exhaustif dans `docs/UI_SPEC.md` — ici, la traduction en commandes Tauri.

- **8.1** Commandes de lecture (`get_dashboard_summary`, `list_flows`, `get_flow_detail`,
  `list_processes`, `get_process_detail`, `list_destinations`, `search_flows`).
- **8.2** Commandes de contrôle (`activate_vitrail`, `deactivate_vitrail`,
  `emergency_stop`, `get_system_status`, `verify_teardown`).
- **8.3** Commandes de configuration (`get_settings`, `update_settings`, `add_exclusion`,
  `remove_exclusion`, `rotate_ca`, `export_config`, `import_config`).
- **8.4** Canal d'événements temps réel (Tauri events) pour le streaming de la timeline
  sans polling.
- **8.5** Contrat de types partagé (génération de types TS depuis les structs Rust, éviter
  toute divergence manuelle front/back).

## EPIC 9 — Sécurité & durcissement
Objectif : l'outil qui inspecte le trafic ne doit pas devenir lui-même une surface
d'attaque.

- **9.1** Permissions fichiers strictes sur la CA privée, le fichier de clés TLS, la base
  SQLite (600, propriétaire uniquement).
- **9.2** Séparation des privilèges : uniquement les opérations nftables/CA nécessitent une
  élévation (polkit ou sudo ciblé), le reste de l'app tourne en utilisateur normal.
- **9.3** Audit des dépendances (`cargo audit`, `bun audit`) en CI.
- **9.4** Revue explicite : que se passe-t-il si Vitrail lui-même crashe pendant qu'il est
  actif ? (garantie de non-blocage réseau permanent — watchdog ou fail-safe nftables avec
  règle de timeout).
- **9.5** Documentation claire des limites de sécurité (ce que Vitrail voit, ce qu'il ne
  voit pas, ce qu'un attaquant local avec accès à la CA pourrait faire).

## EPIC 10 — Packaging & distribution
Objectif : installable facilement par la communauté visée.

- **10.1** Build AppImage (pattern déjà validé sur Aegis).
- **10.2** Script d'installation utilisateur sans root pour la partie applicative,
  instructions claires pour la partie qui nécessite des privilèges (nftables, CA).
- **10.3** Paquet AUR (cohérent avec l'usage Arch de Chris, mais généralisable).
- **10.4** Vérification de compatibilité sur au moins deux familles de distros (Arch +
  Debian/Ubuntu) avant toute release publique.

## EPIC 11 — Documentation communautaire & visibilité
Objectif : projet public qui a une chance d'être découvert et utilisé, pas juste déposé.

- **11.1** README avec positionnement clair face à Sniffnet/Wireshark/OpenSnitch (tableau
  comparatif honnête, pas de survente).
- **11.2** GIF/démo courte du flux d'usage (capture d'écran une fois l'UI intégrée).
- **11.3** `CONTRIBUTING.md` détaillé (setup dev, structure par domaine, comment ajouter un
  domaine).
- **11.4** Publication ciblée post-release (r/linux, r/archlinux, Hacker News "Show HN") —
  action de Chris, pas de l'agent, mais à préparer (texte de présentation prêt).
- **11.5** Topics GitHub pertinents (`network-monitoring`, `tls`, `linux`, `privacy`,
  `ebpf`) pour la découvrabilité passive.
