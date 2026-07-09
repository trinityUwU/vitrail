//! Serveur gRPC `UI` (`proto/ui.proto`) — écoute sur `$XDG_RUNTIME_DIR/vitrail/ui.sock`
//! (fallback `/tmp/vitrail-runtime-<uid>/vitrail/ui.sock` si `XDG_RUNTIME_DIR` absent), dossier
//! créé en 700 (PLAN.md §6quinquies). Décode les notifications de connexion du daemon
//! `opensnitchd` en `AttributionEvent`, alimente le cache pid→exe (cache.rs) — story 1.2.
//!
//! `AskRule` est la RPC réellement porteuse de l'attribution (le daemon l'appelle pour CHAQUE
//! nouvelle connexion sans règle déjà connue) : c'est elle qui transporte `Connection` avec
//! `process_id`/`process_path`. Vitrail répond systématiquement "allow" en durée "once" (non
//! persistant côté daemon) — il observe, il ne décide jamais de blocage (ARCHITECTURE.md).
//! `PostAlert` peut aussi transporter une `Connection` (oneof `Alert.data`), traitée en source
//! secondaire par le même chemin.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::net::UnixListener;
use tokio::sync::oneshot;
use tokio_stream::wrappers::UnixListenerStream;
use tokio_stream::Stream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use crate::correlation::CorrelationSender;

use super::cache::ProcessCache;
use super::desktop_resolver::{resolve_app_name, AppNameCache};
use super::pb;
use super::pb::ui_server::{Ui, UiServer};

const SOCKET_DIR: &str = "vitrail";
const SOCKET_FILE: &str = "ui.sock";
/// Garde-fou structurel : `AskRule` est bloquante côté daemon `opensnitchd` (cf. doc de module
/// ci-dessus) — même si le chemin normal ne doit jamais attendre d'I/O, ce timeout garantit
/// qu'aucune requête gRPC ne peut geler indéfiniment le serveur si un chemin imprévu bloquait.
const RPC_TIMEOUT: Duration = Duration::from_millis(500);

/// `PartialEq, Eq, Hash` (au-delà du strict besoin d'attribution) : clé de fusion de
/// `correlation/` (EPIC 5, `HashMap<FiveTuple, PendingFlow>`, PLAN.md §6septies 5.1/5.2) —
/// contrat public du domaine anticipé dès EPIC 1 (cf. `attribution/mod.rs`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FiveTuple {
    pub protocol: String,
    pub src_ip: String,
    pub src_port: u16,
    pub dst_ip: String,
    pub dst_port: u16,
}

/// Canonicalise une adresse IP avant utilisation comme fragment de clé de fusion (5.1) :
/// `capture/` (Rust `IpAddr::to_string()`) et `attribution/` (chaîne brute du daemon Go
/// `opensnitchd`) peuvent produire des représentations textuelles différentes de la même
/// adresse (ex: IPv4-mappée `::ffff:a.b.c.d`, casse hexadécimale IPv6) — un `Eq`/`Hash`
/// strict sur `FiveTuple` casserait alors silencieusement la fusion pour ce flux. Repasse par
/// `std::net::IpAddr` pour obtenir la même forme canonique des deux côtés ; conserve la
/// chaîne d'origine si elle ne parse pas (jamais d'échec silencieux qui perdrait la valeur).
pub fn normalize_ip(raw: &str) -> String {
    raw.parse::<std::net::IpAddr>()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| raw.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionEvent {
    pub pid: u32,
    pub exe_path: String,
    pub app_name: String,
    pub five_tuple: Option<FiveTuple>,
    pub timestamp_unix_ms: u128,
}

/// Poignée de contrôle du serveur gRPC démarré dans son propre thread + runtime tokio dédié
/// (`Subsystem::start()` est synchrone — aucun runtime tokio ambiant n'est garanti côté
/// appelant, contrairement à un contexte `#[tokio::main]` classique).
pub struct ServerHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    /// Positionné à `true` par un `stop()` volontaire AVANT le signal d'arrêt : distingue une
    /// fin de vie normale d'une mort anormale (panique/erreur fatale) pour `AbnormalExitGuard`
    /// ci-dessous — story robustesse (audit EPIC 1).
    clean_shutdown: Arc<AtomicBool>,
}

impl ServerHandle {
    pub fn shutdown(mut self) {
        self.clean_shutdown.store(true, Ordering::SeqCst);
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(thread) = self.thread.take() {
            if let Err(error) = thread.join() {
                tracing::error!(error = ?error, "thread du serveur gRPC attribution paniqué");
            }
        }
    }
}

