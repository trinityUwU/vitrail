//! Tests white-box du moteur de corrélation (story 5.5, fix audit 5.2) — appellent
//! directement `ingest`/`sweep_expired` plutôt que `spawn` + vrai `sleep(CORRELATION_WINDOW)`
//! — déterministe et rapide (une fenêtre de 5s simulée par manipulation directe de
//! `PendingFlow::first_seen`, jamais par une vraie attente). Storage en mémoire, jamais le
//! vrai fichier `vitrail.db`. Fichier séparé d'`engine.rs` (même convention que
//! `storage/tests.rs`) pour rester sous la limite de 500 lignes/fichier.

use std::sync::Mutex;

use crate::attribution::AttributionEvent;
use crate::capture::CapturedPacket;
use crate::keylog::DecryptedFragment;
use crate::shared::FlowVisibility;

use super::*;

fn five_tuple() -> FiveTuple {
    FiveTuple {
        protocol: "tcp".into(),
        src_ip: "192.168.1.42".into(),
        src_port: 51000,
        dst_ip: "1.2.3.4".into(),
        dst_port: 443,
    }
}

fn capture_packet(tuple: &FiveTuple) -> CapturedPacket {
    CapturedPacket {
        timestamp_unix_ms: 0,
        interface: "eth0".into(),
        protocol: tuple.protocol.clone(),
        src_ip: tuple.src_ip.clone(),
        dst_ip: tuple.dst_ip.clone(),
        src_port: Some(tuple.src_port),
        dst_port: Some(tuple.dst_port),
        bytes: 1024,
        sni: Some("example.com".into()),
        detected_protocol: Some("TLS 1.3".into()),
    }
}

fn attribution_event(tuple: &FiveTuple) -> AttributionEvent {
    AttributionEvent {
        pid: 4242,
        exe_path: "/usr/bin/firefox".into(),
        app_name: "Firefox".into(),
        five_tuple: Some(tuple.clone()),
        timestamp_unix_ms: 0,
    }
}

fn decrypted_fragment(tuple: &FiveTuple) -> DecryptedFragment {
    DecryptedFragment {
        five_tuple: tuple.clone(),
        host: Some("example.com".into()),
        method: Some("GET".into()),
        path: Some("/api".into()),
        status: Some(200),
        request_headers: Vec::new(),
        response_headers: Vec::new(),
        body_preview: None,
        content_type: None,
        certificate: None,
    }
}

fn test_storage() -> StorageHandle {
    StorageHandle::open_in_memory().expect("ouverture storage en mémoire pour le test")
}

/// Collecteur `emit` — évite de dupliquer un `Mutex<Vec<Flow>>` dans chaque test.
struct Collected(Mutex<Vec<Flow>>);

impl Collected {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }
    fn emit(&self, flow: &Flow) {
        self.0
            .lock()
            .expect("mutex collecteur test empoisonné")
            .push(flow.clone());
    }
    fn flows(&self) -> Vec<Flow> {
        self.0
            .lock()
            .expect("mutex collecteur test empoisonné")
            .clone()
    }
}

#[test]
fn attribution_puis_capture_fusionnent_immediatement_sans_attendre_la_fenetre() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    ingest(
        &mut buffer,
        CorrelationEvent::Attribution(attribution_event(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    assert!(
        buffer.contains_key(&tuple),
        "attribution seule doit rester en attente"
    );

    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(
        buffer.is_empty(),
        "la clé doit être retirée du buffer après fusion"
    );
    let flows = collected.flows();
    assert_eq!(
        flows.len(),
        1,
        "une seule fusion doit produire un seul flow"
    );
    assert_eq!(flows[0].visibility, FlowVisibility::Meta);
    assert_eq!(flows[0].process, "Firefox");
}

#[test]
fn capture_puis_attribution_fusionnent_immediatement_dans_l_ordre_inverse() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    ingest(
        &mut buffer,
        CorrelationEvent::Attribution(attribution_event(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(buffer.is_empty());
    let flows = collected.flows();
    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].visibility, FlowVisibility::Meta);
}

#[test]
fn capture_seule_expire_en_meta_apres_la_fenetre() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    expire_entry(&mut buffer, &tuple);
    sweep_expired(
        &mut buffer,
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(buffer.is_empty());
    let flows = collected.flows();
    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].visibility, FlowVisibility::Meta);
    assert_eq!(flows[0].process, "Processus inconnu");
}

