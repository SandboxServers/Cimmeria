//! LF-15: the Upload flow end to end against an HTTP double, with a fake clock.
use super::ingest::{
    rows, send_with, ChunkSink, IngestOutcome, Pacer, GAME_LOG_TARGET, LAUNCHER_LOG_TARGET,
};
use super::lines::LogLine;
use super::offsets::Offsets;
use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::game_telemetry::launcher_session::BatchOutcome;
use crate::test_capture::capture;
use cimmeria_launcher_core::telemetry::events::TelemetryEvent;
use std::future::Future;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";
const LOG_FILE: &str = "launcher.2026-10-22.log";

#[derive(Default)]
struct FakePacer {
    now: u64,
    sleeps: Vec<u64>,
}
impl Pacer for FakePacer {
    fn now_ms(&self) -> u64 {
        self.now
    }
    fn sleep_ms(&mut self, ms: u64) -> impl Future<Output = ()> + Send {
        self.sleeps.push(ms);
        self.now += ms;
        std::future::ready(())
    }
}

/// The state under `<root>/state`, whose logs folder is `<root>/logs`, with its
/// saved login server pointing at `url`. Diagnostics are never opted in.
fn state_at(root: &Path, url: &str) -> DesktopState {
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    state
        .save_client_options(
            vec![LoginServer {
                name: "fixture".into(),
                url: url.into(),
            }],
            false,
            0,
        )
        .unwrap();
    state
}

/// `count` appender lines of one fixed width. Returns the bytes per line.
fn write_log(root: &Path, count: usize) -> usize {
    let dir = root.join("logs");
    std::fs::create_dir_all(&dir).unwrap();
    let text: String = (0..count)
        .map(|i| format!("2026-10-22T12:34:56.123456Z  INFO cimmeria_launcher::net: line {i:05}\n"))
        .collect();
    std::fs::write(dir.join(LOG_FILE), &text).unwrap();
    text.len() / count.max(1)
}

fn upload() -> LogUploadCommand {
    LogUploadCommand::Upload { schema_version: 1 }
}

fn inspect() -> LogUploadCommand {
    LogUploadCommand::Inspect { schema_version: 1 }
}

fn ingest_of(step: LogUploadStep) -> LogIngest {
    match step {
        LogUploadStep::Ingest(ingest) => ingest,
        other => panic!("expected an ingest, got {other:?}"),
    }
}

async fn mount_mint(server: &MockServer) {
    let expires_at_ms = chrono::Utc::now().timestamp_millis() + 3_600_000;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .and(body_partial_json(
            serde_json::json!({"session_kind": "launcher", "purpose": "logs"}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "session-1", "token": "payload.sig", "expires_at_ms": expires_at_ms,
            "upload_endpoint": format!("{}/api/telemetry", server.uri()),
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
        })))
        .mount(server)
        .await;
}

async fn accepting_server() -> MockServer {
    let server = MockServer::start().await;
    mount_mint(&server).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    server
}

async fn chunk_posts(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == CHUNK)
        .count()
}

async fn run_upload(
    state: &DesktopState,
    uploader: &LogUploader,
) -> Result<LogUploadState, LogUploadError> {
    let ingest = ingest_of(state.begin_log_upload(upload(), uploader)?);
    ingest
        .run_with(&mut FakePacer::default())
        .await
        .map(|status| status.state)
}

#[tokio::test]
async fn an_upload_during_reconciliation_required_sends() {
    let root = tempfile::tempdir().unwrap();
    let server = accepting_server().await;
    write_log(root.path(), 3);
    let mut state = state_at(root.path(), &server.uri());
    // An interrupted operation reopens as ReconciliationRequired.
    state
        .operations_mut()
        .unwrap()
        .begin(Uuid::new_v4(), crate::OperationKind::Install, [1; 32], 0)
        .unwrap();
    drop(state);
    let state = DesktopState::open(&root.path().join("state")).unwrap();
    assert_eq!(
        state
            .operations()
            .snapshot()
            .operation
            .as_ref()
            .unwrap()
            .state,
        crate::OperationState::ReconciliationRequired
    );
    assert_eq!(
        state.ensure_no_active_operation(),
        Err(StorageError::Busy),
        "the mutation gate is closed; the upload does not use it"
    );
    let sent = run_upload(&state, &LogUploader::new()).await.unwrap();
    let LogUploadState::Sent {
        lines_sent,
        lines_total,
        upload_id,
    } = sent
    else {
        panic!("expected sent, got {sent:?}");
    };
    assert_eq!((lines_sent, lines_total, upload_id.len()), (3, 3, 16));
    assert_eq!(chunk_posts(&server).await, 1);
}

