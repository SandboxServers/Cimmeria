use super::launcher_session::{LauncherSender, LauncherTarget, SendOutcome, LAUNCHER_SESSION_KIND};
use super::session::mint_body_for_test;
use super::*;
use crate::client_setup::login_servers::LoginServer;
use cimmeria_launcher_core::telemetry::events::{ClientLogEvent, TelemetryEvent};
use std::time::Duration;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";

fn target(url: &str) -> LauncherTarget {
    LauncherTarget {
        identity: Identity {
            install_id: Uuid::from_u128(7),
            machine_id: "0123456789abcdef".into(),
        },
        servers: vec![LoginServer {
            name: "fixture".into(),
            url: url.into(),
        }],
    }
}

fn event() -> TelemetryEvent {
    TelemetryEvent::ClientLog(ClientLogEvent {
        ts_ms: 1,
        seq: 1,
        source_file: "launcher".into(),
        level: "info".into(),
        category: "adoption".into(),
        packet_no: None,
        message: "step".into(),
    })
}

/// A mint reply whose token is good for an hour, so a send does not re-mint.
fn reply(upload_endpoint: &str) -> ResponseTemplate {
    let expires_at_ms = chrono::Utc::now().timestamp_millis() + 3_600_000;
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "session_id": "session-1", "token": "payload.sig", "expires_at_ms": expires_at_ms,
        "upload_endpoint": upload_endpoint, "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
    }))
}

/// Mounts the launcher mint route, which only matches a launcher-kind body.
async fn mount_mint(server: &MockServer, times: u64) {
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .and(body_partial_json(
            serde_json::json!({"session_kind": "launcher", "machine_id": ""}),
        ))
        .respond_with(reply(&format!("{}/api/telemetry", server.uri())))
        .expect(times)
        .mount(server)
        .await;
}

/// Mounts the chunk route with one answer for every upload.
async fn mount_chunk(server: &MockServer, status: u16, retry_after: Option<&str>) {
    let mut response = ResponseTemplate::new(status);
    if let Some(seconds) = retry_after {
        response = response.insert_header("Retry-After", seconds);
    }
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(response)
        .mount(server)
        .await;
}

#[test]
fn a_player_mint_body_has_no_session_kind() {
    assert!(mint_body_for_test(None).get("session_kind").is_none());
    assert_eq!(
        mint_body_for_test(Some(LAUNCHER_SESSION_KIND))["session_kind"],
        "launcher"
    );
    assert_eq!(
        mint_body_for_test(Some(LAUNCHER_SESSION_KIND))["machine_id"],
        ""
    );
    assert_eq!(mint_body_for_test(None)["machine_id"], "0123456789abcdef");
}

#[test]
fn the_target_needs_the_opt_in() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    assert!(state.launcher_target().is_none());
    state.set_game_telemetry(true).unwrap();
    let found = state
        .launcher_target()
        .expect("an opted-in player has a target");
    assert!(!found.servers.is_empty());
    state.set_game_telemetry(false).unwrap();
    assert!(
        state.launcher_target().is_none(),
        "an opted-out player has no target"
    );
}

#[tokio::test]
async fn a_launcher_session_is_minted_and_events_are_posted() {
    let server = MockServer::start().await;
    mount_mint(&server, 1).await;
    mount_chunk(&server, 200, None).await;
    let sender = LauncherSender::new();
    let target = target(&server.uri());
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Sent);
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Sent);
}

#[tokio::test]
async fn offline_fails_silently_then_holds_with_backoff() {
    let sender = LauncherSender::new();
    let target = target("http://127.0.0.1:9");
    assert_eq!(
        sender.send(&target, &[event()]).await,
        SendOutcome::MintFailed {
            reason: "transport"
        }
    );
    let first = sender.hold_remaining().expect("a failed mint holds");
    assert!(first >= Duration::from_secs(50) && first <= Duration::from_secs(60));
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Held);
    sender.clear_hold();
    assert_eq!(
        sender.send(&target, &[event()]).await,
        SendOutcome::MintFailed {
            reason: "transport"
        }
    );
    let second = sender.hold_remaining().expect("the second failure holds");
    assert!(second >= Duration::from_secs(110) && second <= Duration::from_secs(120));
}

#[tokio::test]
async fn a_rejected_token_is_reminted_once() {
    let server = MockServer::start().await;
    mount_mint(&server, 2).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(401))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    mount_chunk(&server, 200, None).await;
    let sender = LauncherSender::new();
    assert_eq!(
        sender.send(&target(&server.uri()), &[event()]).await,
        SendOutcome::Sent
    );
}

#[tokio::test]
async fn a_forbidden_upload_is_not_reminted() {
    let server = MockServer::start().await;
    mount_mint(&server, 1).await;
    mount_chunk(&server, 403, None).await;
    let sender = LauncherSender::new();
    let target = target(&server.uri());
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Dropped);
    assert_ne!(sender.send(&target, &[event()]).await, SendOutcome::Held);
}

#[tokio::test]
async fn a_retry_after_holds_the_sender() {
    let server = MockServer::start().await;
    mount_mint(&server, 1).await;
    // One upload is answered; a second would find no mock and fail the expectation.
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "600"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    let sender = LauncherSender::new();
    let target = target(&server.uri());
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Dropped);
    let held = sender.hold_remaining().expect("a 429 holds");
    assert!(held >= Duration::from_secs(590) && held <= Duration::from_secs(600));
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Held);
    sender.clear_hold();

    // Retry-After 5 still holds for the 60 s floor.
    mount_chunk(&server, 429, Some("5")).await;
    assert_eq!(sender.send(&target, &[event()]).await, SendOutcome::Dropped);
    let floor = sender.hold_remaining().expect("a 429 holds");
    assert!(floor >= Duration::from_secs(55));
}

#[tokio::test]
async fn an_empty_batch_sends_nothing() {
    let server = MockServer::start().await;
    mount_mint(&server, 0).await;
    let sender = LauncherSender::new();
    assert_eq!(
        sender.send(&target(&server.uri()), &[]).await,
        SendOutcome::Empty
    );
}