#[test]
fn attribution_seule_expire_en_attrib_apres_la_fenetre() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    ingest(
        &mut buffer,
        CorrelationEvent::Attribution(attribution_event(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    expire_entry(&mut buffer, &tuple);
    sweep_expired(
        &mut buffer,
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    let flows = collected.flows();
    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].visibility, FlowVisibility::Attrib);
    assert_eq!(flows[0].process, "Firefox");
}

#[test]
fn aucun_fragment_dans_la_fenetre_ne_produit_aucun_flow() {
    let mut buffer: HashMap<FiveTuple, PendingFlow> = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    sweep_expired(
        &mut buffer,
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(collected.flows().is_empty());
}

#[test]
fn deux_paquets_capture_du_meme_5_tuple_ne_produisent_qu_un_seul_flow() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    // Deux fragments capture avant l'attribution — ne doivent jamais compter comme deux
    // clés distinctes (5.2 : jamais un doublon par source dans la fenêtre).
    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    assert_eq!(
        buffer.len(),
        1,
        "un seul agrégat en attente pour ce 5-tuple"
    );

    ingest(
        &mut buffer,
        CorrelationEvent::Attribution(attribution_event(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    let flows = collected.flows();
    assert_eq!(
        flows.len(),
        1,
        "un seul flow émis malgré les deux fragments capture"
    );
}

#[test]
fn un_fragment_decrypte_seul_fusionne_immediatement_en_fully() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    ingest(
        &mut buffer,
        CorrelationEvent::Decryption(decrypted_fragment(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(
        buffer.is_empty(),
        "un fragment déchiffré seul doit émettre immédiatement, sans attendre capture/attribution"
    );
    let flows = collected.flows();
    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].visibility, FlowVisibility::Fully);
    assert_eq!(flows[0].destination, "example.com");
    assert_eq!(flows[0].method.as_deref(), Some("GET"));
}

#[test]
fn capture_puis_decryption_fusionnent_immediatement_en_fully_avec_process_inconnu() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    assert!(
        buffer.contains_key(&tuple),
        "capture seule doit rester en attente"
    );

    ingest(
        &mut buffer,
        CorrelationEvent::Decryption(decrypted_fragment(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(buffer.is_empty());
    let flows = collected.flows();
    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].visibility, FlowVisibility::Fully);
    assert_eq!(flows[0].process, "Processus inconnu");
}

/// Fix audit 5.2 : reproduit exactement l'ordre chronologique signalé — capture+attribution
/// ferment et émettent (persistent) un `Flow` `Meta`, PUIS un fragment `Decryption` arrive
/// pour le MÊME 5-tuple alors que le buffer actif est déjà vide (comme dans la vraie vie :
/// tshark reconstruit le contenu HTTP après la fin du handshake TLS, souvent après que
/// capture+attribution aient déjà fusionné). Avant le fix, ce second fragment aurait créé
/// un second `Flow` `Fully` en storage pour la même connexion — viole EPICS.md 5.2.
#[test]
fn decryption_tardive_apres_fermeture_capture_attribution_enrichit_au_lieu_de_dupliquer() {
    let tuple = five_tuple();
    let mut buffer = HashMap::new();
    let storage = test_storage();
    let collected = Collected::new();
    let sequence = AtomicU64::new(0);

    // Capture puis attribution ferment et émettent immédiatement un Flow Meta, comme
    // avant ce fix — buffer vidé de cette clé.
    ingest(
        &mut buffer,
        CorrelationEvent::Capture(capture_packet(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    ingest(
        &mut buffer,
        CorrelationEvent::Attribution(attribution_event(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );
    assert!(
        buffer.is_empty(),
        "capture+attribution doivent avoir fusionné et vidé le buffer avant la decryption"
    );
    assert_eq!(
        collected.flows().len(),
        1,
        "un seul flow Meta émis à ce stade"
    );

    // Le fragment déchiffré arrive APRÈS coup, pour le même 5-tuple, sans rien en attente
    // dans le buffer.
    ingest(
        &mut buffer,
        CorrelationEvent::Decryption(decrypted_fragment(&tuple)),
        &storage,
        &|flow| collected.emit(flow),
        &sequence,
    );

    assert!(
        buffer.is_empty(),
        "le fragment tardif ne doit jamais ouvrir une nouvelle entrée de buffer"
    );

    // Un seul enregistrement en storage pour cette connexion (pas deux lignes distinctes).
    let persisted = storage::flows::list_flows(&storage, 10)
        .expect("lecture storage pour vérifier l'absence de doublon");
    assert_eq!(
        persisted.len(),
        1,
        "une même connexion vue par plusieurs sources doit produire UN SEUL enregistrement (EPICS.md 5.2)"
    );
    assert_eq!(persisted[0].visibility, FlowVisibility::Fully);
    assert_eq!(persisted[0].destination, "example.com");
    assert_eq!(persisted[0].method.as_deref(), Some("GET"));
    assert_eq!(
        persisted[0].process, "Firefox",
        "l'attribution déjà connue doit être préservée par l'enrichissement"
    );

    // L'UI doit recevoir la mise à jour en temps réel (ré-émission du flow enrichi), sans
    // que cela crée une seconde ligne : deux émissions IPC, un seul id, une seule ligne DB.
    let emitted = collected.flows();
    assert_eq!(
        emitted.len(),
        2,
        "Meta initial + réémission Fully enrichie, mais toujours le même flow"
    );
    assert_eq!(emitted[0].id, emitted[1].id);
    assert_eq!(emitted[1].visibility, FlowVisibility::Fully);
}

/// Recule `first_seen` au-delà de `CORRELATION_WINDOW` pour simuler l'expiration sans
/// vrai `sleep` dans les tests.
fn expire_entry(buffer: &mut HashMap<FiveTuple, PendingFlow>, tuple: &FiveTuple) {
    let pending = buffer
        .get_mut(tuple)
        .expect("entrée attendue dans le buffer");
    pending.first_seen = std::time::Instant::now()
        .checked_sub(CORRELATION_WINDOW + Duration::from_secs(1))
        .expect("horloge monotone insuffisante pour ce test");
}