#[tokio::test]
async fn with_diagnostics_off_an_upload_still_sends() {
    let root = tempfile::tempdir().unwrap();
    let server = accepting_server().await;
    write_log(root.path(), 2);
    let state = state_at(root.path(), &server.uri());
    assert!(!state.game_telemetry().unwrap().opted_in);
    assert!(
        state.launcher_target().is_none(),
        "the event target needs the opt-in"
    );
    assert_eq!(
        status_of(state.begin_log_upload(inspect(), &LogUploader::new())),
        LogUploadState::Ready
    );
    assert!(matches!(
        run_upload(&state, &LogUploader::new()).await,
        Ok(LogUploadState::Sent { lines_sent: 2, .. })
    ));
    assert!(
        !state.game_telemetry().unwrap().opted_in
            && state.game_telemetry().unwrap().identity.is_none(),
        "the press must not opt the player in or save an identity"
    );
}

fn status_of(step: Result<LogUploadStep, LogUploadError>) -> LogUploadState {
    match step.unwrap() {
        LogUploadStep::Finished(status) => status.state,
        other => panic!("expected a status, got {other:?}"),
    }
}

#[test]
fn no_telemetry_target_is_unreachable_never_disabled() {
    let root = tempfile::tempdir().unwrap();
    let mut state = state_at(root.path(), "http://login.example:8081");
    state.preferences.login_servers = vec![LoginServer {
        name: String::new(),
        url: String::new(),
    }];
    assert!(state.launcher_log_target().is_none());
    let uploader = LogUploader::new();
    for request in [inspect(), upload()] {
        assert_eq!(
            status_of(state.begin_log_upload(request, &uploader)),
            LogUploadState::Unreachable,
            "{request:?}"
        );
    }
    // The refused upload released the busy flag.
    assert_eq!(
        status_of(state.begin_log_upload(upload(), &uploader)),
        LogUploadState::Unreachable
    );
}

#[test]
fn a_second_upload_while_one_runs_is_busy() {
    let root = tempfile::tempdir().unwrap();
    let state = state_at(root.path(), "http://login.example:8081");
    let uploader = LogUploader::new();
    let first = ingest_of(state.begin_log_upload(upload(), &uploader).unwrap());
    assert_eq!(
        state.begin_log_upload(upload(), &uploader).unwrap_err(),
        LogUploadError::Storage(StorageError::Busy)
    );
    assert_eq!(
        status_of(state.begin_log_upload(inspect(), &uploader)),
        LogUploadState::Ready,
        "Inspect is not gated"
    );
    drop(first);
    assert!(state.begin_log_upload(upload(), &uploader).is_ok());
}

#[tokio::test]
async fn a_second_press_after_sent_is_nothing_new_and_no_files_is_no_logs() {
    let root = tempfile::tempdir().unwrap();
    let server = accepting_server().await;
    let state = state_at(root.path(), &server.uri());
    let uploader = LogUploader::new();
    assert_eq!(
        run_upload(&state, &uploader).await.unwrap(),
        LogUploadState::NoLogs
    );
    write_log(root.path(), 5);
    assert!(matches!(
        run_upload(&state, &uploader).await,
        Ok(LogUploadState::Sent { lines_sent: 5, .. })
    ));
    assert_eq!(
        run_upload(&state, &uploader).await.unwrap(),
        LogUploadState::NothingNew
    );
    assert_eq!(chunk_posts(&server).await, 1, "the repeat posted nothing");
}

