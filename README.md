# Vitrail

Visibilité complète, locale et réversible de tout le trafic réseau d'une machine Linux —
attribution par processus, décryptage TLS (coopératif et actif avec repli automatique sur
le certificate pinning), et une garantie centrale : **couper l'outil ne laisse aucun
résidu**.

> Statut : en phase de planification/architecture. Aucune ligne de code fonctionnelle
> encore. Voir `docs/PLAN.md` et `docs/EPICS.md`.

## Pourquoi

Les outils existants couvrent chacun un morceau du problème, aucun ne les combine :

| Outil | Attribution processus | Contenu déchiffré | Réversibilité garantie |
|---|---|---|---|
| [Wireshark](https://www.wireshark.org/) + `SSLKEYLOGFILE` | Non | Oui (apps coopérantes) | N/A (passif) |
| [Sniffnet](https://github.com/GyulyVGC/sniffnet) | Non | Non | N/A (passif) |
| [OpenSnitch](https://github.com/evilsocket/opensnitch) | Oui | Non | Oui |
| [PolarProxy](https://www.netresec.com/?page=PolarProxy) | Non | Oui (MITM + fail-open sur pinning) | Partielle |
| **Vitrail** | Oui | Oui (les deux voies) | Oui, avec vérification |

Vitrail n'est pas un nouveau moteur de capture ou de déchiffrement — c'est la couche qui
fusionne ce que ces outils voient chacun de leur côté en une timeline unique, lisible, et
qui garantit qu'on peut tout couper sans laisser de trace.

## Périmètre

- Une machine, un utilisateur, Linux uniquement.
- Aucune exposition réseau — application locale, pas de dashboard distant.
- Observation pure en v1 (pas de blocage — délégué à OpenSnitch si souhaité).
- Zéro casse sur les applications à certificate pinning : repli automatique en mode
  métadonnées, jamais de connexion bloquée à cause de l'interception.

## Documentation

- [`docs/PLAN.md`](docs/PLAN.md) — architecture technique complète, raisonnement, état de
  l'art.
- [`docs/EPICS.md`](docs/EPICS.md) — plan d'implémentation détaillé (epics/stories).
- [`docs/UI_SPEC.md`](docs/UI_SPEC.md) — spécification fonctionnelle exhaustive de
  l'interface (sans design — le mockup visuel est produit séparément).
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — carte des domaines et frontières de module.

## Stack

Tauri (Rust + frontend React/TypeScript), SQLite (WAL), orchestration d'
[OpenSnitch](https://github.com/evilsocket/opensnitch) et
[PolarProxy](https://www.netresec.com/?page=PolarProxy) comme sous-systèmes.

## Licence

MIT — voir [`LICENSE`](LICENSE).

## Contribuer

Voir [`CONTRIBUTING.md`](CONTRIBUTING.md).
