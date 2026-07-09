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
    attribution/mod.rs        — stub EPIC 1 : client OpenSnitch, cache pid→exe (non implémenté)
    capture/mod.rs             — stub EPIC 2 : capture AF_PACKET, 5-tuple, SNI (non implémenté)
    decryption/mod.rs          — stub EPIC 4 : orchestration PolarProxy, fail-open (non implémenté)
    keylog/mod.rs               — stub EPIC 3 : pipeline SSLKEYLOGFILE (non implémenté)
    correlation/mod.rs          — stub EPIC 5 : fusion des sources en timeline (non implémenté)
    storage/mod.rs               — stub EPIC 6 : SQLite WAL, rétention (non implémenté)
    killswitch/mod.rs             — stub EPIC 7 : cycle de vie orchestré, snapshot/diff (non implémenté)
    shared/mod.rs                  — stub : types communs, config, logging (non implémenté)
    commands/                       — EPIC 8, seule vraie logique de cette passe
      mod.rs                        — déclaration des sous-modules
      types.rs                      — structs serde partagées (contrat IPC)
      mock_data.rs                   — source unique des données de démo (portées du mockup)
      dashboard.rs / flows.rs / processes.rs / destinations.rs / killswitch.rs / settings.rs
                                     — 24 commandes #[tauri::command], retournent des mocks
                                       explicitement commentés (EPIC réel qui les remplacera)

src/                        — frontend React/TypeScript (Vite)
  main.tsx / App.tsx / vite-env.d.ts — bootstrap, routage entre écrans
  dashboard/                 — écran 1 (UI_SPEC) : vue d'ensemble, métriques, top listes
  timeline/                  — écran 2 : flux temps réel, filtres, table
  processes/                 — écran 3 : liste + détail par processus
  destinations/              — écran 4 : liste + détail par destination
  inspector/                  — écran 5 : détail d'un flux (contenu déchiffré/métadonnées)
  search/                     — écran 6 : recherche avancée
  alerts/                     — écran 7 : règles d'alerte + déclenchements
  killswitch/                  — écran 8 : panneau kill switch, sous-systèmes, arrêt d'urgence
  settings/                    — écran 9 : paramètres, 7 onglets (CA, réseau, exclusions,
                                  rétention, keylog, notifications, à propos)
  privacy/                     — écran 10 : confidentialité & gouvernance des données
  logs/                        — écran 11 : journal système
  history/                     — écran 12 : sessions passées
  onboarding/                   — écran 13 : parcours guidé première installation
  shared/
    components/                 — Badge, Button, EmptyState, KillSwitchProvider, Table,
                                   ToastProvider, Toggle, VisibilityBadge
    hooks/                       — useAlertBadge, useKillSwitchState, useToast
    layout/                      — Sidebar, Topbar, DegradationBanner, nav-items
    lib/                          — format-utils, logger, types, visibility, vitrail-api
                                    (couche d'accès IPC — invoke() vers commands/, jamais de
                                    données en dur dans un composant)
    styles/                       — tokens.css (variables portées du mockup), base/layout/
                                    components.css
```

## Statut d'implémentation

- **EPIC 0 (fondations)** : scaffold Tauri fait, CI/CONTRIBUTING/LICENSE en place.
- **EPIC 8 (contrat UI/IPC)** : frontend modulaire complet, 24 commandes IPC mockées,
  streaming temps réel simulé (émetteur factice documenté comme temporaire).
- **EPICs 1-7 (attribution/capture/décryptage/keylog/corrélation/storage/killswitch réels)** :
  non commencés — modules stubs uniquement (`mod.rs` = un commentaire de responsabilité).
