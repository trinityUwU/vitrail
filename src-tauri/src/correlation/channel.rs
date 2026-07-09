//! Canal interne `capture/`/`attribution/` → `correlation/` (PLAN.md §6septies) —
//! `std::sync::mpsc::sync_channel` : `try_send` est non-bloquant (aucune I/O, aucun
//! `.await`), utilisable aussi bien depuis un `std::thread` classique (`capture/`) que
//! depuis une tâche `tokio` (`attribution/`, RPC `AskRule`/`PostAlert`) sans jamais retarder
//! l'appelant. Un canal plein ou fermé (récepteur arrêté, corrélation non démarrée) ne fait
//! jamais échouer ni paniquer l'émetteur — c'est non critique, `storage::capture_events`/
//! `storage::attribution_state` restent la source de vérité brute.

use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};

use crate::attribution::AttributionEvent;
use crate::capture::CapturedPacket;
use crate::keylog::DecryptedFragment;

/// Capacité du canal — un débordement (rafale de connexions) dégrade seulement la
/// corrélation temps réel, jamais la capture/l'attribution/le keylog eux-mêmes.
const CHANNEL_CAPACITY: usize = 1024;

pub enum CorrelationEvent {
    Capture(CapturedPacket),
    Attribution(AttributionEvent),
    /// EPIC 3 (PLAN.md §6octies) : fragment déchiffré produit par `tshark` (pipeline
    /// SSLKEYLOGFILE) — publié en plus de `storage::events`, jamais à la place.
    Decryption(DecryptedFragment),
}

#[derive(Clone)]
pub struct CorrelationSender(SyncSender<CorrelationEvent>);

impl CorrelationSender {
    pub fn send_capture(&self, packet: CapturedPacket) {
        self.try_send_logged(CorrelationEvent::Capture(packet), "capture");
    }

    pub fn send_attribution(&self, event: AttributionEvent) {
        self.try_send_logged(CorrelationEvent::Attribution(event), "attribution");
    }

    pub fn send_decryption(&self, fragment: DecryptedFragment) {
        self.try_send_logged(CorrelationEvent::Decryption(fragment), "keylog");
    }

    fn try_send_logged(&self, event: CorrelationEvent, origin: &'static str) {
        match self.0.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                tracing::warn!(
                    origin,
                    "canal de corrélation saturé, événement ignoré (non critique)"
                );
            }
            Err(TrySendError::Disconnected(_)) => {
                tracing::debug!(
                    origin,
                    "moteur de corrélation non démarré/arrêté, événement ignoré"
                );
            }
        }
    }
}

/// Crée la paire émetteur/récepteur — l'émetteur est cloné dans `capture::CaptureSubsystem`
/// et `attribution::AttributionSubsystem`, le récepteur est consommé une seule fois par
/// `correlation::spawn`.
pub fn channel() -> (CorrelationSender, Receiver<CorrelationEvent>) {
    let (tx, rx) = sync_channel(CHANNEL_CAPACITY);
    (CorrelationSender(tx), rx)
}
