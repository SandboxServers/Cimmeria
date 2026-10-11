use super::*;
use crate::game_telemetry::start;
use crate::storage::DesktopState;
use cimmeria_launcher_core::telemetry::events::ClientLogEvent;
use std::io::Read;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";

fn servers(server: &MockServer) -> Vec<LoginServer> {
    vec![LoginServer {
        name: "fixture".into(),
        url: server.uri(),
    }]
}

/// A mint or refresh reply. The token lives 8 h from now, so the proactive
/// refresh stays out of the way unless a test asks for it.
fn reply(server: &MockServer, token: &str) -> ResponseTemplate {
    let expires = chrono::Utc::now().timestamp_millis() + 8 * 60 * 60 * 1000;
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "session_id": "session-1", "token": token, "expires_at_ms": expires,
        "upload_endpoint": format!("{}/api/telemetry", server.uri()),
        "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
    }))
}

async fn mount_mint(server: &MockServer, token: &str) {
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(reply(server, token))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

fn opted_in(root: &Path) -> DesktopState {
    let mut state = DesktopState::open(root).unwrap();
    state.set_game_telemetry(true).unwrap();
    state
}

/// Play's mint, then the transport on the session it returned.
async fn open(state: &DesktopState, server: &MockServer, install: &Path) -> Transport {
    let consent = Consent::read(state).unwrap().expect("opted in");
    let session = start(&consent.identity, &servers(server), &["desktop-launcher"])
        .await
        .unwrap();
    Transport::open(consent, &session, &servers(server), install).unwrap()
}

fn event(message: &str) -> TelemetryEvent {
    TelemetryEvent::ClientLog(ClientLogEvent {
        ts_ms: 0,
        seq: 0,
        source_file: "SGWDebugLog.log".into(),
        level: "info".into(),
        category: "raw".into(),
        packet_no: None,
        message: message.into(),
    })
}

/// The messages of every chunk the server took, in order.
async fn delivered(server: &MockServer, token: &str) -> Vec<String> {
    let bearer = format!("Bearer {token}");
    let mut out = Vec::new();
    for request in server.received_requests().await.unwrap() {
        if request.url.path() != CHUNK || !authorized(&request, &bearer) {
            continue;
        }
        let mut text = String::new();
        flate2::read::GzDecoder::new(request.body.as_slice())
            .read_to_string(&mut text)
            .unwrap();
        for line in text.lines() {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            out.push(value["message"].as_str().unwrap_or_default().to_string());
        }
    }
    out
}

fn authorized(request: &Request, bearer: &str) -> bool {
    request
        .headers
        .get("authorization")
        .is_some_and(|v| v.as_bytes() == bearer.as_bytes())
}

fn chunk_posts(requests: &[Request]) -> usize {
    requests.iter().filter(|r| r.url.path() == CHUNK).count()
}

/// A 401 from the upload refreshes the token with the old one, and the same
/// events go again under the new one: nothing is lost or sent twice.
#[tokio::test]
async fn a_401_refreshes_the_token_and_the_flush_succeeds() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .and(header("authorization", "Bearer tok-1"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session/refresh"))
        .and(body_json(serde_json::json!({"token": "tok-1"})))
        .respond_with(reply(&server, "tok-2"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .and(header("authorization", "Bearer tok-2"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    transport.enqueue(event("first")).await.unwrap();
    transport.enqueue(event("second")).await.unwrap();
    assert_eq!(transport.flush().await, Ok(2));
    assert_eq!(delivered(&server, "tok-2").await, ["first", "second"]);
    // Nothing is left to send.
    assert_eq!(transport.flush().await, Ok(0));
    server.verify().await;
}

/// The server refuses a refresh of an expired token (or past the session
/// cap): the transport mints a new session and sends under it.
#[tokio::test]
async fn a_refused_refresh_mints_a_new_session() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    mount_mint(&server, "tok-3").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .and(header("authorization", "Bearer tok-1"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session/refresh"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .and(header("authorization", "Bearer tok-3"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    transport.enqueue(event("kept")).await.unwrap();
    assert_eq!(transport.flush().await, Ok(1));
    assert_eq!(delivered(&server, "tok-3").await, ["kept"]);
    server.verify().await;
}

/// A 429 keeps the events and sends nothing until its `Retry-After` has
/// passed; then the same events go.
#[tokio::test]
async fn a_429_backs_off_for_retry_after_then_retries() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    for message in ["a", "b", "c"] {
        transport.enqueue(event(message)).await.unwrap();
    }
    assert_eq!(transport.flush().await, Err(TransportError::BackingOff));
    // Inside the back-off nothing is posted.
    assert_eq!(transport.flush().await, Ok(0));
    assert_eq!(chunk_posts(&server.received_requests().await.unwrap()), 1);

    tokio::time::sleep(Duration::from_millis(1_100)).await;
    assert_eq!(transport.flush().await, Ok(3));
    let requests = server.received_requests().await.unwrap();
    assert_eq!(chunk_posts(&requests), 2);
    // Both posts carried the same three events.
    assert_eq!(
        delivered(&server, "tok-1").await,
        ["a", "b", "c", "a", "b", "c"]
    );
    server.verify().await;
}

/// Events queued before a launcher restart (one from a session, one with no
/// session at all) are in `telemetry-queue.jsonl` in the state directory and
/// go out with the next session's first flush.
#[tokio::test]
async fn the_queue_survives_a_reopen_and_ships_with_the_next_session() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    {
        let offline = MockServer::start().await;
        mount_mint(&offline, "tok-old").await;
        let state = opted_in(root.path());
        let transport = open(&state, &offline, install.path()).await;
        transport.enqueue(event("before-restart")).await.unwrap();
        let consent = Consent::read(&state).unwrap().unwrap();
        queue_without_session(&consent, event("install-result"))
            .await
            .unwrap();
        // The launcher closes before any flush.
        drop(transport);
        drop(state);
    }
    assert!(root.path().join("telemetry-queue.jsonl").is_file());

    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(recover_on_startup(&state).await, Recovery::Pending(2));
    let server = MockServer::start().await;
    mount_mint(&server, "tok-new").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let transport = open(&state, &server, install.path()).await;
    assert_eq!(transport.flush().await, Ok(2));
    assert_eq!(
        delivered(&server, "tok-new").await,
        ["before-restart", "install-result"]
    );
    assert!(!root.path().join("telemetry-queue.jsonl").exists());
}

/// No consent, no transport: an opted-out player has nothing to open one
/// with, and the queue left from before the opt-out is deleted at startup,
/// so opting back in later does not ship it.
#[tokio::test]
async fn an_opted_out_player_sends_nothing() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    {
        let state = DesktopState::open(root.path()).unwrap();
        // Never opted in: no consent, and no queue to recover.
        assert!(Consent::read(&state).unwrap().is_none());
        assert_eq!(recover_on_startup(&state).await, Recovery::Empty);
    }
    let mut state = opted_in(root.path());
    let consent = Consent::read(&state).unwrap().unwrap();
    queue_without_session(&consent, event("collected-while-opted-in"))
        .await
        .unwrap();
    state.set_game_telemetry(false).unwrap();
    assert!(Consent::read(&state).unwrap().is_none());
    assert_eq!(recover_on_startup(&state).await, Recovery::Discarded);
    assert!(!root.path().join("telemetry-queue.jsonl").exists());

    // Opting back in starts from an empty queue.
    state.set_game_telemetry(true).unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let transport = open(&state, &server, install.path()).await;
    assert_eq!(transport.flush().await, Ok(0));
    server.verify().await;
}

/// Error codes carry no token, address or server text.
#[test]
fn transport_errors_are_codes() {
    let text = serde_json::to_string(&TransportError::TokenRejected).unwrap();
    assert_eq!(text, "\"token_rejected\"");
    assert_eq!(
        serde_json::to_string(&TransportError::BackingOff).unwrap(),
        "\"backing_off\""
    );
}
