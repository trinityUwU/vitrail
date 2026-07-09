# Vitrail — Spécification fonctionnelle de l'interface

**Portée de ce document : le QUOI, jamais le COMMENT visuel.** Zéro couleur, zéro layout,
zéro composant nommé. Chaque section liste ce qui doit être affichable, pilotable, et dans
quel état. Sert de base à un mockup produit séparément (GLM 5.2), à intégrer ensuite dans
le frontend Tauri (`EPIC 8`).

Convention : pour chaque écran — **Objectif**, **Données affichées**, **Actions
disponibles**, **États**, **Réglages liés**.

---

## 0. Cadre général (transverse à tous les écrans)

- Un indicateur d'état global toujours visible : Vitrail actif / inactif / en transition
  (activation ou désactivation en cours) / dégradé (un sous-système est down mais l'app
  tourne quand même).
- Un accès permanent au kill switch (pas enterré dans un sous-menu — c'est la fonction de
  sécurité centrale de l'outil).
- Distinction visuelle systématique du **niveau de visibilité** d'un flux, partout où un
  flux est listé : entièrement déchiffré / métadonnées seules (pinning détecté) / attribué
  sans contenu (pas de TLS) / inconnu. Ce n'est pas une info secondaire, c'est une donnée
  de confiance que l'utilisateur doit voir en un coup d'œil sur chaque ligne.

---

## 1. Écran — Vue d'ensemble (dashboard)

**Objectif** : réponse en 3 secondes à "qu'est-ce qui se passe là, maintenant, sur ma
machine".

**Données affichées**
- Statut du kill switch (actif depuis quand, ou inactif).
- Compteur de connexions actives en temps réel.
- Débit entrant/sortant instantané et cumulé depuis l'activation.
- Top processus par volume (5-10), avec leur niveau de visibilité dominant.
- Top destinations (domaines/IP) par volume.
- Nombre de flux en `MetadataOnly` (pinning détecté) depuis l'activation — compteur dédié,
  car c'est une info de couverture importante.
- Bannière d'alerte si un sous-système attendu est down (OpenSnitch non détecté, PolarProxy
  crashé, etc.) — jamais silencieux.
- Dernière vérification de kill switch (date, résultat propre/divergence détectée).

**Actions disponibles**
- Activer / désactiver Vitrail (le bouton principal, dupliqué du panneau kill switch
  dédié pour accès rapide).
- Accès direct "voir tout" vers la Timeline filtrée sur la fenêtre temporelle courante.
- Clic sur un processus/destination du top → drill-down vers sa vue dédiée.

**États**
- Vide (juste activé, pas encore de données).
- Actif normal.
- Dégradé (un sous-système manque).
- Inactif (rien à afficher, juste un état "prêt à activer" + dernier résumé de session).

**Réglages liés** : aucun réglage direct ici, tout est en lecture.

---

## 2. Écran — Timeline temps réel

**Objectif** : flux brut de tous les événements réseau, façon "console de vol".

**Données affichées** (par ligne)
- Timestamp, processus (icône/nom résolu + pid), destination (domaine si connu sinon IP),
  port, protocole détecté, taille, niveau de visibilité, durée de la connexion (vivante ou
  terminée).

**Actions disponibles**
- Filtres combinables : par processus, par domaine/IP, par port, par protocole, par niveau
  de visibilité, par plage temporelle.
- Tri par n'importe quelle colonne.
- Pause/reprise du flux temps réel (pour pouvoir inspecter sans que ça défile).
- Clic sur une ligne → Inspecteur de flux (écran 5).
- Export de la sélection filtrée courante (JSON/CSV).
- Recherche texte libre (matche domaine, process, et — si déchiffré — contenu).

