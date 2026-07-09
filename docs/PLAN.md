# Vitrail — Plan technique

Document de référence pour l'architecture complète. `ARCHITECTURE.md` (racine) résume les
frontières de domaines ; ce fichier détaille le raisonnement, les flux de données et les
décisions techniques.

## 1. Problème et périmètre

Donner à un utilisateur d'une machine Linux (Arch/Hyprland dans le cas de Chris, mais
généralisable à toute distro) une vue **complète, claire et réversible** de tout le trafic
réseau entrant/sortant de sa machine — y compris le contenu applicatif transporté en TLS —
sans jamais laisser de résidu quand l'outil est coupé, et sans casser les applications qui
font du certificate pinning.

Périmètre v1 :
- Une seule machine, un seul utilisateur.
- Application locale (Tauri), **aucune exposition réseau** — ni LAN ni Internet. Le doute
  initial ("accessible depuis le réseau") est résolu par défaut sur *aucune surface réseau
  du tout* : c'est une app de bureau, IPC Tauri interne uniquement. Si un dashboard
  consultable depuis un autre appareil (téléphone) est voulu plus tard, ce sera une
  décision produit explicite à part (surface d'attaque supplémentaire), pas un défaut v1.
- Linux uniquement. Pas de portage Windows/macOS prévu.
- Pas de blocage de trafic en v1 (observation pure). Le blocage (façon pare-feu interactif)
  est une extension possible via OpenSnitch mais hors scope initial — Vitrail *consomme*
  les décisions d'OpenSnitch, il n'en prend pas de nouvelles.

## 2. Non-réinvention — état de l'art (recherche 2026-07-09)

| Besoin | Outil existant | Rôle dans Vitrail |
|---|---|---|
| Attribution processus ↔ connexion | [OpenSnitch](https://github.com/evilsocket/opensnitch) (eBPF, Go daemon, gRPC) | Source d'événements d'attribution, consommé tel quel |
| Décryptage TLS actif + repli sur pinning | [PolarProxy](https://www.netresec.com/?page=PolarProxy) (Netresec, fail-open mode natif) | Source de flux déchiffrés, orchestré comme sous-processus |
| Décryptage TLS coopératif sans interception | `SSLKEYLOGFILE` (standard NSS/BoringSSL/OpenSSL) | Pipeline complémentaire pour apps qui exportent leurs clés (Firefox, Chrome, curl, Node) |
| Capture brute / métadonnées de flux | libpcap / AF_PACKET | Visibilité de base indépendante du TLS (DNS, QUIC, plaintext) |

Aucun outil existant ne fait la fusion des trois premières sources dans une timeline unique
avec attribution + contenu + réversibilité garantie. **C'est la valeur ajoutée de Vitrail** :
une couche d'orchestration et de corrélation, pas un nouveau moteur de capture/décryptage.

Concurrents partiels à citer dans le README pour se positionner honnêtement :
- **Sniffnet** (Rust, GUI) — visibilité de flux, zéro décryptage TLS, zéro attribution
  processus native au niveau du détail voulu.
- **Wireshark + SSLKEYLOGFILE** — le standard historique, mais scriptable/manuel, pas de
  vue "application-centric" ni de kill switch unifié.
- **OpenSnitch seul** — attribution excellente, aucune visibilité sur le contenu.

## 3. Architecture générale

```
┌─────────────────────────────────────────────────────────────────┐
│                         Vitrail (Tauri app)                     │
│                                                                   │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐     │
│  │attribution│  │  capture  │  │decryption │  │  keylog   │     │
│  │ (OpenSnitch│  │ (libpcap/ │  │(PolarProxy │  │(SSLKEYLOG │     │
│  │  gRPC client)│ │  AF_PACKET)│ │ subprocess)│  │  tail)    │     │
│  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └─────┬─────┘     │
│        │              │              │              │            │
│        └──────────────┴──────┬───────┴──────────────┘            │
│                               ▼                                   │
│                       ┌───────────────┐                           │
│                       │  correlation  │  (fusion 5-tuple+temps)   │
│                       └───────┬───────┘                           │
│                               ▼                                   │
│                       ┌───────────────┐                           │
│                       │    storage    │  (SQLite WAL)             │
│                       └───────┬───────┘                           │
│                               ▼                                   │
│                       ┌───────────────┐                           │
│                       │   commands    │  (surface IPC Tauri)      │
│                       └───────┬───────┘                           │
│                               ▼                                   │
│                          Frontend React/TS (mockup GLM à intégrer)│
│                                                                   │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │                      killswitch                            │   │
│  │  orchestre le cycle de vie de TOUS les sous-systèmes       │   │
│  │  ci-dessus + nftables + CA + snapshot/diff d'état          │   │
│  └───────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## 4. Domaines (Screaming Architecture)

Chaque domaine = un module Rust sous `src-tauri/src/`, frontière explicite, communique via
des types partagés dans `shared/`, jamais d'accès direct aux internes d'un autre domaine.

### `attribution/`
Client du daemon OpenSnitch (déjà installé et lancé par l'utilisateur, pas géré en
sous-processus — c'est un daemon système persistant). Consomme son flux d'événements
(gRPC ou lecture de ses logs structurés selon ce que permet sa version). Produit un type
`AttributionEvent { pid, exe_path, uid, five_tuple, verdict, timestamp }`.

### `capture/`
Capture passive AF_PACKET (via `pnet` ou `pcap` crate) sur les interfaces actives.
Parsing minimal : 5-tuple, taille, timestamp, extraction du SNI depuis le ClientHello TLS
(sans déchiffrer — juste le champ en clair du handshake). Fournit la visibilité de base
même quand aucune autre couche ne peut aider (pinning, protocole non-TLS).

### `decryption/`
Gère le cycle de vie de PolarProxy en sous-processus : génération/rotation de la CA locale,
configuration du mode fail-open, lancement/arrêt, lecture de sa sortie (PCAP en continu ou
export JSON si la version le permet). Produit un type `DecryptedFlow { five_tuple, host,
path, headers, body_preview, content_type }` quand le déchiffrement réussit, ou un simple
signal `PinningDetected { five_tuple, host }` quand fail-open s'est déclenché.

### `keylog/`
Gère l'injection de `SSLKEYLOGFILE` pour les applications coopérantes : variable d'env
globale utilisateur (fichier de session), wrapper de lancement pour les `.desktop`
d'applications ciblées (navigateurs), tail du fichier de clés en continu. Fait tourner une
instance `tshark` en tâche de fond pointée sur ce fichier de clés pour produire des
événements déchiffrés à partir du pcap de `capture/`, sans passer par un MITM.

### `correlation/`
Le cœur de la valeur ajoutée. Fusionne les quatre sources par 5-tuple + fenêtre temporelle
en une timeline unique. Résout les conflits (une même connexion peut être vue par
`capture/` en métadonnées ET par `decryption/` en clair ET par `attribution/` avec un pid).
Détermine le niveau de visibilité final par flux : `FullyDecrypted` / `MetadataOnly`
(pinning détecté, fail-open) / `AttributedOnly` (pas de TLS) / `Unknown`.

### `storage/`
SQLite en mode WAL (règle projet). Schéma : table `flows` (timeline unifiée), table
`processes` (cache résolution pid→exe au fil du temps, les pid se recyclent), table
`system_events` (actions killswitch, démarrages/arrêts de sous-systèmes, erreurs). Politique
de rétention configurable (par défaut : 30 jours, purgeable manuellement).

### `killswitch/`
Domaine transverse, orchestrateur de cycle de vie. Responsabilités :
- **Activation** : snapshot de l'état système avant toute modification (règles nftables
  existantes, liste des CA de confiance, état des daemons) → écrit dans
  `storage/system_events` → applique la chaîne nftables dédiée (`VITRAIL_REDIRECT`,
  jamais de règle en dehors de cette chaîne nommée) → installe la CA locale → démarre
  PolarProxy et le tail keylog.
- **Désactivation** : flush de la chaîne `VITRAIL_REDIRECT` uniquement → retrait de la CA
  (par empreinte exacte, jamais un retrait générique du trust store) → arrêt des
  sous-processus → **diff de vérification** : compare l'état système post-arrêt à l'état
  pré-activation, log toute divergence dans `system_events` et l'affiche à l'utilisateur.
- Ne touche jamais : DNS, configuration proxy système/navigateur, autres règles nftables/
  iptables préexistantes.

### `shared/`
Types communs (`FiveTuple`, `FlowVisibility`, `ProcessRef`), config (fichier TOML utilisateur),
logging (`tracing` crate, équivalent Rust de loguru — pas de crate `log` nu).

### `commands/`
Seule surface exposée au frontend via `#[tauri::command]`. Aucun domaine n'est appelé
directement par l'UI — tout passe par ce module, qui ne fait qu'agréger/déléguer (pas de
logique métier ici).

## 5. Réversibilité — garanties concrètes

1. **nftables** : une seule chaîne nommée (`VITRAIL_REDIRECT`), jamais de manipulation des
   chaînes système. Flush = suppression de la chaîne, rien d'autre.
2. **CA locale** : générée au premier lancement, stockée hors du repo, empreinte affichée
   dans les paramètres. Le retrait cible l'empreinte exacte injectée, jamais un
   `update-ca-trust` générique qui pourrait toucher d'autres CA installées par ailleurs.
3. **DNS** : jamais touché, à aucun moment. Décision explicite pour éviter la classe de bug
   déjà rencontrée sur d'autres projets (résolveur qui ne revient pas proprement après
   coupure).
4. **Diff de vérification obligatoire** : chaque désactivation produit un rapport
   avant/après consultable dans l'UI (panneau kill switch, cf. `UI_SPEC.md`).
5. **Fail-open par défaut** : jamais de blocage silencieux d'une app à cause du pinning —
   passthrough automatique, visibilité dégradée mais app fonctionnelle.

## 6. Stack technique (décisions, division du travail)

| Choix | Défaut retenu | Justification |
|---|---|---|
| Framework app | Tauri (Rust + frontend React/TS à venir) | Cohérent avec Aegis/NULLNODE/Anamnese ; les opérations privilégiées (nftables, CA, sous-process) sont naturelles en Rust, gRPC OpenSnitch a des bindings Rust matures |
| Base de données | SQLite WAL | Règle projet, pas de serveur externe, cohérent local-first |
| Logging | `tracing` + `tracing-subscriber` | Équivalent Rust de la stack loguru/pino imposée aux autres langages |
| Licence | MIT | Cohérent avec les autres repos publics de Chris (Aegis) |
| Packaging | AppImage (pattern Aegis) | Éprouvé, install sans root |

Ce tableau est une proposition par défaut, pas une décision fermée — à valider une fois
qu'on attaque l'implémentation (cf. section "orchestration" à discuter séparément).

## 6bis. Élévation de privilèges (décidé 2026-07-09)

nftables (application/retrait de la chaîne `VITRAIL_REDIRECT`) et l'installation/retrait de
la CA locale dans le trust store système exigent root. Modèle retenu : **polkit par action**,
pas de daemon privilégié persistant.

- Un petit binaire séparé `vitrail-helper` (pas l'app Tauri elle-même) exécute les opérations
  root ponctuelles : `apply-nft-chain` / `flush-nft-chain` / `install-ca` / `remove-ca`.
- Chaque appel passe par `pkexec` avec une règle polkit dédiée (`re.vitrail.helper.policy`)
  déclenchant un prompt natif — une action = une autorisation, jamais un daemon root qui
  tourne en continu.
- L'app Tauri (`killswitch/`) orchestre la séquence en invoquant `vitrail-helper` pour chaque
  étape privilégiée, capture son code de sortie/stderr pour le journal d'audit du kill switch
  (écran 8), jamais de sudo interactif caché dans un script.
- Conséquence sur `killswitch/` (EPIC 7) : la séquence d'activation/désactivation doit
  supporter qu'une étape déclenche un prompt utilisateur (polkit) et attendre sa réponse avant
  de continuer — pas une simple boucle synchrone silencieuse.
- Conséquence packaging (EPIC 10) : le paquet doit installer `vitrail-helper` avec le bon
  binaire de contrôle, le fichier `.policy` polkit, et documenter clairement à l'utilisateur
  ce que chaque prompt autorise (contre la fatigue d'autorisation aveugle).

Rejeté : daemon privilégié persistant (surface d'attaque continue, contraire au principe
zéro-résidu déjà posé en section 5) ; app Tauri lancée root (violerait le moindre privilège
de façon injustifiable pour une app dont l'essentiel du travail — UI, corrélation, lecture —
n'a besoin d'aucun privilège).

## 6ter. Squelette d'orchestration EPIC 7 (décidé 2026-07-09)

Démarrage de l'implémentation réelle (EPICs 1-7) dans l'ordre déjà acté : kill switch
d'abord, en squelette, car aucun sous-système (capture, attribution, decryption, keylog)
n'existe encore. Le squelette doit permettre à chaque EPIC suivant de brancher sa vraie
logique sans toucher au code d'orchestration.

- **Workspace Cargo** : `vitrail-helper` devient un second membre d'un workspace Cargo
  racine (`/Cargo.toml` avec `[workspace] members = ["src-tauri", "vitrail-helper"]`),
  binaire minimal, aucune dépendance Tauri. Surface volontairement étroite : sous-commandes
  fixes (`nft-apply`, `nft-flush`), arguments passés en tableau (`std::process::Command`,
  jamais d'interpolation shell), refus de toute autre opération — le binaire ne fait QUE ce
  que son nom dit, c'est la garantie de sécurité qui justifie l'élévation polkit. Le fichier
  `.policy` (`re.vitrail.helper.policy`) référence le chemin absolu du binaire installé.
- **Trait `Subsystem`** (`killswitch/subsystem.rs`) : `start()`/`stop()`/`is_active()`/
  `name()`. Chaque domaine (`capture`, `attribution`, `decryption`, `keylog`) implémentera
  ce trait quand son EPIC arrive — en attendant, une implémentation stub (flip d'un booléen
  atomique + log `tracing`, pas d'action système réelle) permet à la séquence 7.2/7.3 de
  tourner de bout en bout dès maintenant sur des sous-systèmes encore vides.
- **Trait `NftablesBackend`** : abstraction entre la logique d'orchestration et l'exécution
  réelle (`pkexec vitrail-helper nft-apply`), avec un `FakeNftablesBackend` en mémoire pour
  les tests. Nécessaire pour 7.6 (100 cycles start/stop) — un test ne doit jamais déclencher
  de vrai prompt polkit.
- **Persistance des `system_events` avant EPIC 6** : `storage/` n'existe pas encore. Le
  squelette écrit en JSONL append-only dans `$XDG_DATA_HOME/vitrail/system_events.jsonl`
  (créé avec permissions 600) plutôt que d'inventer une persistance jetable. Migration vers
  SQLite explicitement prévue en EPIC 6 (remplacement du fichier par une table, pas une
  réécriture de la logique de snapshot).
- **Chaîne nftables squelette** : EPIC 7 crée/détruit la table/chaîne `VITRAIL_REDIRECT` vide
  (marqueur d'état "actif"), sans aucune règle de redirection réelle — celles-ci arrivent
  avec EPIC 4 (PolarProxy) et EPIC 2 (capture). Un diff pré/post qui trouve la chaîne
  toujours présente après désactivation est donc déjà un signal exploitable dès ce squelette.
- **CA et PolarProxy dans la séquence 7.2** : tant qu'EPIC 4 n'existe pas, ces étapes sont
  des `Subsystem` stub (no-op) au même titre que les autres — l'ordre CA → nftables →
  PolarProxy → attribution → capture → keylog est câblé dès maintenant pour ne pas avoir à
  retoucher `sequence.rs` plus tard.

## 7. Ouvert / à trancher avec Chris

- **Portée réseau réellement voulue** : confirmation que v1 = zéro exposition réseau
  (IPC Tauri pur), et que "accessible depuis le réseau" visait juste l'accès local à la
  machine elle-même, pas un dashboard distant.
- **Blocage vs observation pure** : Vitrail v1 n'écrit aucune règle de blocage lui-même
  (délégué à OpenSnitch) — à confirmer que c'est bien le périmètre voulu.
- **Rétention par défaut** des événements stockés (proposé : 30 jours) et politique de
  purge automatique.
