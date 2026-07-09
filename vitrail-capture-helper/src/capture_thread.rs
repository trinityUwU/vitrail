//! Boucle de capture par interface : ouvre un canal AF_PACKET, lit avec un timeout court
//! (pour réagir à SIGTERM sans blocage indéfini), applique le token-bucket, parse chaque
//! paquet retenu et l'écrit en JSON Lines sur stdout.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pnet::datalink::{self, Channel, NetworkInterface};

use crate::output;
use crate::packet::parse_ethernet_frame;
use crate::rate_limiter::TokenBucket;

const READ_TIMEOUT: Duration = Duration::from_millis(200);
const DROP_LOG_INTERVAL: Duration = Duration::from_secs(10);

pub fn run(interface: NetworkInterface, terminate: Arc<AtomicBool>, bucket: Arc<TokenBucket>) {
    let config = datalink::Config {
        read_timeout: Some(READ_TIMEOUT),
        ..datalink::Config::default()
    };

    let mut rx = match open_receiver(&interface, config) {
        Some(rx) => rx,
        None => return,
    };

    let dropped = AtomicU64::new(0);
    let mut last_drop_log = Instant::now();

    while !terminate.load(Ordering::SeqCst) {
        match rx.next() {
            Ok(raw_frame) => {
                handle_frame(raw_frame, &interface.name, &bucket, &dropped);
                maybe_log_drops(&dropped, &mut last_drop_log, &interface.name);
            }
            Err(error) if is_timeout(&error) => continue,
            Err(error) => {
                tracing_eprintln(&interface.name, &error);
                break;
            }
        }
    }
}

fn open_receiver(
    interface: &NetworkInterface,
    config: datalink::Config,
) -> Option<Box<dyn datalink::DataLinkReceiver>> {
    match datalink::channel(interface, config) {
        Ok(Channel::Ethernet(_tx, rx)) => Some(rx),
        Ok(_) => {
            eprintln!(
                "vitrail-capture-helper: type de canal non supporté sur {}",
                interface.name
            );
            None
        }
        Err(error) => {
            eprintln!(
                "vitrail-capture-helper: ouverture du canal {} échouée: {error}",
                interface.name
            );
            None
        }
    }
}

fn handle_frame(raw_frame: &[u8], interface_name: &str, bucket: &TokenBucket, dropped: &AtomicU64) {
    if !bucket.try_acquire() {
        dropped.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if let Some(record) = parse_ethernet_frame(raw_frame, interface_name) {
        output::write_record(&record);
    }
}

/// Un log par paquet perdu saturerait stderr sous forte charge (story 2.5) — agrégation sur
/// `DROP_LOG_INTERVAL`, silence total si rien n'a été droppé sur la période.
fn maybe_log_drops(dropped: &AtomicU64, last_log: &mut Instant, interface_name: &str) {
    if last_log.elapsed() < DROP_LOG_INTERVAL {
        return;
    }
    let count = dropped.swap(0, Ordering::Relaxed);
    if count > 0 {
        eprintln!(
            "vitrail-capture-helper: {count} paquets droppés (rate-limit) sur {interface_name}"
        );
    }
    *last_log = Instant::now();
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
    )
}

fn tracing_eprintln(interface_name: &str, error: &std::io::Error) {
    eprintln!("vitrail-capture-helper: erreur de lecture sur {interface_name}: {error}");
}
