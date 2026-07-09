//! Moteur de corrélation (story 5.1/5.2/5.4, PLAN.md §6septies) — tourne dans son propre
//! thread, accumule les fragments capture/attribution dans un buffer en mémoire keyé par
//! 5-tuple, émet un `Flow` unique par clé (jamais de doublon dans la fenêtre) soit dès que
//! capture+attribution sont réunies, soit à expiration de la fenêtre tolérante.
//!
//! Pas un `Subsystem` (killswitch/subsystem.rs) : `correlation/` n'apparaît pas dans la
//! séquence d'activation EPICS.md 7.2 (CA → nftables → PolarProxy → attribution → capture →
//! keylog) — décision non explicite dans PLAN.md, tranchée ainsi (cf. rapport EPIC 5) car le
//! moteur n'a pas d'état système à activer/désactiver (aucun nftables/CA/process
//! privilégié) : il tourne pour toute la durée de vie de l'app, se contente d'accumuler ce
//! que `capture`/`attribution` lui envoient (rien s'ils sont inactifs). Le démarrer/l'arrêter
//! avec le kill switch forcerait à bufferiser ou jeter des événements pendant qu'il est
//! "arrêté", sans bénéfice : aucune ressource sensible n'est en jeu.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::attribution::FiveTuple;
use crate::capture::CapturedPacket;
use crate::shared::Flow;
use crate::storage::{self, StorageHandle};

use super::builder::{build_flow, PendingFlow};
use super::channel::CorrelationEvent;

/// Fenêtre tolérante de fusion (story 5.1) — les timestamps capture/attribution ne sont
/// jamais strictement synchrones.
pub const CORRELATION_WINDOW: Duration = Duration::from_secs(5);
/// Cadence de purge des entrées expirées — assez court pour une émission "temps réel"
/// perçue, assez long pour ne pas saturer le thread en boucle serrée.
const SWEEP_INTERVAL: Duration = Duration::from_millis(500);

pub struct EngineHandle {
    /// Jamais lu en production : le moteur tourne pour toute la durée de vie de l'app (cf.
    /// doc de module), le handle est juste abandonné (le thread continue, un `JoinHandle`
    /// droppé ne l'arrête pas) — conservé pour un futur arrêt propre (EPIC 9 watchdog ?).
    #[allow(dead_code)]
    thread: Option<JoinHandle<()>>,
}

/// Démarre le moteur dans son propre thread. `emit` est appelé pour chaque `Flow` produit
/// (persistance déjà faite avant l'appel) — `lib.rs` y branche l'émission Tauri
/// `vitrail://flow`, les tests y branchent une simple collecte en `Vec`.
pub fn spawn(
    receiver: Receiver<CorrelationEvent>,
    storage: StorageHandle,
    emit: impl Fn(&Flow) + Send + 'static,
) -> EngineHandle {
    let thread = std::thread::spawn(move || run_loop(receiver, storage, emit));
    EngineHandle {
        thread: Some(thread),
    }
}

fn run_loop(receiver: Receiver<CorrelationEvent>, storage: StorageHandle, emit: impl Fn(&Flow)) {
    let mut buffer: HashMap<FiveTuple, PendingFlow> = HashMap::new();
    let sequence = AtomicU64::new(0);
    loop {
        match receiver.recv_timeout(SWEEP_INTERVAL) {
            Ok(event) => ingest(&mut buffer, event, &storage, &emit, &sequence),
            Err(RecvTimeoutError::Timeout) => {}
            // Tous les émetteurs (capture/attribution) sont clonés et vivent pour la durée de
            // vie de l'app côté normal ; un canal fermé ne se produit qu'en test ou à l'arrêt
            // complet du process — sortie propre de la boucle.
            Err(RecvTimeoutError::Disconnected) => break,
        }
        sweep_expired(&mut buffer, &storage, &emit, &sequence);
    }
}

