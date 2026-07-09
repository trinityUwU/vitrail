# Architecture — Vitrail

Raisonnement complet dans `docs/PLAN.md`. Ce fichier est le contrat anti-dérive : chaque
dossier a une définition unique, non ambiguë.

## Domaines (`src-tauri/src/`)

| Dossier | Responsabilité unique | Ne fait jamais |
|---|---|---|
| `attribution/` | Consommer les événements OpenSnitch (pid ↔ connexion) | Ne capture pas de paquets, ne décide pas de blocage |
| `capture/` | Capture réseau brute (AF_PACKET), 5-tuple, SNI en clair | Ne déchiffre jamais de contenu TLS |
| `decryption/` | Orchestrer PolarProxy, produire du contenu déchiffré + signaux de pinning | Ne fait pas d'attribution processus |
| `keylog/` | Pipeline `SSLKEYLOGFILE` (injection + tail + tshark) | N'intercepte jamais activement (pas de MITM) |
| `correlation/` | Fusionner les 4 sources en une timeline unique | Ne capture ni ne déchiffre rien lui-même |
| `storage/` | Persistance SQLite WAL, rétention, recherche | Ne contient aucune logique métier de corrélation |
| `killswitch/` | Cycle de vie orchestré de tous les sous-systèmes, snapshot/diff | Ne touche jamais DNS ni config proxy système |
| `shared/` | Types communs, config, logging (`tracing`) | Pas de logique métier |
| `commands/` | Seule surface IPC exposée au frontend | Pas de logique métier — agrégation/délégation uniquement |

## Frontière stricte

Un domaine ne référence jamais les internes d'un autre : uniquement ses types publics
exportés via `shared/` ou via le canal d'événements de `correlation/`. Tout accès UI passe
par `commands/` — jamais d'appel direct d'un domaine depuis le frontend.

## Frontend (`src/`)

Non scaffoldé en détail à ce stade (cf. `docs/UI_SPEC.md` — spécification fonctionnelle
en attente d'un mockup visuel produit séparément). Structure prévue : vertical slice par
écran listé dans `UI_SPEC.md`, logique métier hors composants (hooks), design system à
définir au moment de l'intégration du mockup.

## Décisions figées

- Aucune exposition réseau (pas de serveur HTTP/API) — IPC Tauri uniquement. Cf.
  `docs/PLAN.md` section 7 pour la clarification de périmètre à confirmer avec Chris.
- Une seule chaîne nftables nommée, jamais de règle en dehors.
- SQLite WAL, pas de serveur DB externe.
- Rust + Tauri, pas de backend Node/Express séparé (cohérence avec Aegis/NULLNODE).

## Historique des décisions d'architecture

Ce fichier suit les faits présents, pas l'historique. Les décisions et leur justification
détaillée vivent dans `docs/PLAN.md`.
