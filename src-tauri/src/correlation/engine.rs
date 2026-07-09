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
        CorrelationEvent::Decryption(fragment) => Some(fragment.five_tuple.clone()),
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

    // Fix audit 5.2 : un fragment `Decryption` qui arrive alors qu'AUCUNE entrée n'est
    // active dans le buffer pour ce 5-tuple est soit une première apparition, soit — cas
    // chronologiquement réaliste (tshark reconstruit après la fin du handshake TLS) — un
    // fragment tardif pour une connexion dont capture+attribution ont déjà fermé et émis un
    // `Flow`. On tente d'abord d'enrichir ce flow déjà émis (`correlation::update`) plutôt que
    // de laisser le chemin normal ci-dessous en créer un second.
    if let CorrelationEvent::Decryption(fragment) = &event {
        if !buffer.contains_key(&five_tuple)
            && super::update::try_enrich_already_emitted(&five_tuple, fragment, storage, emit)
        {
            return;
        }
    }

    let entry = buffer
        .entry(five_tuple.clone())
        .or_insert_with(|| PendingFlow::new(five_tuple.clone()));
    match event {
        CorrelationEvent::Capture(packet) => entry.capture = Some(packet),
        CorrelationEvent::Attribution(attribution_event) => {
            entry.attribution = Some(attribution_event)
        }
        CorrelationEvent::Decryption(fragment) => entry.decryption = Some(fragment),
    }

    // "Au moins une source de contenu confirme la fusion" (5.2, PLAN.md §6octies) : un
    // fragment déchiffré (keylog, EPIC 3) suffit seul à émettre immédiatement, sans attendre
    // capture+attribution ni la fenêtre — sinon, capture+attribution réunies suffisent comme
    // avant EPIC 3.
    let ready = buffer.get(&five_tuple).is_some_and(|pending| {
        pending.decryption.is_some() || (pending.capture.is_some() && pending.attribution.is_some())
    });
    if ready {
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

/// Tests dans `engine_tests.rs` (fichier séparé, même convention que `storage/tests.rs`) —
/// évite de dépasser la limite de 500 lignes/fichier tout en gardant un accès direct aux
/// fonctions privées (`ingest`/`sweep_expired`) via `use super::*;` côté fichier de tests.
#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
