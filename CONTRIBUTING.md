# Contribuer à Vitrail

## Avant de commencer

Ce projet observe le trafic réseau d'une machine — toute contribution touchant à la
capture, au décryptage ou au kill switch doit respecter deux invariants non négociables :

1. **Aucune régression de réversibilité.** Toute modification touchant `killswitch/` doit
   être accompagnée d'un test de bascule (activation/désactivation) vérifiant l'absence de
   résidu système.
2. **Le certificate pinning n'est jamais contourné.** Vitrail respecte le pinning et bascule
   en mode métadonnées (fail-open) — aucune PR visant à "débloquer" des apps pinnées via
   patch binaire ou instrumentation ne sera acceptée. Voir `docs/PLAN.md` pour le
   raisonnement.

## Setup dev

Prérequis : Rust stable, Bun, `nftables`, [OpenSnitch](https://github.com/evilsocket/opensnitch)
installé et son daemon actif, [PolarProxy](https://www.netresec.com/?page=PolarProxy)
disponible dans le PATH (ou chemin renseigné via `.env`).

```bash
cp .env.example .env
bun install
./start.sh
```

## Structure du projet

Architecture par domaine, pas par couche technique — voir `ARCHITECTURE.md`. Avant
d'ajouter du code, identifiez le domaine concerné (`src-tauri/src/<domaine>/`) ; si aucun
domaine existant ne convient, proposez-en un nouveau en PR séparée avec sa justification
avant le code qui l'utilise.

## Plan d'implémentation

Le travail est découpé en epics/stories dans `docs/EPICS.md`. Toute PR référence l'epic et
la story qu'elle adresse.

## Style de code

Voir les règles appliquées à tout le projet : limites de taille fichier/fonction, typage
strict (zéro `any` côté TS équivalent, zéro `unwrap()` non justifié côté Rust), logging
obligatoire sur toute opération touchant système de fichiers/réseau/sous-processus.
