# TODO — Vitrail

Plan détaillé complet : [`docs/EPICS.md`](docs/EPICS.md). Ce fichier est la vue résumée +
le backlog non structuré.

## Epics (résumé — statut détaillé dans docs/EPICS.md)

- [x] EPIC 0 — Fondations projet (scaffold Tauri, CI, scripts, licence) — CI (0.3) restant
- [x] EPIC 1 — Attribution processus (OpenSnitch) — serveur gRPC ui.proto (tonic), cache
      pid/start_time, AskRule non-bloquant + panic guard de restauration
- [x] EPIC 2 — Capture réseau brute — pnet + tls-parser, vitrail-capture-helper (setcap
      cap_net_raw/cap_net_admin), CaptureSubsystem branché dans le kill switch
- [ ] EPIC 3 — Décryptage TLS coopératif (SSLKEYLOGFILE)
- [ ] EPIC 4 — Décryptage TLS actif (PolarProxy, fail-open)
- [x] EPIC 5 — Moteur de corrélation — fusion capture+attribution par 5-tuple/fenêtre 5s,
      visibilité Meta/Attrib réelle (Fully/Unknown prêts pour EPIC 3/4), flows/flows_fts
      alimentées, timeline temps réel réelle (remplace l'émetteur factice EPIC 8.4)
- [x] EPIC 6 — Stockage & requêtes — SQLite WAL (rusqlite bundled), migre les 3 JSONL
      provisoires EPIC 7/2/1, purge/rétention/sessions réelles ; flows/processes/FTS5
      créées vides (alimentées en EPIC 5)
- [~] EPIC 7 — Kill switch & réversibilité — squelette d'orchestration livré et audité
      (7.1-7.6 couverts avec sous-systèmes stub ; CA/PolarProxy/attribution/capture/keylog
      réels arrivent avec leurs EPICs respectifs)
- [~] EPIC 8 — Contrat UI / IPC — frontend + commandes complètes livrées et auditées
      (contrat Flow complet, exclusions centralisées, CRUD alertes, recherche sauvegardée,
      purge, tag, historique session, notifications/keylog persistés), streaming réel (8.4)
      désormais réel (EPIC 5, événement vitrail://flow), contrat de types généré (8.5)
      toujours manuel
- [ ] EPIC 9 — Sécurité & durcissement
- [ ] EPIC 10 — Packaging & distribution
- [ ] EPIC 11 — Documentation communautaire & visibilité

## Immédiat

- [x] Repo GitHub public créé et poussé : https://github.com/trinityUwU/vitrail.
- [x] EPIC 7 (squelette kill switch) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 2 (capture réseau brute) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 1 (attribution processus) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 6 (storage SQLite) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 5 (corrélation timeline) livré, audité, corrigé — voir STATE.md.
- [ ] EPIC 3 — Pipeline SSLKEYLOGFILE, prochain de l'ordre décidé.
- [ ] Décider du sort des polices (`DM Serif Display`/`Outfit`) : self-host `@fontsource` ou
      fichiers fournis par Chris (cf. STATE.md "Ouvert").
- [ ] Confirmer périmètre réseau exact (cf. STATE.md "Ouvert").
- [ ] Remplacer les icônes app Tauri (encore le template par défaut).
- [ ] Packaging EPIC 10 : ajuster le chemin en dur `/usr/local/bin/vitrail-helper`
      (Rust + `.policy` polkit) au vrai chemin d'installation choisi.

## Backlog (non priorisé)

- Portage éventuel du blocage interactif (au-delà de la simple consommation des décisions
  OpenSnitch) — explicitement hors scope v1.
- Dashboard distant consultable depuis un autre appareil — hors scope v1, surface réseau
  supplémentaire à évaluer séparément si jamais voulu.
- Publication communautaire (Reddit, HN) — action de Chris, texte de présentation à
  préparer (EPIC 11.4).