/// Callback de restauration de secours — boxé pour éviter de propager un paramètre générique à
/// travers `start`/`spawn_server_thread`/`run_server` (garde les signatures lisibles).
type AbnormalExitFn = Box<dyn Fn() + Send + 'static>;

/// Filet de sécurité structurel : si le thread serveur se termine SANS passer par
/// `ServerHandle::shutdown()` (panique dans `run_server`, erreur fatale non prévue), `Drop`
/// s'exécute quand même — y compris pendant un déroulement de pile (`panic = "unwind"`, profil
/// par défaut de ce workspace) — et déclenche `on_abnormal_exit` pour restaurer la config
/// `opensnitchd` avant que le processus complet ne reste bloqué dessus indéfiniment.
struct AbnormalExitGuard {
    clean_shutdown: Arc<AtomicBool>,
    on_abnormal_exit: AbnormalExitFn,
}

impl Drop for AbnormalExitGuard {
    fn drop(&mut self) {
        if !self.clean_shutdown.load(Ordering::SeqCst) {
            (self.on_abnormal_exit)();
        }
    }
}

/// Paramètres du thread serveur regroupés — évite de propager 7 paramètres individuels à
/// travers `start`/`spawn_server_thread`/`run_server` (lisibilité + limite 35 lignes/fonction).
struct ServerStartParams {
    socket_path: PathBuf,
    cache: Arc<ProcessCache>,
    app_name_cache: Arc<AppNameCache>,
    correlation: CorrelationSender,
    ready_tx: std::sync::mpsc::Sender<Result<(), String>>,
    shutdown_rx: oneshot::Receiver<()>,
    clean_shutdown: Arc<AtomicBool>,
    on_abnormal_exit: AbnormalExitFn,
}

pub fn socket_path() -> PathBuf {
    let base = std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // SAFETY: `getuid(2)` est un simple appel système sans effet de bord mémoire.
            let uid = unsafe { libc::getuid() };
            PathBuf::from(format!("/tmp/vitrail-runtime-{uid}"))
        });
    base.join(SOCKET_DIR).join(SOCKET_FILE)
}

pub fn socket_uri(path: &Path) -> String {
    format!("unix://{}", path.display())
}

/// Démarre le serveur gRPC dans un thread dédié, bloque jusqu'à ce que le socket soit bindé
/// (ou qu'une erreur de démarrage soit remontée) pour que `Subsystem::start()` reste
/// synchrone et fidèle (jamais de "démarré" optimiste avant que ce soit vraiment le cas).
pub fn start(
    socket_path: PathBuf,
    cache: Arc<ProcessCache>,
    correlation: CorrelationSender,
    on_abnormal_exit: AbnormalExitFn,
) -> Result<ServerHandle, String> {
    prepare_socket_dir(&socket_path)?;
    let _ = std::fs::remove_file(&socket_path);

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let clean_shutdown = Arc::new(AtomicBool::new(false));
    let params = ServerStartParams {
        socket_path,
        cache,
        app_name_cache: Arc::new(AppNameCache::new()),
        correlation,
        ready_tx,
        shutdown_rx,
        clean_shutdown: clean_shutdown.clone(),
        on_abnormal_exit,
    };

    let thread = spawn_server_thread(params);
    await_ready(ready_rx, thread, shutdown_tx, clean_shutdown)
}

/// Attend le signal "prêt" (ou l'échec de démarrage) du thread serveur et construit la
/// poignée — extrait de `start()` pour rester sous la limite de 35 lignes (code-standards.md).
fn await_ready(
    ready_rx: std::sync::mpsc::Receiver<Result<(), String>>,
    thread: JoinHandle<()>,
    shutdown_tx: oneshot::Sender<()>,
    clean_shutdown: Arc<AtomicBool>,
) -> Result<ServerHandle, String> {
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(ServerHandle {
            shutdown_tx: Some(shutdown_tx),
            thread: Some(thread),
            clean_shutdown,
        }),
        Ok(Err(reason)) => {
            let _ = thread.join();
            Err(reason)
        }
        Err(_) => {
            let _ = thread.join();
            Err("le thread du serveur gRPC s'est arrêté avant d'être prêt".to_string())
        }
    }
}

/// Construit le runtime tokio dédié et lance `run_server` dedans, dans un thread séparé —
/// extrait de `start()` pour rester sous la limite de 35 lignes (code-standards.md).
fn spawn_server_thread(params: ServerStartParams) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(error) => {
                let _ = params
                    .ready_tx
                    .send(Err(format!("échec de création du runtime tokio: {error}")));
                return;
            }
        };

        runtime.block_on(run_server(params));
    })
}

