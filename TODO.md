# TODO — Vitrail

Plan détaillé complet : [`docs/EPICS.md`](docs/EPICS.md). Ce fichier est la vue résumée +
le backlog non structuré.

## Epics (résumé — statut détaillé dans docs/EPICS.md)

- [x] EPIC 0 — Fondations projet (scaffold Tauri, CI, scripts, licence) — CI (0.3) restant
- [ ] EPIC 1 — Attribution processus (OpenSnitch)
- [x] EPIC 2 — Capture réseau brute — pnet + tls-parser, vitrail-capture-helper (setcap
      cap_net_raw/cap_net_admin), CaptureSubsystem branché dans le kill switch
- [ ] EPIC 3 — Décryptage TLS coopératif (SSLKEYLOGFILE)
- [ ] EPIC 4 — Décryptage TLS actif (PolarProxy, fail-open)
- [ ] EPIC 5 — Moteur de corrélation
- [ ] EPIC 6 — Stockage & requêtes
- [~] EPIC 7 — Kill switch & réversibilité — squelette d'orchestration livré et audité
      (7.1-7.6 couverts avec sous-systèmes stub ; CA/PolarProxy/attribution/capture/keylog
      réels arrivent avec leurs EPICs respectifs)
- [~] EPIC 8 — Contrat UI / IPC — frontend + commandes complètes livrées et auditées
      (contrat Flow complet, exclusions centralisées, CRUD alertes, recherche sauvegardée,
      purge, tag, historique session, notifications/keylog persistés), streaming réel (8.4)
      encore un émetteur factice, contrat de types généré (8.5) toujours manuel
- [ ] EPIC 9 — Sécurité & durcissement
- [ ] EPIC 10 — Packaging & distribution
- [ ] EPIC 11 — Documentation communautaire & visibilité

## Immédiat

- [x] Repo GitHub public créé et poussé : https://github.com/trinityUwU/vitrail.
- [x] EPIC 7 (squelette kill switch) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 2 (capture réseau brute) livré, audité, corrigé — voir STATE.md.
- [ ] EPIC 1 — Attribution processus (serveur gRPC ui.proto), prochain de l'ordre décidé.
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
