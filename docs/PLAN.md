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

## 6quater. EPIC 2 — Capture réseau brute (décidé 2026-07-09)

Premier domaine réel après le squelette kill switch. Décisions tranchées pour lever
l'ambiguïté laissée ouverte en section 2 ("libpcap / AF_PACKET").

- **Crate de capture : `pnet`** (pas `pcap`/libpcap). Pur Rust, pas de dépendance système
  C supplémentaire à l'exécution, cohérent avec la préférence sovereignty (moins de
  dépendances externes imposées). `pnet::datalink` ouvre un canal AF_PACKET par interface,
  `pnet::packet` parse Ethernet/IPv4/IPv6/TCP/UDP sans code maison bas niveau.
- **Parsing SNI (2.3) : crate `tls-parser`** (rusticata) pour extraire le champ SNI du
  ClientHello TLS en clair — pas de déchiffrement, lecture d'un champ non chiffré du
  handshake. Pas de dépendance OpenSSL/rustls nécessaire pour cette seule extraction.
- **Élévation de privilèges — divergence assumée par rapport à 6bis** : la capture est un
  processus continu (tant que le kill switch est actif), pas une action ponctuelle comme
  nftables/CA. Un prompt polkit à chaque activation serait une friction inutile pour quelque
  chose qui n'est pas destructif ni système-global. Décision : un troisième binaire du
  workspace, `vitrail-capture-helper`, reçoit à l'installation (EPIC 10, `setcap`) les
  capacités Linux `cap_net_raw,cap_net_admin+eip` — un périmètre bien plus étroit que root,
  jamais de mot de passe à l'usage. Il tourne en utilisateur normal, pas de daemon root.
  En dev, Chris devra faire `sudo setcap cap_net_raw,cap_net_admin+eip
  target/debug/vitrail-capture-helper` manuellement après chaque build (documenté dans
  `README.md`/`CONTRIBUTING.md`).
- **Modèle de process** : `capture/mod.rs` (app Tauri, non privilégiée) spawn le binaire
  `vitrail-capture-helper` une fois à l'activation (`Subsystem::start`), le tue proprement à
  la désactivation (`Subsystem::stop`, `SIGTERM` puis timeout avant `SIGKILL`). Le helper
  ouvre un thread de capture par interface active détectée dynamiquement
  (`pnet::datalink::interfaces()`, filtre `up && !loopback`, pas d'interface en dur — 2.1),
  et écrit un flux JSON Lines sur stdout (un enregistrement par paquet retenu : 5-tuple,
  timestamp, octets, SNI si présent, protocole détecté best-effort). L'app parent lit stdout
  en continu (thread dédié) et alimente un buffer interne.
- **Débit (2.5)** : limiteur de débit type token-bucket dans le helper lui-même (pas côté
  app), seuil configurable (défaut proposé : 2000 paquets/s), paquets excédentaires comptés
  et droppés avec un avertissement périodique (pas un log par paquet perdu) plutôt que
  transmis — protège autant le helper que l'app parent.
- **Persistance provisoire** : comme `system_events` (6ter), les enregistrements de capture
  sont journalisés en JSONL append-only (`$XDG_DATA_HOME/vitrail/capture_events.jsonl`,
  600) en attendant EPIC 6/SQLite — PAS une réécriture du contrat `Flow` existant
  (`commands/types.rs`), qui reste servi par les mocks jusqu'à EPIC 5 (corrélation) qui
  fusionnera captures réelles + attribution + contenu déchiffré en un seul flux exploitable
  par l'UI. Ce périmètre EPIC 2 est donc backend uniquement, sans nouvelle commande IPC
  visible dans le Timeline/Dashboard pour l'instant (juste le statut "actif" du subsystem
  dans le panneau kill switch, déjà câblé par EPIC 7).
