//! Sortie PolarProxy → corrélation (story 4.4). Deux flux indépendants, jamais mélangés :
//!
//! 1. **Contenu déchiffré** : `tshark` rejoue le flux PCAP-over-IP exposé par PolarProxy
//!    (`--pcapoverip <port>`, tshark `-i TCP@127.0.0.1:<port>`) et produit `-T ek` — RÉUTILISE
//!    `keylog::parse_ek_line` telle quelle (non-réinvention, précédent déjà établi
//!    `attribution::normalize_ip` consommé par `keylog/`). Publié vers `correlation/` via
//!    `CorrelationSender::send_decryption`, identique au pipeline EPIC 3.
//! 2. **Pinning détecté** : PolarProxy n'expose AUCUN signal dédié de pinning (recherche EPIC
//!    4 — PAS de mécanisme "fail-open réactif" documenté). Heuristique best-effort assumée :
//!    le fichier de flux `-f` liste TOUTE session externe interceptée (5-tuple + domaine) ;
//!    toute entrée sans fragment déchiffré correspondant dans la fenêtre `PINNING_WINDOW` est
//!    traitée comme un signal de pinning probable (handshake client→PolarProxy jamais abouti).
//!    Format exact de `-f` NON vérifié contre une sortie réelle (parsing volontairement
//!    tolérant, jamais de panic) — à valider par Chris, cf. rapport de livraison.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::attribution::FiveTuple;
use crate::correlation::CorrelationSender;
use crate::keylog::parse_ek_line;
use crate::storage::decryption::{self as storage_decryption, PinningEvent};
use crate::storage::StorageHandle;

const PINNING_WINDOW: Duration = Duration::from_secs(10);
const FLOWLOG_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Fragments déchiffrés récemment vus, partagés entre le thread de contenu (écrit) et le
/// thread de veille du flowlog (lit) — clé de corrélation identique à `correlation/` (5-tuple).
/// Élagué opportunément à chaque insertion pour ne jamais croître sans borne sur une session
/// longue (volumétrie mono-utilisateur desktop, pas un souci de performance ici).
type SeenFragments = Arc<Mutex<HashMap<FiveTuple, Instant>>>;

/// Handles des deux threads de sortie — conservés (pas de join explicite dans cette passe, le
/// pipeline vit pour la durée du process PolarProxy et se termine naturellement avec lui) pour
/// un futur arrêt propre explicite (EPIC 8) plutôt qu'un `let _ = spawn(...)` qui perdrait
/// silencieusement la référence.
pub struct OutputPipeline {
    #[allow(dead_code)]
    pub content_thread: std::thread::JoinHandle<()>,
    #[allow(dead_code)]
    pub flowlog_thread: std::thread::JoinHandle<()>,
}

/// Démarre les deux threads de sortie — `pcapoverip_port` doit déjà être confirmé actif par
/// l'appelant (`PolarProxySubsystem::start()`, garde-fou 4.2) avant cet appel.
pub fn spawn_output_pipeline(
    pcapoverip_port: u16,
    flowlog_path: PathBuf,
    correlation: CorrelationSender,
    storage: StorageHandle,
) -> OutputPipeline {
    let seen: SeenFragments = Arc::new(Mutex::new(HashMap::new()));

    let seen_for_content = seen.clone();
    let content_thread = std::thread::spawn(move || {
        run_content_pipeline(pcapoverip_port, correlation, seen_for_content)
    });

    let flowlog_thread =
        std::thread::spawn(move || run_flowlog_watcher(flowlog_path, storage, seen));

    OutputPipeline {
        content_thread,
        flowlog_thread,
    }
}

/// Lance `tshark -i TCP@127.0.0.1:<port> -T ek` — se termine naturellement quand le flux
/// PCAP-over-IP se ferme (PolarProxy arrêté). Ligne invalide = ignorée (jamais fatale, même
/// discipline que `keylog::parser`).
fn run_content_pipeline(pcapoverip_port: u16, correlation: CorrelationSender, seen: SeenFragments) {
    let interface = format!("TCP@127.0.0.1:{pcapoverip_port}");
    let mut command = Command::new("tshark")
        .arg("-i")
        .arg(&interface)
        .arg("-T")
        .arg("ek")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let Ok(child) = command.as_mut() else {
        tracing::error!(
            interface = %interface,
            "decryption: échec de démarrage de tshark sur le flux PCAP-over-IP PolarProxy"
        );
        return;
    };
    let Some(stdout) = child.stdout.take() else {
        tracing::error!("decryption: stdout de tshark (PCAP-over-IP) indisponible");
        return;
    };

    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        if let Some(fragment) = parse_ek_line(&line) {
            mark_seen(&seen, &fragment.five_tuple);
            correlation.send_decryption(fragment);
        }
    }

    if let Ok(mut child) = command {
        let _ = child.wait();
    }
}

fn mark_seen(seen: &SeenFragments, key: &FiveTuple) {
    let mut guard = seen.lock().expect("mutex seen-fragments empoisonné");
    guard.insert(key.clone(), Instant::now());
    guard.retain(|_, seen_at| seen_at.elapsed() < PINNING_WINDOW * 6);
}

