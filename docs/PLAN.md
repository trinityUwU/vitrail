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

## 7. Ouvert / à trancher avec Chris

- **Portée réseau réellement voulue** : confirmation que v1 = zéro exposition réseau
  (IPC Tauri pur), et que "accessible depuis le réseau" visait juste l'accès local à la
  machine elle-même, pas un dashboard distant.
- **Blocage vs observation pure** : Vitrail v1 n'écrit aucune règle de blocage lui-même
  (délégué à OpenSnitch) — à confirmer que c'est bien le périmètre voulu.
- **Rétention par défaut** des événements stockés (proposé : 30 jours) et politique de
  purge automatique.
