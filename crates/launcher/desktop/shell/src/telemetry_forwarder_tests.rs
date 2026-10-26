use super::{forward_loop, request_flush, FlushSignal, EXIT_FLUSH_WAIT};
use cimmeria_launcher_engine::game_telemetry::launcher_event::{
    launcher_events, LauncherRecord, Redaction,
};
use cimmeria_launcher_engine::game_telemetry::launcher_forwarder::{
    ForwardSource, Forwarder, Pumped,
};
use cimmeria_launcher_engine::game_telemetry::launcher_session::{LauncherSender, LauncherTarget};
use cimmeria_launcher_engine::game_telemetry::transport::TelemetryEvent;
use cimmeria_launcher_engine::DesktopState;
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";

/// A source that always has a target and builds events the way the host does.
struct Fake {
    target: Option<LauncherTarget>,
}

impl ForwardSource for Fake {
    fn events(&self, records: &[LauncherRecord]) -> Option<Vec<TelemetryEvent>> {
        Some(launcher_events(records, &Redaction::default()))
    }
    fn target(&self) -> Option<LauncherTarget> {
        self.target.clone()
    }
    fn epoch(&self) -> u64 {
        0
    }
}

fn record(target: &'static str, event: Option<&str>) -> LauncherRecord {
    let mut fields = serde_json::Map::new();
    if let Some(event) = event {
        fields.insert("event".into(), event.into());
    }
    LauncherRecord {
        ts_ms: 1,
        level: "info".into(),
        target,
        fields,
    }
}