/// Suit le fichier `-f` de PolarProxy (append-only) par relecture polling des octets ajoutés —
/// PolarProxy n'étant pas installé sur cette machine de dev, le format exact n'a pas pu être
/// rejoué contre une vraie sortie (cf. doc de module). S'arrête quand le fichier redevient
/// introuvable pendant plus de `FLOWLOG_POLL_INTERVAL` × 4 (signal informel d'arrêt du
/// process — pas de canal d'arrêt dédié dans cette passe, cf. rapport de livraison).
fn run_flowlog_watcher(path: PathBuf, storage: StorageHandle, seen: SeenFragments) {
    let mut offset: u64 = 0;
    let mut missing_polls = 0;

    loop {
        std::thread::sleep(FLOWLOG_POLL_INTERVAL);
        match read_new_lines(&path, &mut offset) {
            Ok(lines) => {
                missing_polls = 0;
                for line in lines {
                    handle_flowlog_line(&line, &storage, &seen);
                }
            }
            Err(_) => {
                missing_polls += 1;
                if missing_polls > 4 {
                    tracing::debug!(
                        "decryption: fichier de flux PolarProxy introuvable, arrêt de la veille"
                    );
                    return;
                }
            }
        }
    }
}

fn read_new_lines(path: &Path, offset: &mut u64) -> std::io::Result<Vec<String>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path)?;
    file.seek(SeekFrom::Start(*offset))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    *offset += buf.len() as u64;
    Ok(buf.lines().map(str::to_string).collect())
}

fn handle_flowlog_line(line: &str, storage: &StorageHandle, seen: &SeenFragments) {
    let Some(entry) = parse_flowlog_line(line) else {
        return;
    };
    let already_decrypted = seen
        .lock()
        .expect("mutex seen-fragments empoisonné")
        .contains_key(&entry.five_tuple);
    if already_decrypted {
        return;
    }

    tracing::warn!(
        host = entry.host.as_deref().unwrap_or("?"),
        "decryption: session externe sans contenu déchiffré correspondant \
         (pinning probable, heuristique best-effort)"
    );
    if let Err(error) = storage_decryption::record_pinning_event(
        storage,
        PinningEvent {
            timestamp_unix: now_unix(),
            protocol: &entry.five_tuple.protocol,
            src_ip: &entry.five_tuple.src_ip,
            src_port: entry.five_tuple.src_port,
            dst_ip: &entry.five_tuple.dst_ip,
            dst_port: entry.five_tuple.dst_port,
            host: entry.host.as_deref(),
        },
    ) {
        tracing::error!(error = %error, "decryption: persistance d'un événement de pinning échouée");
    }
}

struct FlowLogEntry {
    five_tuple: FiveTuple,
    host: Option<String>,
}

/// Parse best-effort d'une ligne `-f` — format supposé délimité par tabulations/espaces d'après
/// l'ordre de champs documenté par Netresec ("timestamp, internal 5-tuple, external 5-tuple,
/// domain_name, ..."), NON vérifié contre une sortie réelle. Retourne `None` sur toute ligne ne
/// correspondant pas au motif minimal attendu — jamais de panic sur une entrée externe non
/// fiable (même discipline que `keylog::parser::parse_ek_line`).
fn parse_flowlog_line(line: &str) -> Option<FlowLogEntry> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 6 {
        return None;
    }
    // Position best-effort : [ts, proto, src_ip:src_port, dst_ip:dst_port, domain, ...]
    let protocol = fields.get(1)?.to_lowercase();
    let (src_ip, src_port) = split_host_port(fields.get(2)?)?;
    let (dst_ip, dst_port) = split_host_port(fields.get(3)?)?;
    let host = fields.get(4).map(|s| s.to_string()).filter(|s| s != "-");

    Some(FlowLogEntry {
        five_tuple: FiveTuple {
            protocol,
            src_ip: crate::attribution::normalize_ip(&src_ip),
            src_port,
            dst_ip: crate::attribution::normalize_ip(&dst_ip),
            dst_port,
        },
        host,
    })
}

fn split_host_port(raw: &str) -> Option<(String, u16)> {
    let (host, port) = raw.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flowlog_line_extrait_le_motif_minimal() {
        let line = "1720000000 tcp 10.0.0.5:51000 93.184.216.34:443 example.com abcd1234";
        let entry = parse_flowlog_line(line).expect("ligne valide attendue");
        assert_eq!(entry.five_tuple.protocol, "tcp");
        assert_eq!(entry.five_tuple.src_port, 51000);
        assert_eq!(entry.five_tuple.dst_port, 443);
        assert_eq!(entry.host.as_deref(), Some("example.com"));
    }

    #[test]
    fn parse_flowlog_line_ignore_une_ligne_trop_courte() {
        assert!(parse_flowlog_line("motif incomplet").is_none());
    }

    #[test]
    fn parse_flowlog_line_ignore_un_host_marque_absent() {
        let line = "1720000000 tcp 10.0.0.5:51000 93.184.216.34:443 - abcd1234";
        let entry = parse_flowlog_line(line).expect("ligne valide attendue");
        assert_eq!(entry.host, None);
    }

    #[test]
    fn mark_seen_puis_lookup_reconnait_le_meme_cinq_tuple() {
        let seen: SeenFragments = Arc::new(Mutex::new(HashMap::new()));
        let five_tuple = FiveTuple {
            protocol: "tcp".to_string(),
            src_ip: "10.0.0.5".to_string(),
            src_port: 51000,
            dst_ip: "93.184.216.34".to_string(),
            dst_port: 443,
        };
        mark_seen(&seen, &five_tuple);
        assert!(seen.lock().unwrap().contains_key(&five_tuple));
    }
}
