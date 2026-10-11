use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::game_telemetry::start;
use crate::game_telemetry::transport::{recover_now, Consent, Recovery};
use crate::storage::DesktopState;
use cimmeria_launcher_core::telemetry::queue::DiskQueue;
use std::io::{Read, Write};
use std::time::Instant;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";
const BUNDLE: &str = "/api/telemetry/upload-bundle";

fn servers(server: &MockServer) -> Vec<LoginServer> {
    vec![LoginServer {
        name: "fixture".into(),
        url: server.uri(),
    }]
}

/// Every mint answers; a 250 ms cadence keeps the tests quick.
async fn mount(server: &MockServer) {
    let expires = chrono::Utc::now().timestamp_millis() + 8 * 60 * 60 * 1000;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "session-1", "token": "tok-1", "expires_at_ms": expires,
            "upload_endpoint": format!("{}/api/telemetry", server.uri()),
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 250,
        })))
        .mount(server)
        .await;
    for route in [CHUNK, BUNDLE] {
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200))
            .mount(server)
            .await;
    }
}

/// The install's `game` directory, with the client's `Binaries`.
fn game(root: &Path) -> (PathBuf, PathBuf) {
    let game = root.join("game");
    let binaries = game.join("Working/Binaries");
    std::fs::create_dir_all(binaries.join("sessions")).unwrap();
    (game, binaries)
}

fn append(path: &Path, text: &str) {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
}

/// Play's mint, then the session on it: what the launch worker does.
async fn begin(state: &DesktopState, server: &MockServer, game: &Path) -> SessionLoop {
    let consent = Consent::read(state).unwrap().expect("opted in");
    let session = start(
        &state.game_telemetry().unwrap().identity.unwrap(),
        &servers(server),
        &["desktop-launcher"],
    )
    .await
    .unwrap();
    let transport = Transport::open(consent, &session, &servers(server), game).unwrap();
    SessionLoop::begin(transport, game)
}

/// Every event of every chunk the server took, in order.
async fn delivered(server: &MockServer) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    for request in server.received_requests().await.unwrap() {
        if request.url.path() != CHUNK {
            continue;
        }
        let mut text = String::new();
        flate2::read::GzDecoder::new(request.body.as_slice())
            .read_to_string(&mut text)
            .unwrap();
        out.extend(text.lines().map(|line| serde_json::from_str(line).unwrap()));
    }
    out
}

/// The tailed lines delivered, in order.
async fn lines(server: &MockServer) -> Vec<String> {
    delivered(server)
        .await
        .iter()
        .filter_map(|event| event["message"].as_str().map(String::from))
        .collect()
}

