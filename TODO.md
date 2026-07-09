# TODO — Vitrail

Plan détaillé complet : [`docs/EPICS.md`](docs/EPICS.md). Ce fichier est la vue résumée +
le backlog non structuré.

## Epics (résumé — statut détaillé dans docs/EPICS.md)

- [x] EPIC 0 — Fondations projet (scaffold Tauri, CI, scripts, licence) — CI (0.3) restant
- [x] EPIC 1 — Attribution processus (OpenSnitch) — serveur gRPC ui.proto (tonic), cache
      pid/start_time, AskRule non-bloquant + panic guard de restauration
- [x] EPIC 2 — Capture réseau brute — pnet + tls-parser, vitrail-capture-helper (setcap
      cap_net_raw/cap_net_admin), CaptureSubsystem branché dans le kill switch
- [x] EPIC 3 — Décryptage TLS coopératif (SSLKEYLOGFILE) — tshark en sous-processus (non
      réinventé), injection .desktop réversible, visibilité Fully réelle pour la première fois
- [x] EPIC 4 — Décryptage TLS actif (PolarProxy) — CA rcgen, redirection nftables NAT,
      garde-fou anti-blackhole (confirm_listening sur le bon port, retry+état honnête si
      PolarProxy meurt), exclusions destination réelles ; process externe non bundlé
      (comme tshark), CLI PolarProxy non vérifiable sur cette machine (absente)
- [x] EPIC 5 — Moteur de corrélation — fusion capture+attribution par 5-tuple/fenêtre 5s,
      visibilité Meta/Attrib réelle (Fully/Unknown prêts pour EPIC 3/4), flows/flows_fts
      alimentées, timeline temps réel réelle (remplace l'émetteur factice EPIC 8.4)
- [x] EPIC 6 — Stockage & requêtes — SQLite WAL (rusqlite bundled), migre les 3 JSONL
      provisoires EPIC 7/2/1, purge/rétention/sessions réelles ; flows/processes/FTS5
      créées vides (alimentées en EPIC 5)
- [x] EPIC 7 — Kill switch & réversibilité — orchestration réelle sur ses 6 étapes (CA,
      nftables, PolarProxy, attribution, capture, keylog) — tous les StubSubsystem
      remplacés par du réel, plus aucun stub dans la séquence d'activation
- [~] EPIC 8 — Contrat UI / IPC — frontend + commandes complètes livrées et auditées
      (contrat Flow complet, exclusions centralisées, CRUD alertes, recherche sauvegardée,
      purge, tag, historique session, notifications/keylog persistés), streaming réel (8.4)
      désormais réel (EPIC 5, événement vitrail://flow), contrat de types généré (8.5)
      toujours manuel
- [ ] EPIC 9 — Sécurité & durcissement
- [ ] EPIC 10 — Packaging & distribution
- [ ] EPIC 11 — Documentation communautaire & visibilité

## Immédiat

- [x] **BLOQUANT USAGE RÉEL** (2026-07-10, résolu — cf. STATE.md) : setup système complet
      (vitrail-helper/policy polkit/setcap/tshark/opensnitchd/PolarProxy/agent polkit
      Hyprland), bug d'idempotence CA corrigé, et raccordement IPC réel de
      dashboard/processus/destinations/journal système (PLAN.md §6decies) — Alertes & Règles
      en stub honnête vide (décision explicite, pas de moteur d'évaluation dans cette passe).
      Reste à confirmer par un test bout-en-bout complet par Chris.

- [x] Repo GitHub public créé et poussé : https://github.com/trinityUwU/vitrail.
- [x] EPIC 7 (squelette kill switch) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 2 (capture réseau brute) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 1 (attribution processus) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 6 (storage SQLite) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 5 (corrélation timeline) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 3 (SSLKEYLOGFILE) livré, audité, corrigé — voir STATE.md.
- [x] EPIC 4 (PolarProxy) livré, audité, corrigé — voir STATE.md. Les 7 EPICs de logique
      système (1-7) sont maintenant tous réels. **Validation manuelle du fail-open réel
      contre une app à pinning connu reste à faire par Chris** (impossible en environnement
      agent, cf. STATE.md).
- [ ] Décider du sort des polices (`DM Serif Display`/`Outfit`) : self-host `@fontsource` ou
      fichiers fournis par Chris (cf. STATE.md "Ouvert").
- [ ] Confirmer périmètre réseau exact (cf. STATE.md "Ouvert").
- [ ] Remplacer les icônes app Tauri (encore le template par défaut).
- [ ] Packaging EPIC 10 : ajuster le chemin en dur `/usr/local/bin/vitrail-helper`
      (Rust + `.policy` polkit) au vrai chemin d'installation choisi.
- [ ] Moteur d'alertes complet (persistance `alert_rules`/`alert_events` + évaluation temps
      réel contre les flux) — reporté à un futur EPIC dédié, stub honnête vide en attendant
      (décision 2026-07-10, cf. STATE.md et PLAN.md §6decies).

## Backlog (non priorisé)

- Portage éventuel du blocage interactif (au-delà de la simple consommation des décisions
  OpenSnitch) — explicitement hors scope v1.
- Dashboard distant consultable depuis un autre appareil — hors scope v1, surface réseau
  supplémentaire à évaluer séparément si jamais voulu.
- Publication communautaire (Reddit, HN) — action de Chris, texte de présentation à
  préparer (EPIC 11.4).