**États**
- Flux vide (rien ne correspond aux filtres actifs).
- Flux en pause.
- Vitrail inactif (écran accessible mais message clair : "activez Vitrail pour voir du
  trafic").

**Réglages liés** : filtres par défaut sauvegardés (dernier état de filtre restauré à la
réouverture), taille de page/pagination.

---

## 3. Écran — Vue par processus

**Objectif** : "qu'est-ce que fait cette application sur le réseau".

**Données affichées**
- Liste de tous les processus ayant eu de l'activité réseau depuis l'activation (nom
  résolu, chemin binaire exact, pid actuel(s) si plusieurs instances, volume total,
  nombre de destinations distinctes, niveau de visibilité dominant).
- Vue détail par processus : historique de volume dans le temps (graphe temporel), liste
  de toutes les destinations contactées avec fréquence, liste des flux individuels (lien
  vers Timeline filtrée sur ce processus), statut de couverture keylog (ce process
  exporte-t-il ses clés TLS ou pas — cf. EPIC 3.5).

**Actions disponibles**
- Recherche/filtre par nom de processus.
- Ajouter ce processus à la liste d'exclusion (jamais intercepté par le MITM) directement
  depuis sa fiche.
- Clic sur une destination → Vue par destination filtrée croisée avec ce processus.

**États**
- Processus mort mais dont l'historique reste consultable (badge "terminé").
- Processus inconnu/non résolu (juste un chemin binaire brut, pas de nom lisible).

**Réglages liés** : liste d'exclusion (lecture + ajout depuis ici, gestion complète dans
Paramètres).

---

## 4. Écran — Vue par destination

**Objectif** : "qui contacte quoi, côté monde extérieur" — miroir de la vue processus.

**Données affichées**
- Liste des domaines/IP contactés (nom résolu si dispo, sinon IP brute), volume total,
  nombre de processus distincts qui la contactent, niveau de visibilité dominant,
  première/dernière connexion vue.
- Vue détail par destination : tous les processus qui la contactent, historique temporel,
  ports utilisés, statut TLS (certificat vu, pinning détecté ou non).

**Actions disponibles**
- Recherche/filtre par domaine/IP.
- Ajouter cette destination à la liste d'exclusion.
- Marquer une destination comme "connue/fiable" ou "à surveiller" (tag utilisateur libre,
  purement informatif, sert aussi de base aux alertes — écran 7).

**États** : identiques à la vue processus (destination plus contactée = historique gelé).

**Réglages liés** : liste d'exclusion, tags utilisateur.

---

## 5. Écran — Inspecteur de flux (détail d'une connexion)

**Objectif** : le niveau de détail maximal sur un flux précis.

**Données affichées**
- 5-tuple complet, timestamps précis (début/fin), processus attribuant (avec lien vers sa
  fiche), destination (avec lien vers sa fiche).
- Si `FullyDecrypted` : requête/réponse complète — méthode, chemin, headers, aperçu du
  corps (avec garde-fou taille — pas de rendu de payloads binaires énormes en brut),
  content-type, code de statut si HTTP.
- Si `MetadataOnly` : bannière explicite "pinning détecté — contenu non visible", avec
  SNI, taille, durée quand même affichés.
- Si `AttributedOnly` : juste 5-tuple + process + protocole détecté (pas de TLS impliqué).
- Certificat vu côté connexion (émetteur, validité) quand disponible, y compris pour les
  flux en fail-open (utile de savoir *pourquoi* le pinning a été détecté).
- Source(s) ayant contribué à cet enregistrement (attribution / capture / decryption /
  keylog) — traçabilité de la corrélation, utile en debug et en confiance utilisateur.

**Actions disponibles**
- Copier en clipboard (headers, corps, 5-tuple).
- Exporter ce flux unique (JSON).
- Naviguer vers le flux précédent/suivant du même processus ou de la même destination.

**États**
- Contenu tronqué (corps trop volumineux, affiche un lien "voir tout" ou export plutôt que
  rendu inline).
- Flux encore actif (streaming du contenu en cours) vs terminé.

**Réglages liés** : taille max de corps affichée inline (le reste en export).

---

## 6. Écran — Recherche & filtres avancés

**Objectif** : requêtage libre sur l'historique stocké, au-delà des filtres rapides de la
Timeline.

**Données affichées** : constructeur de requête (combinaison de critères : process, domaine,
port, protocole, plage temporelle, niveau de visibilité, recherche plein texte sur contenu
déchiffré) + résultats sous forme de liste façon Timeline.

**Actions disponibles**
- Sauvegarder une requête comme "vue" nommée réutilisable.
- Exporter les résultats.
- Transformer une requête sauvegardée en règle d'alerte (écran 7).