async fn wait_for_line(server: &MockServer, line: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !lines(server).await.iter().any(|l| l == line) {
        assert!(Instant::now() < deadline, "{line:?} never arrived");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn posts(server: &MockServer, route: &str) -> usize {
    let requests = server.received_requests().await.unwrap();
    requests.iter().filter(|r| r.url.path() == route).count()
}

fn opted_in(root: &Path) -> DesktopState {
    let mut state = DesktopState::open(root).unwrap();
    state.set_game_telemetry(true).unwrap();
    state
}

const EXIT: GameExit = GameExit {
    host_pid: 4100,
    guest_pid: 4242,
    code: 7,
    early: false,
};

/// A log grown in steps ships each complete line once, in order: never a
/// line from before Play, never a partial line, never a line twice. A
/// session log that appears mid-session ships from its first line, in the
/// egui launcher's `client_log` shape. The exit closes the stream.
#[tokio::test]
async fn a_log_grown_in_steps_ships_each_line_once_and_the_exit_closes_it() {
    let root = tempfile::tempdir().unwrap();
    let (game, binaries) = game(root.path());
    let debug = binaries.join("SGWDebugLog.log");
    append(&debug, "from the last session\n");
    append(&binaries.join("sessions/earlier.log"), "earlier session\n");
    let server = MockServer::start().await;
    mount(&server).await;
    let state = opted_in(&root.path().join("state"));

    let session = begin(&state, &server, &game).await;
    append(&debug, "one\n");
    wait_for_line(&server, "one").await;
    append(&debug, "two\nthree\npart");
    append(
        &binaries.join("sessions/2026-10-11_12-00.log"),
        "[t] [warn] [Mercury] retransmit pkt=42\n",
    );
    wait_for_line(&server, "three").await;
    wait_for_line(&server, "retransmit pkt=42").await;
    // A few more ticks with nothing new: nothing goes again.
    tokio::time::sleep(Duration::from_millis(800)).await;
    append(&debug, "ial\n");
    let end = session.finish(Some(EXIT)).await.unwrap();

    let events = delivered(&server).await;
    // Each file in its own order (files are read in no fixed order).
    let of = |kind: &str| -> Vec<String> {
        events
            .iter()
            .filter(|e| e["type"] == kind)
            .map(|e| e["message"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(of("debug_log"), ["one", "two", "three", "partial"]);
    assert_eq!(of("client_log"), ["retransmit pkt=42"]);
    assert_eq!(events.first().unwrap()["kind"], "started");
    let debug_line = events.iter().find(|e| e["message"] == "one").unwrap();
    assert_eq!(debug_line["type"], "debug_log", "{debug_line}");
    assert_eq!(debug_line["source_file"], "SGWDebugLog.log");
    let client_line = events
        .iter()
        .find(|e| e["message"] == "retransmit pkt=42")
        .unwrap();
    assert_eq!(client_line["type"], "client_log", "{client_line}");
    assert_eq!(client_line["level"], "warn");
    assert_eq!(client_line["category"], "Mercury");
    assert_eq!(client_line["packet_no"], 42);
    assert_eq!(client_line["source_file"], "2026-10-11_12-00.log");
    let exited = events.last().unwrap();
    assert_eq!(exited["kind"], "game_exited", "{exited}");
    assert_eq!(
        exited["fields"],
        serde_json::json!({"exit_code": 7, "pid": 4242, "host_pid": 4100, "early": false})
    );
    assert_eq!(
        end,
        SessionEnd {
            events: 7,
            exit: Some(EXIT),
            bundle_sent: true,
            withdrawn: false,
        }
    );
    assert_eq!(posts(&server, BUNDLE).await, 1);
    let seqs: Vec<u64> = events.iter().map(|e| e["seq"].as_u64().unwrap()).collect();
    assert!(seqs.windows(2).all(|w| w[0] < w[1]), "{seqs:?}");
}

/// No exit observed (the supervisor's outcome is unknown): the stream
/// closes and the bundle goes, with no `game_exited` event to claim a code.
#[tokio::test]
async fn an_unobserved_exit_records_no_code() {
    let root = tempfile::tempdir().unwrap();
    let (game, binaries) = game(root.path());
    let server = MockServer::start().await;
    mount(&server).await;
    let state = opted_in(&root.path().join("state"));
    let session = begin(&state, &server, &game).await;
    append(&binaries.join("SGWDebugLog.log"), "only line\n");
    let end = session.finish(None).await.unwrap();
    assert_eq!((end.exit, end.bundle_sent), (None, true));
    let kinds: Vec<_> = delivered(&server)
        .await
        .iter()
        .filter_map(|e| e["kind"].as_str().map(String::from))
        .collect();
    assert_eq!(kinds, ["started"]);
    assert_eq!(lines(&server).await, ["only line"]);
}

/// The launcher closes while the game runs: the loop dies with the runtime
/// and is not resumed. After the restart, the next Play's session sends only
/// what is written after it began: nothing the first session sent goes
/// again.
#[tokio::test]
async fn a_launcher_restart_mid_session_ships_no_line_twice() {
    let root = tempfile::tempdir().unwrap();
    let state_dir = root.path().join("state");
    let (game, binaries) = game(root.path());
    let debug = binaries.join("SGWDebugLog.log");
    let server = MockServer::start().await;
    mount(&server).await;

    // The first launcher: its own runtime, as the shell's would be.
    let launcher = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let state = opted_in(&state_dir);
    let consent = Consent::read(&state).unwrap().unwrap();
    let minted = start(
        &state.game_telemetry().unwrap().identity.unwrap(),
        &servers(&server),
        &["desktop-launcher"],
    )
    .await
    .unwrap();
    let transport = Transport::open(consent, &minted, &servers(&server), &game).unwrap();
    let session = {
        let _launcher = launcher.enter();
        SessionLoop::begin(transport, &game)
    };
    append(&debug, "before the close\n");
    wait_for_line(&server, "before the close").await;
    // The launcher closes: the runtime goes, and the loop with it.
    tokio::task::spawn_blocking(move || launcher.shutdown_timeout(Duration::from_secs(5)))
        .await
        .unwrap();
    drop(session);
    drop(state);
    // Written while no launcher was running.
    append(&debug, "while closed\n");

    let state = DesktopState::open(&state_dir).unwrap();
    assert!(matches!(
        recover_now(&state),
        Recovery::Empty | Recovery::Pending(_)
    ));
    let session = begin(&state, &server, &game).await;
    append(&debug, "after the restart\n");
    wait_for_line(&server, "after the restart").await;
    session.finish(Some(EXIT)).await.unwrap();
    assert_eq!(
        lines(&server).await,
        ["before the close", "after the restart"]
    );
}

/// The bundle's files are the tail's: session logs below a subdirectory of
/// `sessions`, and a `Sessions` directory spelt with a capital.
#[tokio::test]
async fn session_logs_at_any_depth_and_in_any_case_are_tailed() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("game");
    let binaries = game.join("Working/Binaries");
    std::fs::create_dir_all(binaries.join("Sessions/sub")).unwrap();
    let server = MockServer::start().await;
    mount(&server).await;
    let state = opted_in(&root.path().join("state"));
    let session = begin(&state, &server, &game).await;
    append(
        &binaries.join("Sessions/sub/x.log"),
        "[t] [info] [UI] nested\n",
    );
    append(
        &binaries.join("Sessions/y.TXT"),
        "[t] [info] [UI] capitalised\n",
    );
    wait_for_line(&server, "nested").await;
    wait_for_line(&server, "capitalised").await;
    session.finish(None).await.unwrap();
}

/// Mints answer `tok-1`, then `tok-2`. A chunk sent on `slow_token` is
/// answered after `slow`; everything else at once.
async fn mount_two_sessions(server: &MockServer, slow_token: &str, slow: Duration) {
    let expires = chrono::Utc::now().timestamp_millis() + 8 * 60 * 60 * 1000;
    for (n, token) in [(1, "tok-1"), (2, "tok-2")] {
        let mint = Mock::given(method("POST"))
            .and(path("/api/auth/dev-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session_id": format!("session-{n}"), "token": token, "expires_at_ms": expires,
                "upload_endpoint": format!("{}/api/telemetry", server.uri()),
                "chunk_max_bytes": 1_048_576, "flush_interval_ms": 250,
            })));
        let mint = if n == 1 {
            mint.up_to_n_times(1).with_priority(1)
        } else {
            mint
        };
        mint.mount(server).await;
    }
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .and(wiremock::matchers::header(
            "authorization",
            format!("Bearer {slow_token}").as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_delay(slow))
        .with_priority(1)
        .mount(server)
        .await;
    for route in [CHUNK, BUNDLE] {
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200))
            .mount(server)
            .await;
    }
}

/// Play's mint and the session on it, through [`SessionLoop::start`] as
/// the launch worker does: it waits for the previous session.
async fn start_after_previous(
    state: &DesktopState,
    server: &MockServer,
    game: &Path,
) -> SessionLoop {
    let consent = Consent::read(state).unwrap().expect("opted in");
    let session = start(
        &state.game_telemetry().unwrap().identity.unwrap(),
        &servers(server),
        &["desktop-launcher"],
    )
    .await
    .unwrap();
    let transport = Transport::open(consent, &session, &servers(server), game).unwrap();
    SessionLoop::start(transport, game).await
}

/// The messages and session kinds of every chunk sent on `token`.
async fn sent_on(server: &MockServer, token: &str) -> Vec<String> {
    let bearer = format!("Bearer {token}");
    let mut out = Vec::new();
    for request in server.received_requests().await.unwrap() {
        let on_token = request
            .headers
            .get("authorization")
            .is_some_and(|value| value.to_str().unwrap() == bearer);
        if request.url.path() != CHUNK || !on_token {
            continue;
        }
        let mut text = String::new();
        flate2::read::GzDecoder::new(request.body.as_slice())
            .read_to_string(&mut text)
            .unwrap();
        for line in text.lines() {
            let event: serde_json::Value = serde_json::from_str(line).unwrap();
            let label = event["message"].as_str().or(event["kind"].as_str());
            out.push(label.unwrap_or_default().to_string());
        }
    }
    out
}

/// A second Play while the first session is still sending its last chunk:
/// the new session queues nothing until the old one is done, so none of its
/// events goes out under the old session's token.
#[tokio::test]
async fn a_new_play_waits_for_the_previous_session_to_finish_sending() {
    let root = tempfile::tempdir().unwrap();
    let (game, binaries) = game(root.path());
    let debug = binaries.join("SGWDebugLog.log");
    let server = MockServer::start().await;
    mount_two_sessions(&server, "tok-1", Duration::from_millis(600)).await;
    let state = opted_in(&root.path().join("state"));

    let first = start_after_previous(&state, &server, &game).await;
    append(&debug, "first play\n");
    wait_for_line(&server, "first play").await;
    let first = first.finish(Some(EXIT));
    // The first session is now in its last, slow, chunk.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let second = start_after_previous(&state, &server, &game).await;
    assert!(first.is_finished(), "the second session began first");
    append(&debug, "second play\n");
    wait_for_line(&server, "second play").await;
    second.finish(None).await.unwrap();

    let old = sent_on(&server, "tok-1").await;
    assert_eq!(
        old.iter().filter(|label| *label == "started").count(),
        1,
        "{old:?}"
    );
    assert!(!old.iter().any(|label| label == "second play"), "{old:?}");
    assert_eq!(sent_on(&server, "tok-2").await, ["started", "second play"]);
}

/// A previous session still sending past the bound is stopped: the new one
/// begins, and the old one sends nothing after that.
#[tokio::test]
async fn a_previous_session_stuck_past_the_bound_is_stopped() {
    let root = tempfile::tempdir().unwrap();
    let (game, binaries) = game(root.path());
    let server = MockServer::start().await;
    // Every chunk on the first token hangs.
    mount_two_sessions(&server, "tok-1", Duration::from_secs(30)).await;
    let state = opted_in(&root.path().join("state"));

    let first = begin(&state, &server, &game).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let first = first.finish(Some(EXIT));
    let began = Instant::now();
    settle_previous(state.state_root(), Duration::from_millis(300)).await;
    assert!(began.elapsed() < Duration::from_secs(5));
    assert!(first.await.unwrap_err().is_cancelled());
    let second = begin(&state, &server, &game).await;
    append(&binaries.join("SGWDebugLog.log"), "second play\n");
    wait_for_line(&server, "second play").await;
    second.finish(None).await.unwrap();
    assert!(
        !sent_on(&server, "tok-1")
            .await
            .iter()
            .any(|l| l == "second play"),
        "the stopped session sent the new one's line"
    );
}

/// Opting out while the game runs stops the session: nothing written after
/// the change is sent, no bundle goes, and no queue is left on disk, though
/// the transport was opened on the earlier consent.
#[tokio::test]
async fn an_opt_out_mid_session_stops_sending_and_leaves_no_queue() {
    let root = tempfile::tempdir().unwrap();
    let state_dir = root.path().join("state");
    let (game, binaries) = game(root.path());
    let debug = binaries.join("SGWDebugLog.log");
    let server = MockServer::start().await;
    mount(&server).await;
    let mut state = opted_in(&state_dir);
    let session = begin(&state, &server, &game).await;
    append(&debug, "while opted in\n");
    wait_for_line(&server, "while opted in").await;

    state.set_game_telemetry(false).unwrap();
    append(&debug, "after the opt-out\n");
    // Several ticks pass before the game exits.
    tokio::time::sleep(Duration::from_millis(1_000)).await;
    let end = session.finish(Some(EXIT)).await.unwrap();

    assert!(end.withdrawn, "{end:?}");
    assert_eq!(lines(&server).await, ["while opted in"]);
    assert_eq!(posts(&server, BUNDLE).await, 0);
    let kinds: Vec<_> = delivered(&server)
        .await
        .iter()
        .filter_map(|e| e["kind"].as_str().map(String::from))
        .collect();
    assert_eq!(kinds, ["started"], "no exit event after the opt-out");
    let queue = DiskQueue::new(&state_dir);
    assert_eq!(
        queue
            .pending_count::<cimmeria_launcher_core::telemetry::events::TelemetryEvent>()
            .unwrap(),
        0
    );
    for name in ["telemetry-queue.jsonl", "telemetry-queue.inflight.jsonl"] {
        assert!(!state_dir.join(name).exists(), "{name} left behind");
    }
}
