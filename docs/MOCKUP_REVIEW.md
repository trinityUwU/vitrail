# Revue du mockup (docs/Mockup.html)

Mockup produit par Chris via GLM 5.2 à partir de `docs/UI_SPEC.md`. Fichier HTML/CSS/JS
statique unique — **référence de design et de comportement uniquement, pas l'interface
réelle**. L'app réelle est un frontend React/TypeScript modulaire dans un projet Tauri,
porté depuis ce fichier (cf. EPIC 0 et EPIC 8 dans `docs/EPICS.md`).

## Couverture face à `docs/UI_SPEC.md`

Les 13 écrans spécifiés sont présents et fidèles à la spec fonctionnelle : dashboard,
timeline, processus, destinations, inspecteur de flux (bannière pinning + sources de
corrélation + certificat vu), recherche avancée, alertes (dont la règle "changement de
visibilité inattendu" qui correspond exactement au signal de dégradation spécifié),
kill switch (sous-systèmes détaillés, arrêt d'urgence distinct, journal d'audit),
paramètres (7 sections = 9.1 à 9.7), confidentialité, logs, historique, onboarding en 5
étapes. Aucun écran de la spec n'est manquant.

## Défauts identifiés — à corriger lors du portage

1. **Données mockées macOS sur un projet Linux-only.** Chemins binaires
   (`/Applications/Google Chrome.app/Contents/MacOS/...`), noms d'interfaces
   (`en0 (Wi-Fi)`, `utun0 (VPN)`). Le périmètre du projet est Arch/Hyprland exclusivement
   (`ARCHITECTURE.md`) — à remplacer par des chemins/interfaces Linux (`/usr/bin/...`,
   `wlan0`/`eth0`/`enp*s0`) partout dans les données de démonstration.
2. **Nom de chaîne nftables incohérent.** Le mockup affiche `VITRAIL` (onglet Paramètres
   > Réseau, entrées du journal Kill Switch). `docs/PLAN.md` section 5 et `ARCHITECTURE.md`
   fixent le nom exact à `VITRAIL_REDIRECT` — choisi précisément pour être un nom unique
   non ambigu, jamais confondu avec une autre chaîne système. À aligner partout.
3. **Bug de texte français.** Description du bouton d'arrêt d'urgence (écran Kill Switch) :
   *"Force la désimmédiate de tous les sous-systèmes sans séquence orchestrée."* — mot
   cassé. Corrigé en : *"Force la désactivation immédiate de tous les sous-systèmes sans
   séquence orchestrée."*

## Ce qui n'est volontairement pas dans le mockup (attendu, pas un manque)

- Logique de filtrage/recherche réelle (le mockup bouchonne avec des données statiques).
- Toute connexion à un backend — c'est un prototype front pur.
- Ces deux points sont couverts par le portage réel (EPIC 8), pas par une correction du
  mockup lui-même.

## Décision de portage

Le mockup n'est pas repris tel quel : transformation en projet Tauri réel, frontend React/TS
modulaire (une vertical slice par écran, cf. `UI_SPEC.md`), connecté à une couche de
commandes IPC (`src-tauri/src/commands/`) plutôt qu'à des tableaux JS en dur dans les
composants. Le détail de ce portage est tracé dans `STATE.md` et `TODO.md` au fur et à
mesure.