**États** : requête vide (aucun résultat tant qu'aucun critère n'est posé, pour éviter de
charger toute la base par défaut).

**Réglages liés** : requêtes sauvegardées (CRUD complet).

---

## 7. Écran — Alertes & règles

**Objectif** : signalement proactif sans que l'utilisateur ait à regarder la Timeline en
continu.

**Données affichées**
- Liste des règles actives (ex. "notifier à la première connexion d'un nouveau processus
  jamais vu", "notifier si une destination taguée 'à surveiller' est contactée", "notifier
  si un processus connu pour être en `MetadataOnly` passe soudain en `FullyDecrypted`" —
  ce dernier cas est un signal fort de dégradation de sécurité côté app, à documenter
  clairement dans l'aide contextuelle).
- Historique des alertes déclenchées (avec lien direct vers le flux concerné).

**Actions disponibles**
- Créer/modifier/supprimer une règle (constructeur de critères, réutilise le même moteur
  que la Recherche avancée).
- Activer/désactiver une règle sans la supprimer.
- Marquer une alerte comme traitée/ignorée.

**États** : aucune règle définie (état initial, message d'invitation à en créer une plutôt
qu'un écran vide sans explication).

**Réglages liés** : canal de notification (notification desktop système uniquement en v1 —
pas d'email/webhook, ça sortirait du périmètre "outil local").

---

## 8. Écran — Panneau Kill Switch

**Objectif** : la garantie centrale du projet rendue visible et vérifiable, pas juste un
bouton on/off dans un coin.

**Données affichées**
- État détaillé de **chaque sous-système** séparément (pas juste un état global) :
  attribution (OpenSnitch détecté/actif), capture, décryptage actif (PolarProxy), keylog,
  règle nftables, CA installée.
- Horodatage de la dernière activation/désactivation.
- Dernier rapport de vérification post-désactivation (propre / divergences listées).
- Historique des activations/désactivations (log d'audit consultable).

**Actions disponibles**
- Activer / désactiver (séquence normale, orchestrée, cf. EPIC 7.2/7.3).
- **Arrêt d'urgence** distinct (EPIC 7.5) — bouton visuellement séparé du bouton normal
  pour éviter tout clic accidentel, mais accessible immédiatement (jamais enterré dans un
  sous-menu).
- Relancer manuellement une vérification d'état système sans passer par un cycle complet
  activation/désactivation.

**États**
- Transition en cours (activation/désactivation) — chaque étape de la séquence affichée
  avec son statut individuel (pas une simple barre de progression opaque).
- Échec partiel (une étape a échoué) — affichage explicite de quelle étape, pas un message
  générique.

**Réglages liés** : aucun réglage ici — action pure, la config vit dans l'écran Paramètres.

---

## 9. Écran — Paramètres

**Objectif** : tout ce qui est configurable, en un seul endroit organisé par section.

### 9.1 Section CA & TLS
- Empreinte de la CA locale actuelle (affichage lecture seule + copie).
- Action : régénérer la CA (avec avertissement clair sur les conséquences — désactive
  Vitrail le temps de l'opération).
- Statut d'installation dans le trust store système.

### 9.2 Section réseau / nftables
- Nom de la chaîne utilisée (affiché, non modifiable en v1 pour éviter les erreurs — un
  nom fixe documenté).
- Interfaces surveillées (sélection parmi les interfaces détectées, avec option "toutes").

### 9.3 Section exclusions
- Liste complète des process/domaines exclus du MITM, gérée en un seul endroit (CRUD),
  reflète et centralise les ajouts faits depuis les écrans 3/4.

### 9.4 Section rétention & stockage
- Politique de rétention (durée, ou illimité avec avertissement sur la taille disque).
- Taille actuelle de la base de données.
- Action de purge manuelle (totale ou par plage de dates).
- Export/import de configuration (pas des données — juste la config).

### 9.5 Section couverture SSLKEYLOGFILE
- Liste des applications actuellement câblées pour exporter leurs clés (cf. EPIC 3.5),
  action d'ajout/retrait d'une application à couvrir.

### 9.6 Section notifications
- Canal (desktop uniquement v1), activation/désactivation globale des alertes.

### 9.7 Section à propos / diagnostics
- Version de Vitrail, versions des sous-systèmes détectées (OpenSnitch, PolarProxy),
  lien vers les logs bruts (écran 11), lien vers la doc de confidentialité (écran 10).

---

## 10. Écran — Confidentialité & gouvernance des données

**Objectif** : transparence explicite sur ce que l'outil fait de ce qu'il voit — cohérent
avec l'éthique du projet (un outil de surveillance de son propre trafic doit être
irréprochable sur ce point).

**Données affichées**
- Explication claire, non technique : tout reste local, rien n'est jamais envoyé nulle
  part, emplacement exact des fichiers (CA, base de données, clés TLS).
- Ce qui est stocké en clair sur disque (contenu déchiffré) vs ce qui ne l'est jamais.

**Actions disponibles**
- Purge totale immédiate de toutes les données (raccourci vers 9.4 mais mis en avant ici).

**États** : aucun, page essentiellement statique + données dynamiques de taille/emplacement.

---

## 11. Écran — Journal système / logs bruts

**Objectif** : debug, transparence technique, support communautaire (repo public → les
utilisateurs vont demander de l'aide avec ces logs).

**Données affichées**
- Logs structurés de chaque sous-système (attribution, capture, decryption, keylog,
  killswitch) avec niveau (info/warn/error), filtrable par sous-système et par niveau.

**Actions disponibles**
- Copier/exporter les logs (pour un rapport de bug).
- Purge des logs.

**États** : logs vides (juste démarré).

**Réglages liés** : niveau de verbosité par sous-système.

---

## 12. Écran — Historique / rétrospective

**Objectif** : revoir une session passée sans devoir tout garder affiché en permanence.

**Données affichées**
- Liste des sessions passées (période d'activation → désactivation, résumé : volume total,
  nombre de process distincts, nombre d'alertes déclenchées).
- Vue détail d'une session : mêmes écrans que le direct (Timeline, vue process/destination)
  mais figés sur cette fenêtre temporelle.

**Actions disponibles**
- Générer un rapport exportable d'une session (résumé synthétique, pas juste un dump brut).
- Supprimer une session de l'historique (purge ciblée).

**États** : aucune session passée (première utilisation).

---

## 13. Écran — Onboarding / première installation

**Objectif** : parcours guidé la toute première fois, avant même le premier dashboard vide.

**Étapes à couvrir** (checklist, chaque étape avec statut vérifié/en attente/échoué)
1. Vérification qu'OpenSnitch est installé et son daemon actif (lien vers instructions
   d'installation si absent, jamais un blocage muet). Si le daemon est déjà configuré pour
   parler à une autre UI (GUI officielle OpenSnitch notamment), avertissement explicite :
   activer Vitrail reconfigure le daemon pour lui parler exclusivement, l'autre UI cessera
   de recevoir des événements tant que Vitrail est actif (restauré à la désactivation).
2. Génération de la CA locale + explication de ce que ça implique avant toute action.
3. Vérification des permissions nécessaires (nftables/CA) et déclenchement de l'élévation
   de privilèges au bon moment, jamais en surprise.
4. Test à blanc (dry run) : activation courte, vérification que du trafic est bien capturé
   et attribué, désactivation, affichage du diff de vérification — l'utilisateur voit la
   garantie de réversibilité fonctionner avant de faire confiance à l'outil en usage
   prolongé.
5. Résumé final + lien vers le dashboard.

**États** : chaque étape peut échouer indépendamment, avec un message actionnable (pas
juste "erreur"), et la possibilité de relancer l'étape sans recommencer tout le parcours.

---

## Récapitulatif — objets de données transverses

Ces objets apparaissent dans plusieurs écrans ; l'UI doit les représenter de façon
cohérente partout (mêmes libellés, mêmes badges de visibilité) :

- **Flux** (connexion réseau individuelle, unité de base de la Timeline et de l'Inspecteur).
- **Processus** (identité applicative, agrégat de flux).
- **Destination** (domaine/IP, agrégat de flux côté distant).
- **Règle d'exclusion** (process ou domaine jamais intercepté).
- **Règle d'alerte** (critère + action de notification).
- **Session** (fenêtre activation→désactivation, unité de l'historique).
- **Rapport de vérification** (diff pré/post kill switch).