/// A mock server that mints a launcher session and accepts every chunk, and a target on it.
fn server_and_target() -> (MockServer, LauncherTarget) {
    let server = tauri::async_runtime::block_on(MockServer::start());
    let expires = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        + 3_600_000;
    tauri::async_runtime::block_on(async {
        Mock::given(method("POST"))
            .and(path("/api/auth/dev-session"))
            .and(body_partial_json(
                serde_json::json!({"session_kind": "launcher"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": "session-1", "token": "payload.sig", "expires_at_ms": expires,
                "upload_endpoint": format!("{}/api/telemetry", server.uri()),
                "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(CHUNK))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;
    });
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    state.set_game_telemetry(true).unwrap();
    let mut target = state.launcher_target().expect("opted in");
    target.servers.truncate(1);
    target.servers[0].url = server.uri();
    (server, target)
}

fn chunk_posts(server: &MockServer) -> usize {
    tauri::async_runtime::block_on(server.received_requests())
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == CHUNK)
        .count()
}

type Seen = Arc<Mutex<Vec<Pumped>>>;

/// Runs `forward_loop` on a thread; returns the thread and what it pumped.
fn start(
    target: LauncherTarget,
    records: Receiver<LauncherRecord>,
) -> (std::thread::JoinHandle<()>, Seen) {
    start_with(Some(target), records, Arc::new(FlushSignal::new()))
}

/// As `start`, with the target the source reports (None = opted out) and the signal
/// `on_flushed` notifies.
fn start_with(
    target: Option<LauncherTarget>,
    records: Receiver<LauncherRecord>,
    flushed: Arc<FlushSignal>,
) -> (std::thread::JoinHandle<()>, Seen) {
    let seen: Seen = Arc::default();
    let sink = Arc::clone(&seen);
    let handle = std::thread::spawn(move || {
        let mut forwarder = Forwarder::new(LauncherSender::new());
        let source = Fake { target };
        forward_loop(
            &records,
            &mut forwarder,
            &source,
            &mut |p| sink.lock().unwrap().push(*p),
            &mut || flushed.notify(),
        );
    });
    (handle, seen)
}

#[test]
fn a_flush_request_sends_the_queue_and_returns_within_the_bound() {
    let (server, target) = server_and_target();
    let (tx, rx) = sync_channel(16);
    let signal = Arc::new(FlushSignal::new());
    let (handle, _seen) = start_with(Some(target), rx, Arc::clone(&signal));
    tx.send(record("client.launcher.adoption", None)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let began = Instant::now();
    assert!(request_flush(&tx, &signal, Duration::from_secs(3)));
    assert!(
        began.elapsed() < Duration::from_millis(1_500),
        "not the 3 s timeout"
    );
    assert_eq!(
        chunk_posts(&server),
        1,
        "the queued row was posted before it returned"
    );
    drop(tx);
    handle.join().unwrap();
}

#[test]
fn a_flush_request_under_a_hold_returns_at_once_without_a_post() {
    let (server, target) = server_and_target();
    tauri::async_runtime::block_on(
        Mock::given(method("POST"))
            .and(path(CHUNK))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "600"))
            .with_priority(1)
            .mount(&server),
    );
    let (tx, rx) = sync_channel(16);
    let signal = Arc::new(FlushSignal::new());
    let (handle, _seen) = start_with(Some(target), rx, Arc::clone(&signal));
    tx.send(record("client.launcher.launch", Some("launch_finished")))
        .unwrap();
    let start_at = Instant::now();
    while chunk_posts(&server) == 0 && start_at.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(chunk_posts(&server), 1, "the 429 that starts the hold");
    std::thread::sleep(Duration::from_millis(100));
    let began = Instant::now();
    assert!(request_flush(&tx, &signal, Duration::from_secs(3)));
    assert!(
        began.elapsed() < Duration::from_millis(500),
        "a hold ends the flush"
    );
    assert_eq!(chunk_posts(&server), 1, "no post during the hold");
    drop(tx);
    handle.join().unwrap();
}

#[test]
fn a_flush_request_does_not_send_when_the_player_is_opted_out() {
    let (server, _target) = server_and_target();
    let (tx, rx) = sync_channel(16);
    let signal = Arc::new(FlushSignal::new());
    // No target: consent is withdrawn. The flush must end, and must post nothing.
    let (handle, _seen) = start_with(None, rx, Arc::clone(&signal));
    tx.send(record("client.launcher.adoption", None)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(request_flush(&tx, &signal, Duration::from_secs(3)));
    assert_eq!(
        chunk_posts(&server),
        0,
        "the exit flush is not a consent bypass"
    );
    drop(tx);
    handle.join().unwrap();
}

#[test]
fn a_flush_request_on_a_closed_channel_fails_at_once() {
    let (tx, rx) = sync_channel(16);
    drop(rx);
    let began = Instant::now();
    assert!(!request_flush(
        &tx,
        &FlushSignal::new(),
        Duration::from_secs(3)
    ));
    assert!(began.elapsed() < Duration::from_millis(500));
}

#[test]
fn a_flush_request_times_out_when_nothing_answers() {
    // The receiver is alive but nobody pumps it, so the signal never moves.
    let (tx, _rx) = sync_channel(16);
    let began = Instant::now();
    assert!(!request_flush(
        &tx,
        &FlushSignal::new(),
        Duration::from_millis(100)
    ));
    assert!(
        began.elapsed() < Duration::from_millis(1_000),
        "bounded by `wait`"
    );
}

#[test]
fn a_burst_ending_in_launch_finished_is_one_post_of_three_rows() {
    let (server, target) = server_and_target();
    let (tx, rx) = sync_channel(16);
    let (handle, seen) = start(target, rx);
    // Spaced out so each lands in its own receive round: a loop that sends per round
    // would post three times.
    tx.send(record("client.launcher.adoption", None)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    tx.send(record("client.launcher.adoption", None)).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    tx.send(record("client.launcher.launch", Some("launch_finished")))
        .unwrap();
    drop(tx);
    handle.join().unwrap();
    assert_eq!(chunk_posts(&server), 1, "one burst is one post");
    let sent: u64 = seen.lock().unwrap().iter().map(|p| p.sent_events).sum();
    assert_eq!(sent, 3, "the post holds all three rows");
}

#[test]
fn without_a_flush_trigger_the_send_waits_for_the_coalesce_window() {
    let (server, target) = server_and_target();
    let (tx, rx) = sync_channel(16);
    let (handle, seen) = start(target, rx);
    let start_at = Instant::now();
    tx.send(record("client.launcher.adoption", None)).unwrap();
    std::thread::sleep(Duration::from_millis(1_900));
    assert_eq!(chunk_posts(&server), 0, "nothing is sent before 2 s");
    // The window closes at 2 s; allow the rest of the 2.5 s.
    while chunk_posts(&server) == 0 && start_at.elapsed() < Duration::from_millis(2_500) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(chunk_posts(&server), 1, "one post once the window closes");
    drop(tx);
    handle.join().unwrap();
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn launch_finished_sends_without_waiting_for_the_window() {
    let (server, target) = server_and_target();
    let (tx, rx) = sync_channel(16);
    let (handle, seen) = start(target, rx);
    tx.send(record("client.launcher.adoption", None)).unwrap();
    tx.send(record("client.launcher.launch", Some("launch_finished")))
        .unwrap();
    // The sender stays alive, so no flush sentinel can send the rows: only the trigger can.
    let start_at = Instant::now();
    while chunk_posts(&server) == 0 && start_at.elapsed() < Duration::from_millis(1_000) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(chunk_posts(&server), 1, "launch_finished posts at once");
    drop(tx);
    handle.join().unwrap();
    let sent: u64 = seen.lock().unwrap().iter().map(|p| p.sent_events).sum();
    assert_eq!(sent, 2, "the post holds both rows");
}

#[test]
fn the_exit_flush_is_bounded_at_three_seconds() {
    assert_eq!(EXIT_FLUSH_WAIT, Duration::from_secs(3));
}