- **`capture::CaptureSubsystem`** implémente le trait `Subsystem` de `killswitch/subsystem.rs`
  et remplace le `StubSubsystem` nommé "capture" dans la séquence 7.2/7.3 — aucune autre
  modification de `killswitch/` nécessaire (c'est exactement la garantie que le squelette
  devait fournir).

## 6quinquies. EPIC 1 — Attribution processus (décidé 2026-07-09)

Deuxième domaine réel après capture. S'appuie sur la correction d'architecture déjà actée
dans `docs/EPICS.md` (EPIC 1) : `attribution/` implémente le **serveur** gRPC `ui.proto`,
`opensnitchd` en est le client.

- **`.proto` source** : récupérer `proto/ui/ui.proto` tel quel depuis le dépôt public
  `github.com/evilsocket/opensnitch` (dernière version sur `main` au moment du build), le
  copier dans `src-tauri/proto/ui.proto` (fichier figé, versionné, pas de génération à la
  volée depuis internet à l'exécution). `tonic-build` + `prost` compilent ce `.proto` dans
  `build.rs` de `src-tauri`.
- **Socket dédié Vitrail** : `$XDG_RUNTIME_DIR/vitrail/ui.sock` (pas `/tmp/osui.sock`, qui
  est le défaut d'OpenSnitch lui-même — Vitrail a son propre socket nommé pour ne jamais le
  confondre avec celui d'une autre UI). Créé avec permissions restrictives (dossier 700).
- **Reconfiguration du daemon (1.2/1.6) — nouvelle action privilégiée** : la config
  `opensnitchd` (`/etc/opensnitchd/default-config.json`, champ `Server.Address`) et le
  redémarrage du service (`systemctl restart opensnitchd`) exigent root — même famille de
  besoin que nftables/CA (6bis). `vitrail-helper` gagne une troisième sous-commande
  allowlistée : `opensnitch-set-socket <chemin-socket>` (édite le champ JSON, redémarre le
  service ; JAMAIS d'exécution shell arbitraire, le chemin socket est validé côté Rust avant
  l'appel — motif attendu strict, refus si ça ne ressemble pas à un chemin de socket UNIX
  légitime). `attribution/` lit l'adresse d'origine AVANT de la remplacer (story 1.1),
  la garde en mémoire/JSONL provisoire (même pattern que `system_events.jsonl`), et rappelle
  `opensnitch-set-socket <adresse-d-origine>` à la désactivation (story 1.6) — restaurer
  n'est jamais un no-op silencieux si le daemon n'a pas pu être contacté, le kill switch doit
  le voir comme une divergence en 7.4.
- **Cache pid→exe (1.3)** : clé composite `(pid, start_time)` — `start_time` lu dans
  `/proc/<pid>/stat` (champ 22, temps de démarrage en ticks depuis le boot), jamais le pid
  seul, pour ne jamais confondre un pid recyclé avec l'ancien process qui l'occupait.
- **Résolution nom d'application (1.4)** : heuristique best-effort, cherche un `.desktop`
  dans `$XDG_DATA_DIRS/applications/` dont la ligne `Exec=` référence le basename du binaire
  résolu ; à défaut, nom du binaire brut. Utilisé UNIQUEMENT pour l'affichage — la logique de
  corrélation (EPIC 5) continue de raisonner sur `pid`/`exe_path` exacts, jamais sur ce nom.
- **Tests (1.5)** : un vrai client `tonic` de test se connecte au socket Vitrail et rejoue des
  messages `ui.proto` construits à la main (notifications de connexion), vérifie le décodage
  en `AttributionEvent` et la mise à jour du cache — pas de mock au niveau du protocole,
  seulement au niveau de la vraie reconfiguration système (pas de vrai `opensnitchd` requis
  en test, seulement le serveur gRPC de Vitrail).
- **`attribution::AttributionSubsystem`** implémente le trait `Subsystem` (comme
  `CaptureSubsystem`), remplace le stub "attribution" dans `killswitch/subsystem.rs` sans
  modification de l'orchestration.

## 6sexies. EPIC 6 — Stockage & requêtes (décidé 2026-07-09)

Troisième domaine réel. Remplace les persistances JSONL provisoires posées en EPIC 7/2/1
(`system_events.jsonl`, `capture_events.jsonl`, état socket attribution) par une vraie base
SQLite, sans changer le comportement observable de ces domaines (mêmes tests verts).

- **Crate : `rusqlite`, feature `bundled`** (SQLite statiquement lié, aucune dépendance
  système `libsqlite3-dev` — cohérent sovereignty). PRAGMA `journal_mode=WAL` à l'ouverture.
- **Connexion** : une seule connexion applicative protégée par `Mutex` (charge attendue très
  faible pour un outil desktop mono-utilisateur, pas besoin de pool). Chemin DB :
  `$XDG_DATA_HOME/vitrail/vitrail.db` (créé 600, même répertoire que les anciens JSONL).
- **Migrations** : fichiers `.sql` embarqués (`include_str!`), numérotés, exécutés dans
  l'ordre au démarrage, version courante trackée dans une table `schema_migrations` — pas de
  dépendance externe de migration, mécanisme volontairement simple (6.1).
- **Schéma minimal EPIC 6** : `system_events`, `capture_events`, `attribution_state`
  (remplacent les 3 JSONL existants, colonnes fidèles aux structs Rust déjà définies),
  `flows` et `processes` créées vides dès maintenant (6.1 les nomme explicitement) mais pas
  encore alimentées par de vraies données — ce sera EPIC 5 (corrélation) qui écrira dedans ;
  ne pas essayer de faire écrire `flows`/`processes` par `capture/`/`attribution/` dans cette
  passe, ça romprait la frontière de domaine (storage n'a pas de logique de corrélation, et
  capture/attribution ne connaissent pas le format `Flow` unifié).
- **Index (6.2)** : `(timestamp)` sur les 3 tables d'événements, `(pid)` sur
  `attribution_state`, préparés maintenant même si peu de volume actuel — coût nul, évite un
  oubli plus tard.
- **FTS5 (6.4)** : table virtuelle FTS5 créée dans le schéma dès cette passe (`flows_fts` sur
  les colonnes texte prévues du futur `Flow`), mais PAS encore alimentée ni branchée à une
  commande IPC de recherche — `flows` reste mockée jusqu'à EPIC 5, chercher dedans n'aurait
  aucun sens réel. Le schéma est prêt, le branchement viendra avec la corrélation.
- **Rétention (6.3)** : tâche de purge basée sur `Settings.retention_days` (déjà dans le
  contrat IPC `commands/types.rs`), supprime les lignes des 3 tables d'événements plus
  vieilles que le seuil, `VACUUM` après purge. Déclenchée par `purge_data`/`purge_logs`
  (6.6, déjà des commandes IPC existantes mockées à rendre réelles) — pas de tâche planifiée
  automatique en tâche de fond dans cette passe (pas de scheduler encore dans le projet),
  purge manuelle depuis Paramètres suffit pour l'instant, un vrai scheduler sera une story
  ultérieure si Chris le demande.
- **6.5 export** : pas de nouvelle commande IPC si l'export peut rester côté client comme
  aujourd'hui (`history-report.ts` génère déjà un rapport à partir de `get_session_detail`) ;
  si le build agent juge qu'un export brut JSON/CSV d'une plage de données a besoin d'un vrai
  accès SQLite (probable pour de gros volumes), ajouter une commande minimale
  `export_data_range(from, to, format) -> String` sur le même modèle que `export_config`
  existant.
- **6.6/6.7** : `purge_data`/`purge_logs`/`get_session_detail`/`delete_session`
  (`commands/settings.rs`, déjà dans le contrat IPC, actuellement mockées) deviennent de
  vraies requêtes SQLite via `storage::`. `commands/` continue à n'agréger/déléguer, jamais
  de SQL directement dans `commands/settings.rs`.
- **Domaines appelants** : `killswitch/snapshot.rs`, `capture/events.rs`, l'écriture d'état
  socket dans `attribution/daemon_config.rs` appellent désormais `storage::events::*` au lieu
  d'écrire un JSONL directement — `storage/` expose une API publique minimale par domaine
  appelant (ex: `storage::events::record_system_event(...)`,
  `storage::events::record_capture_packet(...)`, `storage::attribution::save_origin_socket`),
  jamais d'accès direct à la connexion SQLite depuis l'extérieur de `storage/`. Les tests
  existants de ces 3 domaines (100 cycles kill switch, tests capture/attribution) doivent
  rester verts — `storage/` doit être testable en mémoire (`rusqlite` supporte
  `Connection::open_in_memory()`), utilisé dans les tests des domaines appelants au lieu du
  vrai fichier `vitrail.db`.

## 6septies. EPIC 5 — Moteur de corrélation (décidé 2026-07-09)

Quatrième domaine réel. EPIC 5 arrive dans l'ordre décidé AVANT keylog (EPIC 3) et
PolarProxy (EPIC 4) — seules 2 des 4 sources prévues (capture, attribution) existent
réellement à ce stade. La fusion doit fonctionner avec seulement ces 2 sources aujourd'hui,
et accueillir décryptage/keylog plus tard sans réécriture.

- **Clé de fusion (5.1)** : le message `Connection` de `ui.proto` (attribution) transporte
  déjà un 5-tuple complet (`protocol`, `src_ip`, `src_port`, `dst_ip`, `dst_port`) — même
  granularité que les enregistrements `capture/`. Clé de fusion : 5-tuple exact + fenêtre
  temporelle tolérante (proposé : 5 secondes — les timestamps capture/attribution ne sont
  jamais strictement synchrones, `AskRule` peut arriver avant ou après les premiers paquets
  vus par `capture/`).
- **Résolution de conflits (5.2)** : un buffer en mémoire (`HashMap<FiveTuple, PendingFlow>`)
  accumule les fragments par clé ; un flux est émis (un seul enregistrement) soit dès qu'au
  moins une source de contenu (decryption/keylog, absentes pour l'instant) confirme la
  fusion, soit après expiration de la fenêtre tolérante avec les sources déjà disponibles
  (capture seul, ou capture+attribution). Jamais un doublon par source.
- **Visibilité (5.3)** — mapping explicite entre sources disponibles et `FlowVisibility`
  (`Fully`/`Meta`/`Attrib`/`Unknown`, déjà dans le contrat IPC `commands/types.rs`) :
  - contenu déchiffré présent (decryption OU keylog — aucune des deux n'existe encore,
    prévu pour rester correct une fois EPIC 3/4 livrés) → `Fully`.
  - capture présente (5-tuple/SNI/protocole vus sur le réseau), pas de contenu → `Meta`.
  - attribution présente (pid/exe connu via `AskRule`) mais capture absente (rare : paquet
    raté par le rate-limiter EPIC 2.5, ou interface non capturée) → `Attrib`.
  - rien de tout ça → `Unknown` (ne devrait normalement jamais être émis comme `Flow` réel,
    réservé aux cas dégénérés/tests).
- **Émission temps réel (5.4)** : remplace l'émetteur factice `spawn_mock_live_flow_emitter`
  (`src-tauri/src/lib.rs`, EPIC 8.4, réservé jusqu'ici) par l'émission réelle de l'événement
  Tauri `vitrail://flow` à chaque `Flow` produit par le moteur de corrélation — contrat
  d'événement déjà consommé côté frontend (`src/timeline/useTimelineFlows.ts`), AUCUN
  changement frontend nécessaire si le payload reste un `Flow` valide. Chaque `Flow` produit
  est aussi persisté dans `storage::flows` (table créée vide en EPIC 6, alimentée pour de
  vrai à partir de cette passe).
- **Remplacement des mocks (`commands/flows.rs`)** : `list_flows`/`get_flow_detail`/
  `search_flows` lisent désormais `storage::flows` au lieu de `mock_flows::flows()` —
  `mock_flows.rs` n'est plus utilisé par `commands/flows.rs` après cette passe (peut rester
  utilisé ailleurs le temps de vérifier, sinon supprimé si mort). `search_flows` peut
  utiliser la table FTS5 `flows_fts` créée en EPIC 6 (encore vide jusqu'ici) — c'est le
  moment de la brancher pour de vrai (5.4 texte + 6.4 recherche, les deux se rejoignent ici).
- **Sources capture/attribution → correlation** : `correlation/` s'abonne aux événements en
  mémoire de `capture/` et `attribution/` (pas de lecture polling de `storage::events` —
  trop tardif/indirect pour du temps réel). Ajoute un canal interne (`std::sync::mpsc` ou
  `tokio::sync::mpsc` selon ce qui s'intègre le mieux avec le reste, à trancher par
  l'implémentation) que `capture::CaptureSubsystem` et `attribution::AttributionSubsystem`
  utilisent pour publier chaque événement retenu vers `correlation/`, en plus de leur
  écriture `storage::events` existante (ne remplace pas la persistance brute déjà en place,
  s'ajoute en parallèle) — MODIFICATION MINIME de ces 2 domaines déjà durcis (juste un envoi
  sur un channel supplémentaire, pas de changement de logique interne).
- **Tests (5.5)** : fixtures combinant capture+attribution dans des ordres et délais variés
  (attribution avant capture, capture avant attribution, capture seul jamais suivi
  d'attribution avant expiration de fenêtre, etc.), vérifie la visibilité assignée et
  l'absence de doublon.

## 6octies. EPIC 3 — Décryptage TLS coopératif SSLKEYLOGFILE (décidé 2026-07-09)

Cinquième domaine réel, avant-dernier de l'ordre décidé. Première source de CONTENU
déchiffré réelle — `correlation/visibility.rs` (EPIC 5) a déjà été construit pour accepter
un signal `decryption`/`keylog` sans réécriture, cette passe l'alimente pour de vrai.

- **Non-réinvention assumée** : `keylog/` ne fait AUCUN parsing TLS/HTTP maison — délègue
  entièrement le déchiffrement et la reconstruction HTTP à `tshark` en sous-processus
  (dissecteurs Wireshark, `-o tls.keylog_file:<chemin>`, sortie `-T ek` = un objet JSON par
  paquet/PDU sur stdout, facile à streamer ligne par ligne comme le fait déjà
  `vitrail-capture-helper`). Pas de re-parsing manuel de sessions TLS (story 3.3 devient donc
  : lire le flux JSON déjà corrélé par tshark, pas suivre le fichier de clés à la main).
- **Dépendance système `tshark` — divergence de privilège assumée** : contrairement à
  `vitrail-capture-helper` (setcap propre à Vitrail), Vitrail ne gère PAS l'élévation de
  `tshark` lui-même — il s'appuie sur le mécanisme standard déjà en place sur la plupart des
  distros pour Wireshark (`dumpcap` avec capacités via le paquet système, groupe `wireshark`).
  Détection (3.1/3.5) : `which tshark` + test réel `tshark -D` (liste les interfaces sans
  capturer) pour vérifier une vraie permission de capture, pas une supposition. État dégradé
  explicite si `tshark` absent ou sans permission — jamais un échec silencieux qui ferait
  croire à une couverture keylog qui n'existe pas.
- **Fichier de clés (3.1)** : `$XDG_DATA_HOME/vitrail/tls_keylog.log`, créé en 600 dès
  l'ouverture (même pattern `OpenOptions` déjà utilisé partout dans le projet), TRONQUÉ à
  chaque activation (jamais d'accumulation de clés entre sessions — cohérent avec la
  discipline de réversibilité/vie privée déjà posée pour les autres domaines).
- **Injection apps ciblées (3.2)** : réutilise EXACTEMENT les commandes IPC déjà existantes
  `list_keylog_apps`/`add_keylog_app`/`remove_keylog_app` (`commands/settings.rs`, jusqu'ici
  mockées en mémoire) — persistées pour de vrai via une nouvelle API `storage::keylog`.
  Pour chaque app de la liste avec un `.desktop` connu : un script wrapper
  `$XDG_DATA_HOME/vitrail/keylog-wrapper.sh` (pose `SSLKEYLOGFILE`, `exec "$@"`) + une
  **copie utilisateur** du `.desktop` dans `$XDG_DATA_HOME/applications/<basename>.desktop`
  (mécanisme XDG standard de surcharge utilisateur, prioritaire sur le `.desktop` système
  SANS jamais le toucher) avec `Exec=` réécrit pour passer par le wrapper. Snapshot de tout
  fichier de surcharge PRÉEXISTANT avant modification (même discipline pré/post que le kill
  switch) pour restaurer l'état exact à la désactivation — jamais une simple suppression
  aveugle qui effacerait une personnalisation de l'utilisateur non liée à Vitrail. Sessions
  shell : pas d'injection possible dans un shell déjà lancé — limite acceptée et déjà
  documentée (`UI_SPEC.md` écran Paramètres > Keylog, story 3.5 "pas de faux sentiment de
  couverture totale"), aucune nouvelle commande IPC nécessaire pour ce sous-cas.
- **Process tshark live (3.3/3.4)** : un seul process `tshark` (pas un par interface — accepte
  plusieurs `-i` en une seule invocation), lancé au `start()` du `Subsystem`, arrêté
  proprement (`SIGTERM`/join, même pattern que `vitrail-capture-helper`) au `stop()`. Parse
  chaque ligne JSON (`-T ek`) en un fragment `DecryptedFragment` (5-tuple, host/method/path/
  status si HTTP présent, headers, `body_preview` tronqué à 2 Ko — même discipline de taille
  que le reste du projet, certificat si TLS présent) et l'envoie vers `correlation/` via un
  nouveau canal (même pattern `mpsc`/`try_send` non-bloquant que `capture/`/`attribution/`
  en EPIC 5, AUCUN risque d'introduire un blocage dans le chemin `AskRule` d'attribution
  puisque ce canal est totalement indépendant).
- **Extension de `correlation/` (EPIC 5)** : `CorrelationEvent` gagne une variante
  `Decryption(DecryptedFragment)`, `PendingFlow`/`engine.rs` fusionnent ce fragment par
  5-tuple exactement comme les 2 sources existantes (même fenêtre 5s), `builder.rs` remplit
  enfin les champs `request_headers`/`response_headers`/`body_preview`/`content_type`/
  `certificate` du `Flow` quand ce fragment est présent, `visibility.rs` reçoit enfin
  `decryption: true` pour de vrai (le paramètre existait déjà, prêt depuis EPIC 5) → `Fully`.
  MODIFICATION CIBLÉE de `correlation/` déjà auditée deux fois — étend sans réécrire.
- **`KeylogSubsystem`** implémente le trait `Subsystem`, remplace le stub "keylog" dans
  `killswitch/subsystem.rs` sans autre modification de l'orchestration (validé 4 fois :
  EPIC 7 → 2 → 1 → 5, même garantie).
- **Tests** : mock du process `tshark` (trait `TsharkBackend` ou équivalent, comme
  `NftablesBackend`/`FakeCaptureSubsystem` déjà dans le projet) — les tests ne doivent jamais
  invoquer le vrai `tshark` (absent sur certaines machines de dev/CI, y compris celle-ci).

## 6nonies. EPIC 4 — Décryptage TLS actif PolarProxy (décidé 2026-07-09)

Dernier domaine réel, le plus risqué du projet : CA système, redirection nftables de TOUT
le trafic 80/443, MITM actif. Contrairement aux EPICs précédents, une régression ici peut
casser la connectivité HTTPS de la machine ENTIÈRE (pas juste une connexion), donc le
garde-fou de secours est non négociable, pas une amélioration optionnelle.

- **CA locale (4.1)** : crate `rcgen` (pur Rust, pas de dépendance OpenSSL système). CA
  dédiée générée dans `$XDG_DATA_HOME/vitrail/ca/` (clé privée 600, jamais réutiliser/modifier
  une CA existante — invariant déjà posé en `docs/PLAN.md` section 5). `vitrail-helper` gagne
  deux sous-commandes allowlistées supplémentaires : `install-ca <chemin-cert>` (déjà
  anticipées en §6bis) et `remove-ca <fingerprint-sha256-exact>` — la suppression cible
  TOUJOURS l'empreinte exacte de la CA installée par Vitrail (jamais un remove générique par
  nom/chemin qui risquerait de supprimer une autre CA système). Mécanisme d'installation :
  détecte `trust` (p11-kit, Arch/Fedora) en priorité, sinon `update-ca-certificates`
  (Debian/Ubuntu) — état dégradé explicite si aucun des deux n'est trouvé, jamais une
  supposition.
- **PolarProxy — dépendance externe non bundlée** : comme `tshark` (EPIC 3), Vitrail ne gère
  pas l'installation de PolarProxy — détection honnête (`which PolarProxy` ou chemin
  connu) + état dégradé explicite si absent. AVANT d'implémenter le câblage CLI exact
  (arguments, format de sortie), rechercher la vraie documentation/CLI de PolarProxy
  (Netresec, dépôt public) plutôt que de deviner — même discipline que la récupération du
  vrai `ui.proto` en EPIC 1 (une divergence honnêtement signalée vaut mieux qu'une
  implémentation qui ne correspond pas au vrai outil).
- **GARDE-FOU ABSOLU (4.2/4.3)** : la règle nftables de redirection 80/443 vers PolarProxy ne
  doit JAMAIS rester active si PolarProxy n'est pas confirmé en train d'écouter. Mécanisme
  requis, non négociable :
  1. La règle de redirection n'est appliquée qu'APRÈS confirmation que PolarProxy écoute
     réellement sur son port local (pas un lancement optimiste du process suivi
     immédiatement de la règle nftables).
  2. Un `AbnormalExitGuard` (même mécanisme Drop-based validé en EPIC 1 pour l'attribution)
     déclenche le retrait IMMÉDIAT de la règle de redirection nftables (pas juste la
     restauration d'une config tierce comme en EPIC 1 — ici c'est le trafic réseau de TOUTE
     la machine qui est en jeu) si le process PolarProxy meurt de façon anormale, AVANT que
     le processus Vitrail ne se termine complètement sur ce chemin.
  3. `4.2` (redémarrage sur crash) : une tentative de relance automatique est acceptable,
     mais tant que PolarProxy n'est pas confirmé de nouveau à l'écoute, la redirection reste
     retirée (jamais de trafic dirigé vers un port mort).
  Un flux HTTPS qui échoue à cause d'une redirection orpheline serait un scénario pire que le
  gel réseau déjà pris au sérieux en EPIC 1 (bloque TOUT le trafic web, pas juste les
  nouvelles connexions en attente d'attribution).
- **nftables réel (4.3)** : `vitrail-helper` gagne `nft-redirect <port-local>` (ajoute des
  règles DNAT `tcp dport {80,443} → 127.0.0.1:<port>` DANS la chaîne `VITRAIL_REDIRECT` déjà
  créée par `nft-apply` en EPIC 7 — jamais une nouvelle chaîne, jamais de règle en dehors) et
  `nft-clear-redirect` (retire uniquement ces règles, laisse la chaîne vide/marqueur intacte,
  cohérent avec la sémantique "chaîne présente = kill switch actif" déjà posée en EPIC 7).
  `<port-local>` validé côté Rust comme un `u16` non privilégié avant tout appel privilégié
  (même discipline que la validation d'adresse socket en EPIC 1).
- **Exclusions (4.5)** : périmètre réaliste — seules les exclusions de type destination/
  domaine (déjà dans le contrat IPC `Exclusion{name, kind}`, `kind == "destination"`) sont
  appliquées en amont nftables via un set nommé (`vitrail-helper` résout le domaine en IP(s)
  localement côté Rust, transmet la liste d'IPs à une nouvelle sous-commande
  `nft-set-exclusions <ip1,ip2,...>` qui peuple un set nftables `except` référencé par la
  règle DNAT). Les exclusions de type `"processus"` restent hors périmètre nftables (aucun
  moyen fiable de filtrer par processus à ce niveau réseau) — documenté explicitement comme
  une limite connue, jamais un faux sentiment de protection.
- **4.4 lecture sortie PolarProxy** : selon ce que la recherche CLI réelle révèle — objectif
  produire des fragments `DecryptedFragment`/équivalent pour `correlation/` (même canal
  `mpsc` que `keylog::DecryptedFragment`, réutilisable tel quel ou étendu légèrement) ET des
  événements `PinningDetected` distincts (jamais mélangés avec du contenu déchiffré) écrits
  dans `storage::events` (nouvelle table si nécessaire, même pattern que les tables
  existantes) pour être visibles dans l'UI (écran Journal système/Confidentialité).
- **4.6 tests** : AUCUN test d'intégration contre une vraie app à pinning réel n'est possible
  en environnement agent (pas de device/app disponible) — couvrir par tests unitaires avec un
  `PolarProxyBackend` fake (même pattern `NftablesBackend`/`TsharkBackend`) simulant un
  process qui meurt pendant que la redirection est active, et vérifier que le garde-fou
  retire bien la règle nftables dans ce scénario simulé. La validation manuelle réelle contre
  une app à pinning reste un test à faire par Chris lui-même sur sa machine — documenter
  clairement cette limite dans le rapport de livraison et dans STATE.md, ne jamais prétendre
  que le fail-open a été validé en conditions réelles.
- **Deux `Subsystem` distincts, pas un seul** : `killswitch/mod.rs` a déjà deux slots stub
  séparés, `"ca"` et `"polarproxy"` (cf. `build_steps`) — remplace CHACUN par une vraie
  implémentation (`CaSubsystem::start()` génère/installe la CA si absente, `stop()` ne fait
  RIEN par défaut — désinstaller la CA à chaque désactivation serait plus agressif que
  nécessaire et cassable si l'utilisateur veut la garder confiante entre deux sessions ;
  documente ce choix, à confirmer avec Chris s'il préfère une désinstallation systématique.
  `PolarProxySubsystem::start()` lance le process + confirme l'écoute + applique la
  redirection nftables ; `stop()` retire la redirection PUIS arrête le process, ordre inverse
  strict). La séquence CA → nftables → PolarProxy → attribution → capture → keylog câblée
  depuis EPIC 7 devient enfin intégralement réelle sur ses 6 étapes.

## 7. Ouvert / à trancher avec Chris

- **Portée réseau réellement voulue** : confirmation que v1 = zéro exposition réseau
  (IPC Tauri pur), et que "accessible depuis le réseau" visait juste l'accès local à la
  machine elle-même, pas un dashboard distant.
- **Blocage vs observation pure** : Vitrail v1 n'écrit aucune règle de blocage lui-même
  (délégué à OpenSnitch) — à confirmer que c'est bien le périmètre voulu.
- **Rétention par défaut** des événements stockés (proposé : 30 jours) et politique de
  purge automatique.