/// Construit la clé de fusion d'un événement capture/attribution — `None` si la source ne
/// porte pas de 5-tuple exploitable (paquet non TCP/UDP sans port, ou connexion attribution
/// sans 5-tuple) : rien à corréler, l'événement est silencieusement ignoré par la
/// corrélation (il reste par ailleurs déjà persisté dans son propre domaine).
fn event_five_tuple(event: &CorrelationEvent) -> Option<FiveTuple> {
    match event {
        CorrelationEvent::Capture(packet) => capture_five_tuple(packet),
        CorrelationEvent::Attribution(attribution_event) => attribution_event.five_tuple.clone(),
    }
}

fn capture_five_tuple(packet: &CapturedPacket) -> Option<FiveTuple> {
    Some(FiveTuple {
        protocol: packet.protocol.to_lowercase(),
        src_ip: crate::attribution::normalize_ip(&packet.src_ip),
        src_port: packet.src_port?,
        dst_ip: crate::attribution::normalize_ip(&packet.dst_ip),
        dst_port: packet.dst_port?,
    })
}

fn ingest(
    buffer: &mut HashMap<FiveTuple, PendingFlow>,
    event: CorrelationEvent,
    storage: &StorageHandle,
    emit: &impl Fn(&Flow),
    sequence: &AtomicU64,
) {
    let Some(five_tuple) = event_five_tuple(&event) else {
        return;
    };

    let entry = buffer
        .entry(five_tuple.clone())
        .or_insert_with(|| PendingFlow::new(five_tuple.clone()));
    match event {
        CorrelationEvent::Capture(packet) => entry.capture = Some(packet),
        CorrelationEvent::Attribution(attribution_event) => {
            entry.attribution = Some(attribution_event)
        }
    }

    // "Toutes les sources actuellement disponibles" (5.2) = capture + attribution tant que
    // decryption/keylog n'existent pas : les deux réunies suffisent à émettre immédiatement,
    // sans attendre l'expiration de la fenêtre.
    if buffer
        .get(&five_tuple)
        .is_some_and(|pending| pending.capture.is_some() && pending.attribution.is_some())
    {
        if let Some(pending) = buffer.remove(&five_tuple) {
            emit_flow(pending, storage, emit, sequence);
        }
    }
}

fn sweep_expired(
    buffer: &mut HashMap<FiveTuple, PendingFlow>,
    storage: &StorageHandle,
    emit: &impl Fn(&Flow),
    sequence: &AtomicU64,
) {
    let expired: Vec<FiveTuple> = buffer
        .iter()
        .filter(|(_, pending)| pending.first_seen.elapsed() >= CORRELATION_WINDOW)
        .map(|(tuple, _)| tuple.clone())
        .collect();

    for tuple in expired {
        if let Some(pending) = buffer.remove(&tuple) {
            emit_flow(pending, storage, emit, sequence);
        }
    }
}

fn emit_flow(
    pending: PendingFlow,
    storage: &StorageHandle,
    emit: &impl Fn(&Flow),
    sequence: &AtomicU64,
) {
    let seq = sequence.fetch_add(1, Ordering::SeqCst);
    let flow = build_flow(&pending, seq);
    if let Err(error) = storage::flows::insert_flow(storage, &flow) {
        tracing::error!(error = %error, flow_id = %flow.id, "persistance d'un flow corrélé échouée");
    }
    emit(&flow);
}

/// Tests white-box (story 5.5) : appellent directement `ingest`/`sweep_expired` plutôt que
/// `spawn` + vrai `sleep(CORRELATION_WINDOW)` — déterministe et rapide (une fenêtre de 5s
/// simulée par manipulation directe de `PendingFlow::first_seen`, jamais par une vraie
/// attente). Storage en mémoire, jamais le vrai fichier `vitrail.db`.
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::attribution::AttributionEvent;
    use crate::capture::CapturedPacket;
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
}