#[tokio::test]
async fn partial_keeps_the_unsent_lines_for_the_next_press() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "600"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let width = write_log(root.path(), 2_500);
    let state = state_at(root.path(), &server.uri());
    let uploader = LogUploader::new();
    let first = run_upload(&state, &uploader).await.unwrap();
    let LogUploadState::Partial {
        lines_sent,
        lines_total,
        retry_after_secs,
        ..
    } = first
    else {
        panic!("expected partial, got {first:?}");
    };
    assert_eq!(
        (lines_sent, lines_total, retry_after_secs),
        (1_000, 2_500, 600)
    );
    let saved = Offsets::load(&state.directory.root);
    assert_eq!(
        saved.files.get(LOG_FILE).copied(),
        Some(1_000 * width as u64),
        "the offset stops at the last chunk the server accepted"
    );
    let second = run_upload(&state, &uploader).await.unwrap();
    assert!(
        matches!(
            second,
            LogUploadState::Sent {
                lines_sent: 1_500,
                lines_total: 1_500,
                ..
            }
        ),
        "{second:?}"
    );
    assert_eq!(
        Offsets::load(&state.directory.root)
            .files
            .get(LOG_FILE)
            .copied(),
        Some(2_500 * width as u64)
    );
    let mints = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path() == "/api/auth/dev-session")
        .count();
    assert_eq!(
        mints, 1,
        "two presses on one LogUploader share one log session"
    );
}

/// LF-19 N1: a failed offsets save on one chunk does not stop the saves after it.
/// The first chunk blocks the save (a folder), the second unblocks it.
#[tokio::test]
async fn a_failed_offsets_save_on_one_chunk_is_retried_on_the_next() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server).await;
    let width = write_log(root.path(), 2_500);
    let state = state_at(root.path(), &server.uri());
    let blocked = state.directory.root.join("uploaded-log-offsets.json");
    let calls = std::sync::atomic::AtomicUsize::new(0);
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(move |_: &wiremock::Request| {
            match calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) {
                0 => _ = std::fs::create_dir_all(&blocked),
                1 => _ = std::fs::remove_dir(&blocked),
                _ => {}
            }
            ResponseTemplate::new(200)
        })
        .mount(&server)
        .await;
    let uploader = LogUploader::new();
    let out = run_upload(&state, &uploader).await.unwrap();
    assert!(
        matches!(
            out,
            LogUploadState::Sent {
                lines_total: 2_500,
                ..
            }
        ),
        "{out:?}"
    );
    let saved = Offsets::load(&state.directory.root);
    assert_eq!(saved.files.get(LOG_FILE), Some(&(2_500 * width as u64)));
}

#[test]
fn a_cancelled_line_read_says_why_in_one_collect_failed_event() {
    let root = tempfile::tempdir().unwrap();
    write_log(root.path(), 3);
    let state = state_at(root.path(), "http://127.0.0.1:9");
    let uploader = LogUploader::new();
    uploader.cancel();
    // A second live dispatcher keeps the callsite interest from caching "never".
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let (result, logged) = capture(|| {
        let step = state.begin_log_upload(upload(), &uploader).unwrap();
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(ingest_of(step).run_with(&mut FakePacer::default()))
    });
    assert!(matches!(result, Err(LogUploadError::CollectFailed)));
    let collect: Vec<_> = logged
        .iter()
        .filter(|e| e.field("event") == Some("log_upload_collect_failed"))
        .collect();
    assert_eq!(collect.len(), 1, "exactly one collect_failed event");
    assert_eq!(collect[0].level, tracing::Level::WARN);
    assert_eq!(collect[0].field("reason"), Some("lines_read"));
}

#[tokio::test]
async fn a_rejected_upload_is_a_network_error_and_moves_no_offset() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(413))
        .mount(&server)
        .await;
    write_log(root.path(), 4);
    let state = state_at(root.path(), &server.uri());
    assert_eq!(
        run_upload(&state, &LogUploader::new()).await,
        Err(LogUploadError::Network)
    );
    assert!(Offsets::load(&state.directory.root).files.is_empty());
}

#[tokio::test]
async fn a_failed_mint_is_a_network_error_the_player_can_retry() {
    let root = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    write_log(root.path(), 4);
    let state = state_at(root.path(), &server.uri());
    let uploader = LogUploader::new();
    assert_eq!(
        run_upload(&state, &uploader).await,
        Err(LogUploadError::Network),
        "a transient mint failure must not read as `no telemetry server`"
    );
    assert_eq!(chunk_posts(&server).await, 0);
    assert!(Offsets::load(&state.directory.root).files.is_empty());
    // The busy flag is released: a second press is admitted.
    assert!(state.begin_log_upload(upload(), &uploader).is_ok());
}