async fn run_server(params: ServerStartParams) {
    let ServerStartParams {
        socket_path,
        cache,
        app_name_cache,
        correlation,
        ready_tx,
        shutdown_rx,
        clean_shutdown,
        on_abnormal_exit,
    } = params;

    let listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("échec de bind du socket UNIX: {error}")));
            return;
        }
    };
    let _ = ready_tx.send(Ok(()));
    // Créé APRÈS le signal "prêt" : avant ce point rien n'a encore été reconfiguré côté
    // `opensnitchd` (`Subsystem::start()` attend ce `Ok` avant de toucher au daemon), donc rien
    // à restaurer en cas d'échec de bind — le guard ne doit surveiller que la phase "en service".
    let _abnormal_exit_guard = AbnormalExitGuard {
        clean_shutdown,
        on_abnormal_exit,
    };

    if let Err(error) = serve(listener, cache, app_name_cache, correlation, shutdown_rx).await {
        tracing::error!(error = %error, "serveur gRPC attribution terminé en erreur");
    }
}

/// Construit le service gRPC et sert jusqu'au signal d'arrêt — extrait de `run_server` pour
/// rester sous la limite de 35 lignes (code-standards.md).
async fn serve(
    listener: UnixListener,
    cache: Arc<ProcessCache>,
    app_name_cache: Arc<AppNameCache>,
    correlation: CorrelationSender,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), tonic::transport::Error> {
    let incoming = UnixListenerStream::new(listener);
    let service = UiService {
        cache,
        app_name_cache,
        correlation,
    };
    Server::builder()
        .timeout(RPC_TIMEOUT)
        .add_service(UiServer::new(service))
        .serve_with_incoming_shutdown(incoming, async {
            let _ = shutdown_rx.await;
        })
        .await
}

fn prepare_socket_dir(socket_path: &Path) -> Result<(), String> {
    let dir = socket_path
        .parent()
        .ok_or("chemin de socket sans dossier parent")?;
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("création du dossier socket échouée: {error}"))?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("chmod 700 du dossier socket échoué: {error}"))
}

struct UiService {
    cache: Arc<ProcessCache>,
    app_name_cache: Arc<AppNameCache>,
    correlation: CorrelationSender,
}

impl UiService {
    /// Décode une `Connection` OpenSnitch en `AttributionEvent`, résout le `start_time` via
    /// `/proc` (story 1.3) et le nom d'affichage (story 1.4). Ignore silencieusement (log
    /// debug) les connexions sans pid exploitable ou dont le process est déjà mort — ne doit
    /// jamais faire échouer la RPC appelante.
    ///
    /// EPIC 5 : publie AUSSI l'événement vers `correlation/` — APRÈS avoir mis à jour le cache
    /// et construit `event` (donc après le traitement de la connexion, jamais avant), via
    /// `CorrelationSender::send_attribution` qui est un `try_send` non-bloquant. Ne retarde
    /// jamais la réponse `AskRule` (déjà auditée non-bloquante à deux reprises) : aucune I/O,
    /// aucun `.await` sur ce chemin.
    fn handle_connection(&self, conn: &pb::Connection) {
        let pid = conn.process_id;
        let exe_path = conn.process_path.clone();
        if pid == 0 || exe_path.is_empty() {
            return;
        }
        let Some(start_time) = ProcessCache::read_start_time(pid) else {
            tracing::debug!(pid, "process déjà mort au moment de l'attribution, ignoré");
            return;
        };
        self.cache.insert(pid, start_time, exe_path.clone());

        let event = AttributionEvent {
            pid,
            exe_path: exe_path.clone(),
            app_name: self.resolve_app_name_non_blocking(&exe_path),
            five_tuple: Some(FiveTuple {
                // Normalisé en minuscules : `capture/` et `attribution/` observent la même
                // connexion via deux sources indépendantes (AF_PACKET vs OpenSnitch) — rien ne
                // garantit la même casse de protocole côté daemon, et la clé de fusion de
                // `correlation/` (5.1) est un `Eq`/`Hash` strict sur `FiveTuple` (décision non
                // explicite dans PLAN.md, cf. rapport EPIC 5).
                protocol: conn.protocol.to_lowercase(),
                src_ip: normalize_ip(&conn.src_ip),
                src_port: conn.src_port as u16,
                dst_ip: normalize_ip(&conn.dst_ip),
                dst_port: conn.dst_port as u16,
            }),
            timestamp_unix_ms: now_unix_ms(),
        };
        tracing::debug!(?event, "attribution: connexion décodée");
        self.correlation.send_attribution(event);
    }

