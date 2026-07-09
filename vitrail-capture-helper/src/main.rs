//! Binaire privilégié `vitrail-capture-helper` — capture AF_PACKET passive uniquement.
//!
//! Reçoit `cap_net_raw,cap_net_admin` via `setcap` à l'installation (PLAN.md §6quater),
//! jamais lancé root. Surface volontairement étroite : détecte les interfaces actives,
//! capture, parse 5-tuple + SNI + protocole best-effort, écrit du JSON Lines sur stdout.
//! Aucune sous-commande, aucune configuration réseau, lecture passive uniquement.

mod capture_thread;
mod output;
mod packet;
mod rate_limiter;
mod tls_sni;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;

use pnet::datalink;

use rate_limiter::TokenBucket;

const DEFAULT_RATE_LIMIT_PPS: u32 = 2000;

fn main() {
    let terminate = Arc::new(AtomicBool::new(false));
    register_sigterm_handler(&terminate);

    let bucket = Arc::new(TokenBucket::new(read_rate_limit()));

    let interfaces = active_interfaces();
    if interfaces.is_empty() {
        eprintln!("vitrail-capture-helper: aucune interface active détectée, sortie");
        std::process::exit(1);
    }

    let handles: Vec<_> = interfaces
        .into_iter()
        .map(|iface| {
            let terminate = Arc::clone(&terminate);
            let bucket = Arc::clone(&bucket);
            thread::spawn(move || capture_thread::run(iface, terminate, bucket))
        })
        .collect();

    for handle in handles {
        if let Err(error) = handle.join() {
            eprintln!("vitrail-capture-helper: thread de capture paniqué: {error:?}");
        }
    }
}

/// Détection dynamique des interfaces actives — aucune interface en dur (story 2.1).
fn active_interfaces() -> Vec<datalink::NetworkInterface> {
    datalink::interfaces()
        .into_iter()
        .filter(|iface| iface.is_up() && !iface.is_loopback())
        .collect()
}

fn read_rate_limit() -> u32 {
    std::env::var("VITRAIL_CAPTURE_RATE_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT_PPS)
}

/// `CaptureSubsystem::stop()` envoie `SIGTERM` au process — ce handler flip un booléen
/// atomique observé par chaque thread de capture entre deux lectures (jamais de kill brutal
/// des threads, arrêt coopératif).
fn register_sigterm_handler(terminate: &Arc<AtomicBool>) {
    if let Err(error) =
        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(terminate))
    {
        eprintln!("vitrail-capture-helper: échec d'enregistrement du handler SIGTERM: {error}");
    }
}