/// Cancels the token during its first post, as an app shutdown would.
struct CancellingSink {
    cancel: CancellationToken,
    posts: usize,
}
impl ChunkSink for CancellingSink {
    fn post(&mut self, _batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        self.posts += 1;
        self.cancel.cancel();
        std::future::ready(BatchOutcome::Posted)
    }
}

fn line(n: u64, message: &str) -> LogLine {
    LogLine {
        file: LOG_FILE.into(),
        line_no: n,
        end: n * 100,
        ts_ms: Some(1_700_000_000_000),
        level: Some("INFO".into()),
        target: Some("cimmeria_launcher::net".into()),
        message: message.into(),
        truncated: false,
    }
}

#[tokio::test]
async fn a_cancel_mid_run_posts_nothing_more_and_keeps_what_was_sent() {
    let lines: Vec<LogLine> = (1..=2_500).map(|n| line(n, "plain text")).collect();
    let events = rows(
        &lines,
        LAUNCHER_LOG_TARGET,
        "0123456789abcdef",
        "desktop-1",
        "windows",
    );
    let cancel = CancellationToken::new();
    let mut sink = CancellingSink {
        cancel: cancel.clone(),
        posts: 0,
    };
    let mut pacer = FakePacer::default();
    let mut marked = Vec::new();
    let out = send_with(&mut sink, &events, &mut pacer, &cancel, |n| marked.push(n)).await;
    assert_eq!(
        out,
        IngestOutcome::Partial {
            lines_sent: 1_000,
            retry_after_secs: 60
        }
    );
    assert_eq!(
        (sink.posts, marked),
        (1, vec![1_000]),
        "no post after the cancel"
    );
    assert!(
        pacer.sleeps.is_empty(),
        "no paced sleep once the token is cancelled"
    );
}

#[test]
fn every_row_is_scrubbed_before_it_leaves_whatever_made_the_line() {
    let raw = r"opened C:\Users\bob\notes\a.txt from 10.0.0.1 ok";
    for target in [LAUNCHER_LOG_TARGET, GAME_LOG_TARGET] {
        let out = rows(
            &[line(1, raw)],
            target,
            "0123456789abcdef",
            "desktop-1",
            "windows",
        );
        let TelemetryEvent::ClientNative(row) = &out[0] else {
            panic!("a log row is a client_native event");
        };
        let message = row.fields["message"].as_str().unwrap();
        assert!(!message.contains("bob"), "{target}: {message}");
        assert!(!message.contains("a.txt"), "{target}: {message}");
        assert!(!message.contains("10.0.0.1"), "{target}: {message}");
        // A line with only an address keeps its words and loses the address.
        let out = rows(
            &[line(2, "peer 10.0.0.1 closed")],
            target,
            "0123456789abcdef",
            "desktop-1",
            "windows",
        );
        let TelemetryEvent::ClientNative(row) = &out[0] else {
            panic!("a log row is a client_native event");
        };
        let message = row.fields["message"].as_str().unwrap();
        assert!(!message.contains("10.0.0.1"), "{target}: {message}");
        assert!(
            message.starts_with("peer ") && message.ends_with(" closed"),
            "{message}"
        );
    }
}

#[test]
fn the_new_states_serialize_flat_for_the_webview() {
    let sent = LogUploadStatus::new(LogUploadState::Sent {
        lines_sent: 3,
        lines_total: 4,
        upload_id: "0123456789abcdef".into(),
    });
    assert_eq!(
        serde_json::to_value(sent).unwrap(),
        serde_json::json!({"schema_version": 1, "state": "sent", "lines_sent": 3,
            "lines_total": 4, "upload_id": "0123456789abcdef"})
    );
    let partial = LogUploadStatus::new(LogUploadState::Partial {
        lines_sent: 1,
        lines_total: 4,
        upload_id: "0123456789abcdef".into(),
        retry_after_secs: 60,
    });
    assert_eq!(
        serde_json::to_value(partial).unwrap(),
        serde_json::json!({"schema_version": 1, "state": "partial", "lines_sent": 1,
            "lines_total": 4, "upload_id": "0123456789abcdef", "retry_after_secs": 60})
    );
    assert_eq!(
        serde_json::to_value(LogUploadStatus::new(LogUploadState::Unreachable)).unwrap(),
        serde_json::json!({"schema_version": 1, "state": "unreachable"})
    );
}

#[path = "ingest_flow_game_tests.rs"]
mod game;
