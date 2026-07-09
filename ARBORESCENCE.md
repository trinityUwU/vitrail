# ARBORESCENCE — Vitrail

Une ligne par fichier/groupe. Régénérer après tout ajout/suppression significatif.
`node_modules/`, `target/`, `dist/`, `src-tauri/gen/schemas/` (généré) omis.

```
.env.example              — variables d'environnement requises (template)
.gitignore                — exclusions Rust/Tauri/Bun + données sensibles
ARCHITECTURE.md            — carte des domaines, frontières de module, décisions figées
ARBORESCENCE.md             — ce fichier
CODE_OF_CONDUCT.md         — règles de contribution liées à la confiance/sécurité du projet
CONTRIBUTING.md            — setup dev, invariants non négociables, style de code
LICENSE                    — MIT
README.md                  — présentation publique, positionnement face aux outils existants
STATE.md                   — état courant du projet, décisions actées, ouvert, prochaine étape
TODO.md                    — vue résumée des epics + backlog non structuré
restart.sh / start.sh / stop.sh — gestion du cycle de vie dev, PID + reset logs
package.json / bun.lock / tsconfig*.json / vite.config.ts / index.html — config Bun/Vite/TS
public/                     — assets statiques Tauri par défaut (icônes vite/tauri)
Cargo.toml                  — workspace racine, membres [src-tauri, vitrail-helper]

vitrail-helper/             — binaire privilégié minimal (EPIC 7/9), invoqué via pkexec
  Cargo.toml                — aucune dépendance Tauri, crate isolé
  src/main.rs                — allowlist stricte nft-apply/nft-flush, Command à arguments fixes
  re.vitrail.helper.policy   — règle polkit, chemin binaire attendu /usr/local/bin/vitrail-helper
                                (à ajuster au vrai chemin d'installation en EPIC 10)

vitrail-capture-helper/     — binaire mono-fonction (EPIC 2), setcap cap_net_raw/cap_net_admin
  Cargo.toml                — deps pnet/tls-parser/signal-hook, aucune dépendance Tauri
  src/main.rs                — détection interfaces actives, spawn 1 thread/interface, SIGTERM
  src/capture_thread.rs       — boucle de capture AF_PACKET par interface
  src/packet.rs                 — parsing Ethernet/IPv4/IPv6/TCP/UDP → 5-tuple, détection
                                   protocole best-effort (DNS/QUIC/TLS/HTTP), testé (troncatures)
  src/tls_sni.rs                 — extraction SNI depuis ClientHello en clair, aucun
                                    déchiffrement, testé (bytes malformés/aléatoires)
  src/rate_limiter.rs             — token-bucket 2000 pps par défaut, drops agrégés/loggés
  src/output.rs                    — JSON Lines sur stdout, flush par ligne

docs/
  EPICS.md                  — plan d'implémentation détaillé (12 epics, stories actionnables)
  PLAN.md                   — architecture technique complète, état de l'art, réversibilité
  UI_SPEC.md                 — spécification fonctionnelle exhaustive de l'UI (source du portage)
  Mockup.html                — prototype statique GLM 5.2, référence figée, ne pas modifier
  MOCKUP_REVIEW.md            — revue du mockup, 3 défauts identifiés et corrigés au portage

src-tauri/                  — backend Tauri (Rust)
  Cargo.toml / Cargo.lock / build.rs / tauri.conf.json / capabilities/ — config Tauri
  icons/                     — icônes app (défaut template, à remplacer)
  src/
    main.rs / lib.rs         — point d'entrée, enregistrement des commandes
    attribution/mod.rs        — stub EPIC 1 : serveur ui.proto (tonic), cache pid→exe (non implémenté)
    capture/                   — EPIC 2 : capture AF_PACKET réelle (livrée, auditée)
      mod.rs                    — déclaration du sous-module
      subsystem.rs               — CaptureSubsystem (trait Subsystem), spawn/SIGTERM→SIGKILL,
                                    threads lecteurs stdout+stderr (diagnostics relayés tracing)
      events.rs                   — CapturedPacket, persistance JSONL 600 (capture_events.jsonl)
    decryption/mod.rs          — stub EPIC 4 : orchestration PolarProxy, fail-open (non implémenté)
    keylog/mod.rs               — stub EPIC 3 : pipeline SSLKEYLOGFILE (non implémenté)
    correlation/mod.rs          — stub EPIC 5 : fusion des sources en timeline (non implémenté)
    storage/mod.rs               — stub EPIC 6 : SQLite WAL, rétention (non implémenté)
    killswitch/                   — EPIC 7 : squelette d'orchestration réel (livré, audité)
      mod.rs                       — KillSwitchState partagé, API publique, snapshot pré-activation
      subsystem.rs                  — trait Subsystem + StubSubsystem (CA/PolarProxy/attribution/
                                       capture/keylog — remplacés un par un par les EPICs réels)
      nftables.rs                    — trait NftablesBackend, SystemNftablesBackend (pkexec réel)
                                        + FakeNftablesBackend (tests, jamais de process réel)
      snapshot.rs                     — SystemSnapshot horodaté, JSONL append-only 600
                                         ($XDG_DATA_HOME/vitrail/system_events.jsonl)
      sequence.rs                      — activate() ordre strict + arrêt au 1er échec,
                                          deactivate() ordre inverse + retry/timeout par étape
      verify.rs                         — diff pré/post, TeardownReport, cas "pas d'activation"
      emergency.rs                       — arrêt d'urgence distinct, best-effort, hors séquence
      tests.rs                            — test 7.6 : 100 cycles, FakeNftablesBackend uniquement
    shared/mod.rs                  — types communs (SystemStatus/SubsystemStatus/TeardownReport,
                                      frontière domaine respectée), config, logging tracing
    commands/                       — EPIC 8, seule vraie logique de cette passe
      mod.rs                        — déclaration des sous-modules
      types.rs                      — structs serde partagées (contrat IPC), inclut
                                       HttpHeader/CertificateInfo/CorrelationSource/AlertEvent/
                                       SearchCriteria/SavedQuery/PurgeResult/SessionDetail
      mock_data.rs / mock_flows.rs   — données de démo (flows séparés pour rester <500 lignes)
      dashboard.rs / flows.rs / processes.rs / destinations.rs / killswitch.rs / settings.rs /
      alerts.rs / search.rs          — commandes #[tauri::command] (contrat complet cf.
                                       docs/EPICS.md 8.1-8.3), mocks commentés EPIC réel

src/                        — frontend React/TypeScript (Vite)
  main.tsx / App.tsx / vite-env.d.ts — bootstrap, routage entre écrans, providers
                                        (Toast/KillSwitch/Exclusions)
  dashboard/                 — écran 1 (UI_SPEC) : vue d'ensemble, métriques, top listes
  timeline/                  — écran 2 : flux temps réel, filtres, table
  processes/                 — écran 3 : liste + détail par processus (exclusion centralisée)
  destinations/              — écran 4 : liste + détail par destination (exclusion + tag)
  inspector/                  — écran 5 : détail d'un flux — contenu/certificat/sources lus
                                depuis le contrat `Flow` (plus rien fabriqué en JSX),
                                copie/export réels (inspector-actions.ts)
  search/                     — écran 6 : recherche avancée + requêtes sauvegardées
                                (useSavedQueries.ts, search-utils.ts)
  alerts/                     — écran 7 : CRUD règles d'alerte (AlertRuleForm.tsx) + historique
                                réel des déclenchements (useAlertEvents.ts)
  killswitch/                  — écran 8 : panneau kill switch, sous-systèmes, arrêt d'urgence
  settings/                    — écran 9 : paramètres, 7 onglets (CA, réseau, exclusions,
                                  rétention, keylog, notifications, à propos) — notifications et
                                  keylog persistés (useKeylogApps.ts), export/import réels
                                  (config-actions.ts), purge réelle (RetentionTab)
  privacy/                     — écran 10 : confidentialité & gouvernance des données
  logs/                        — écran 11 : journal système, purge/copie/export réels
                                (log-actions.ts)
  history/                     — écran 12 : sessions passées, détail de session
                                (SessionDetailView.tsx, useSessionDetail.ts), rapport téléchargé
                                (history-report.ts)
  onboarding/                   — écran 13 : parcours guidé première installation
  shared/
    components/                 — Badge, Button, EmptyState, ExclusionsProvider,
                                   KillSwitchProvider, Table, ToastProvider, Toggle,
                                   VisibilityBadge
    hooks/                       — useAlertBadge, useExclusionsState (Context partagé —
                                    corrige la désync exclusions entre écrans),
                                    useKillSwitchState, useToast
    layout/                      — Sidebar, Topbar, DegradationBanner, nav-items
    lib/                          — format-utils, logger, types, visibility, vitrail-api
                                    (couche d'accès IPC — invoke() vers commands/, jamais de
                                    données en dur dans un composant)
    styles/                       — tokens.css (variables portées du mockup), base/layout/
                                    components.css
```