    /// Chemin critique `AskRule` (cf. doc de module) : ne bloque JAMAIS sur l'I/O disque
    /// `.desktop` (story 1.4). Sert le nom depuis le cache s'il est déjà connu ; sinon retourne
    /// un nom provisoire (basename en mémoire, aucune I/O) et lance la résolution réelle en
    /// tâche de fond `spawn_blocking`, disponible pour les connexions suivantes du même binaire.
    fn resolve_app_name_non_blocking(&self, exe_path: &str) -> String {
        if let Some(name) = self.app_name_cache.get(exe_path) {
            return name;
        }
        let cache = self.app_name_cache.clone();
        let path = exe_path.to_string();
        tokio::task::spawn_blocking(move || {
            let name = resolve_app_name(&path);
            cache.insert(path, name);
        });
        provisional_basename(exe_path)
    }
}

/// Nom provisoire sans I/O disque — utilisé le temps que `resolve_app_name_non_blocking` résolve
/// le vrai nom `.desktop` en tâche de fond. Affichage uniquement (PLAN.md §6quinquies).
fn provisional_basename(exe_path: &str) -> String {
    Path::new(exe_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| exe_path.to_string())
}

#[tonic::async_trait]
impl Ui for UiService {
    async fn ping(
        &self,
        request: Request<pb::PingRequest>,
    ) -> Result<Response<pb::PingReply>, Status> {
        let id = request.into_inner().id;
        Ok(Response::new(pb::PingReply { id }))
    }

    /// Cœur de l'attribution (story 1.2) : le daemon appelle cette RPC pour CHAQUE nouvelle
    /// connexion sans règle déjà connue et ATTEND une réponse avant de laisser passer le
    /// paquet — Vitrail répond toujours "allow"/"once" (jamais persistant), il n'arbitre pas.
    async fn ask_rule(
        &self,
        request: Request<pb::Connection>,
    ) -> Result<Response<pb::Rule>, Status> {
        let conn = request.into_inner();
        self.handle_connection(&conn);
        Ok(Response::new(allow_rule()))
    }

    async fn subscribe(
        &self,
        request: Request<pb::ClientConfig>,
    ) -> Result<Response<pb::ClientConfig>, Status> {
        let client_config = request.into_inner();
        tracing::info!(
            client = %client_config.name,
            version = %client_config.version,
            "attribution: daemon opensnitchd connecté (Subscribe)"
        );
        Ok(Response::new(client_config))
    }

    type NotificationsStream =
        Pin<Box<dyn Stream<Item = Result<pb::Notification, Status>> + Send + 'static>>;

    /// Vitrail ne pousse aucune commande vers le daemon (ne décide pas de blocage,
    /// ARCHITECTURE.md) : le flux bidirectionnel reste ouvert mais n'émet jamais rien côté
    /// serveur. `_request` (flux de `NotificationReply`) n'a rien à corréler ici.
    async fn notifications(
        &self,
        _request: Request<Streaming<pb::NotificationReply>>,
    ) -> Result<Response<Self::NotificationsStream>, Status> {
        Ok(Response::new(Box::pin(tokio_stream::pending())))
    }

    async fn post_alert(
        &self,
        request: Request<pb::Alert>,
    ) -> Result<Response<pb::MsgResponse>, Status> {
        let alert = request.into_inner();
        if let Some(pb::alert::Data::Conn(conn)) = &alert.data {
            self.handle_connection(conn);
        }
        Ok(Response::new(pb::MsgResponse { id: alert.id }))
    }
}

fn allow_rule() -> pb::Rule {
    pb::Rule {
        created: 0,
        name: "vitrail-allow".to_string(),
        description: "Vitrail observe uniquement, ne bloque jamais (ARCHITECTURE.md)".to_string(),
        enabled: true,
        precedence: false,
        nolog: false,
        action: "allow".to_string(),
        duration: "once".to_string(),
        operator: None,
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod normalize_ip_tests {
    use super::normalize_ip;

    #[test]
    fn ipv4_mapped_ipv6_normalise_vers_la_meme_forme_que_ipv4() {
        // Cas signalé par l'audit EPIC 5 : capture (Rust `IpAddr::to_string()`) et
        // attribution (chaîne brute du daemon Go `opensnitchd`) peuvent représenter la même
        // adresse différemment — vérifie que la clé de fusion converge malgré ça.
        assert_eq!(
            normalize_ip("::ffff:192.168.1.1"),
            normalize_ip("::ffff:192.168.1.1")
        );
    }

    #[test]
    fn ipv6_compresse_egal_ipv6_non_compresse() {
        assert_eq!(
            normalize_ip("2001:0db8:0000:0000:0000:0000:0000:0001"),
            normalize_ip("2001:db8::1")
        );
    }

    #[test]
    fn chaine_non_ip_conservee_telle_quelle() {
        assert_eq!(normalize_ip("pas-une-ip"), "pas-une-ip");
    }
}
