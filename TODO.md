# TODO — Vitrail

Plan détaillé complet : [`docs/EPICS.md`](docs/EPICS.md). Ce fichier est la vue résumée +
le backlog non structuré.

## Epics (résumé — statut détaillé dans docs/EPICS.md)

- [ ] EPIC 0 — Fondations projet (scaffold Tauri, CI, scripts, licence)
- [ ] EPIC 1 — Attribution processus (OpenSnitch)
- [ ] EPIC 2 — Capture réseau brute
- [ ] EPIC 3 — Décryptage TLS coopératif (SSLKEYLOGFILE)
- [ ] EPIC 4 — Décryptage TLS actif (PolarProxy, fail-open)
- [ ] EPIC 5 — Moteur de corrélation
- [ ] EPIC 6 — Stockage & requêtes
- [ ] EPIC 7 — Kill switch & réversibilité
- [ ] EPIC 8 — Contrat UI / IPC
- [ ] EPIC 9 — Sécurité & durcissement
- [ ] EPIC 10 — Packaging & distribution
- [ ] EPIC 11 — Documentation communautaire & visibilité

## Immédiat

- [ ] Discussion orchestration technique avec Chris (supervision des sous-processus,
      séquencement kill switch précis).
- [ ] Confirmer périmètre réseau exact (cf. STATE.md "Ouvert").
- [ ] Décider si/quand créer le repo GitHub distant public.
- [ ] Mockup UI via GLM 5.2 (Chris) à partir de `docs/UI_SPEC.md`, puis intégration
      frontend.

## Backlog (non priorisé)

- Portage éventuel du blocage interactif (au-delà de la simple consommation des décisions
  OpenSnitch) — explicitement hors scope v1.
- Dashboard distant consultable depuis un autre appareil — hors scope v1, surface réseau
  supplémentaire à évaluer séparément si jamais voulu.
- Publication communautaire (Reddit, HN) — action de Chris, texte de présentation à
  préparer (EPIC 11.4).