## Statut d'implémentation

- **EPIC 0 (fondations)** : scaffold Tauri fait, CI/CONTRIBUTING/LICENSE en place (CI 0.3
  restante).
- **EPIC 8 (contrat UI/IPC)** : frontend modulaire complet, contrat `Flow` exhaustif (headers,
  corps, certificat, sources de corrélation, IP/port source), toutes les commandes listées
  dans `docs/EPICS.md` 8.1-8.3 implémentées et appelées (plus aucun bouton factice hors
  "Régénérer la CA" qui appelle déjà `rotate_ca` — seule la partie réellement système reste
  à faire en EPIC 4/9), streaming temps réel simulé (émetteur factice documenté comme
  temporaire).
- **EPIC 7 (kill switch)** : squelette d'orchestration réel livré, audité, corrigé —
  7.1 à 7.6 couverts. `vitrail-helper` + polkit posent la base d'élévation de privilèges
  pour EPIC 9.2. Deux étapes de la séquence sont maintenant réelles (nftables, capture),
  les autres (CA/PolarProxy/attribution/keylog) restent `StubSubsystem`.
- **EPIC 2 (capture)** : réel, livré et audité — `vitrail-capture-helper` (pnet + tls-parser,
  setcap) + `CaptureSubsystem` branché sans aucune modification de `killswitch/` au-delà du
  remplacement du stub. 5-tuple/timestamp/volumétrie/SNI/protocole best-effort/rate limiting
  tous couverts, 11 tests unitaires sur le parsing (donnée réseau non fiable).
- **EPICs 1,3,4,5,6 (attribution/keylog/décryptage/corrélation/storage réels)** :
  non commencés — modules stubs uniquement (`mod.rs` = un commentaire de responsabilité),
  branchés dans `killswitch/subsystem.rs` comme `StubSubsystem` en attendant. EPIC 6 a gagné
  deux stories (6.6 purge, 6.7 détail/suppression session) pour couvrir les commandes
  mockées en attente de vraie persistance SQLite, plus la migration des JSONL EPIC 7/2.
