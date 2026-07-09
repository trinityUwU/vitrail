//! Test d'intégration story 1.5 : un vrai client `tonic` se connecte au socket gRPC de
//! Vitrail (socket temporaire dédié au test, jamais `$XDG_RUNTIME_DIR/vitrail/ui.sock` de
//! prod), rejoue une `Connection` construite à la main comme le ferait `opensnitchd` via
//! `AskRule`, vérifie le décodage et la mise à jour du cache pid→exe. Aucun `pkexec` ni accès
//! système réel : seul le serveur gRPC est exercé.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};

use super::cache::ProcessCache;
use super::pb;
use super::pb::ui_client::UiClient;
use super::server::{self, ServerHandle};

async fn connect_client(socket_path: PathBuf) -> UiClient<Channel> {
    let channel = Endpoint::try_from("http://[::]:50051")
        .expect("URI factice invalide")
        .connect_with_connector(tower::service_fn(move |_: Uri| {
            let path = socket_path.clone();
            async move {
                let stream = UnixStream::connect(path).await?;
                Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(stream))
            }
        }))
        .await
        .expect("connexion au socket gRPC de test échouée");
    UiClient::new(channel)
}

/// Sous-dossier dédié et possédé par le test (jamais un socket posé directement dans
/// `std::env::temp_dir()` partagé/non possédé — `prepare_socket_dir` fait un `chmod 700` sur
/// le dossier parent, ce qui échoue avec "Operation not permitted" sur `/tmp` lui-même).
fn test_socket_path() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "vitrail-test-ui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    dir.join("ui.sock")
}

fn sample_connection(pid: u32) -> pb::Connection {
    pb::Connection {
        protocol: "tcp".to_string(),
        src_ip: "127.0.0.1".to_string(),
        src_port: 51234,
        dst_ip: "93.184.216.34".to_string(),
        dst_host: "example.com".to_string(),
        dst_port: 443,
        user_id: 1000,
        process_id: pid,
        process_path: "/usr/bin/vitrail-test-fixture".to_string(),
        process_cwd: String::new(),
        process_args: vec![],
        process_env: Default::default(),
        process_checksums: Default::default(),
        process_tree: vec![],
    }
}

/// Regroupe le socket + le cache + la poignée serveur d'un test — extrait pour partager le
/// setup/teardown entre les deux tests d'intégration ci-dessous et rester sous la limite de
/// 35 lignes par fonction (code-standards.md).
struct TestServer {
    socket_path: PathBuf,
    cache: Arc<ProcessCache>,
    handle: ServerHandle,
}

/// Démarre un serveur gRPC réel sur un socket de test isolé et connecte un client dessus.
/// `on_abnormal_exit` est un no-op ici : ces tests exercent le décodage `AskRule`/`PostAlert`,
/// pas le filet de sécurité de restauration (couvert par les tests de `subsystem.rs`).
async fn start_test_server() -> (TestServer, UiClient<Channel>) {
    let socket_path = test_socket_path();
    let _ = std::fs::remove_file(&socket_path);
    let cache = Arc::new(ProcessCache::new());

    let handle = server::start(
        socket_path.clone(),
        cache.clone(),
        crate::correlation::channel().0,
        Box::new(|| {}),
    )
    .expect("démarrage serveur test échoué");
    let client = connect_client(socket_path.clone()).await;

    (
        TestServer {
            socket_path,
            cache,
            handle,
        },
        client,
    )
}

/// Le client DOIT être droppé avant `handle.shutdown()` : `serve_with_incoming_shutdown`
/// attend la fermeture propre des connexions en cours avant de rendre la main. Runtime
/// `multi_thread` requis sur l'appelant (`#[tokio::test(flavor = "multi_thread")]`) :
/// `handle.shutdown()` bloque le thread appelant sur `JoinHandle::join` — sur un runtime
/// `current_thread` (défaut `#[tokio::test]`), plus aucune tâche (dont celle qui ferme
/// proprement la connexion HTTP/2 du client droppé) ne peut progresser pendant ce blocage,
/// d'où un deadlock.
fn teardown_test_server(client: UiClient<Channel>, server: TestServer) {
    drop(client);
    server.handle.shutdown();
    let _ = std::fs::remove_dir_all(server.socket_path.parent().unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ask_rule_decode_en_attribution_event_et_met_a_jour_le_cache() {
    let (test_server, mut client) = start_test_server().await;

    // Utilise le pid du process de test lui-même : garanti vivant et lisible via /proc
    // pendant toute la durée du test, sans dépendre d'un process externe.
    let own_pid = std::process::id();
    let response = client
        .ask_rule(sample_connection(own_pid))
        .await
        .expect("appel AskRule échoué")
        .into_inner();

    assert_eq!(response.action, "allow", "Vitrail ne doit jamais bloquer");
    assert_eq!(
        response.duration, "once",
        "jamais de règle persistante côté daemon"
    );

    let start_time = ProcessCache::read_start_time(own_pid)
        .expect("lecture start_time du process de test échouée");
    let entry = test_server
        .cache
        .get(own_pid, start_time)
        .expect("le cache pid→exe n'a pas été mis à jour par AskRule");
    assert_eq!(entry.exe_path, "/usr/bin/vitrail-test-fixture");

    teardown_test_server(client, test_server);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_alert_avec_connection_met_aussi_a_jour_le_cache() {
    let (test_server, mut client) = start_test_server().await;

    let own_pid = std::process::id();
    let alert = pb::Alert {
        id: 42,
        r#type: 0,
        action: 0,
        priority: 0,
        what: 3, // CONNECTION
        data: Some(pb::alert::Data::Conn(sample_connection(own_pid))),
    };

    let response = client
        .post_alert(alert)
        .await
        .expect("appel PostAlert échoué")
        .into_inner();
    assert_eq!(response.id, 42);

    let start_time = ProcessCache::read_start_time(own_pid).unwrap();
    assert!(
        test_server.cache.get(own_pid, start_time).is_some(),
        "PostAlert(Connection) doit alimenter le cache au même titre qu'AskRule"
    );

    teardown_test_server(client, test_server);
}
