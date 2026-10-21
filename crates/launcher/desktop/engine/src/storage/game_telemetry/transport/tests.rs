use super::*;
use crate::game_telemetry::start;
use crate::storage::DesktopState;
use cimmeria_launcher_core::telemetry::events::ClientLogEvent;
use std::io::Read;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

pub(super) const CHUNK: &str = "/api/telemetry/upload-chunk";

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

pub(super) async fn mount_mint(server: &MockServer, token: &str) {
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(reply(server, token))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

pub(super) fn opted_in(root: &Path) -> DesktopState {
    let mut state = DesktopState::open(root).unwrap();
    state.set_game_telemetry(true).unwrap();
    state
}

/// Play's mint, then the transport on the session it returned.
pub(super) async fn open(state: &DesktopState, server: &MockServer, install: &Path) -> Transport {
    let consent = Consent::read(state).unwrap().expect("opted in");
    let session = start(&consent.identity, &servers(server), &["desktop-launcher"])
        .await
        .unwrap();
    Transport::open(consent, &session, &servers(server), install).unwrap()
}

pub(super) fn event(message: &str) -> TelemetryEvent {
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
/// with. **Opting out deletes the queue at once**, so opting back in during
/// the same run (no startup recovery in between) does not ship it, nor an
/// in-flight chunk a flush had already taken.
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
    for message in ["in-flight", "collected-while-opted-in"] {
        queue_without_session(&consent, event(message))
            .await
            .unwrap();
    }
    // A flush took one chunk and was cut off before the server answered.
    let core = DiskQueue::new(root.path());
    drop(
        core.take_batch::<TelemetryEvent>(u64::MAX, 1)
            .unwrap()
            .unwrap(),
    );
    queue_without_session(&consent, event("after-the-take"))
        .await
        .unwrap();

    state.set_game_telemetry(false).unwrap();
    assert!(Consent::read(&state).unwrap().is_none());
    assert_eq!(core.pending_count::<TelemetryEvent>().unwrap(), 0);

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

/// The sync write behind the install result and the exit-time
/// client-patches events keeps the same rule: a consent read before an
/// opt-out writes nothing after it, and leaves no queue on disk.
#[test]
fn queue_now_after_an_opt_out_writes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let mut state = opted_in(root.path());
    let consent = Consent::read(&state).unwrap().unwrap();
    queue_now(&consent, &event("before")).unwrap();
    state.set_game_telemetry(false).unwrap();
    assert_eq!(
        queue_now(&consent, &event("after")),
        Err(TransportError::NotOptedIn)
    );
    let core = DiskQueue::new(root.path());
    assert_eq!(core.pending_count::<TelemetryEvent>().unwrap(), 0);
    assert!(!root.path().join("telemetry-queue.jsonl").exists());
}

/// Both writes refuse after an opt-out.
async fn refuse_writes(transport: &Transport, consent: &Consent) {
    assert_eq!(
        transport.enqueue(event("after")).await,
        Err(TransportError::NotOptedIn)
    );
    assert_eq!(
        queue_without_session(consent, event("after")).await,
        Err(TransportError::NotOptedIn)
    );
}

/// A transport opened before an opt-out refuses everything after it and
/// leaves no queue. A write that raced the opt-out's delete (put on disk
/// here directly) is neither sent by a flush, shutdown or bundle nor added
/// to: a refused write never touches the queue.
#[tokio::test]
async fn a_transport_from_before_an_opt_out_refuses_every_call() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let sessions =
        cimmeria_launcher_core::install_layout::binaries_dir(install.path()).join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::write(sessions.join("a.log"), "a session log\n").unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    for route in [CHUNK, "/api/telemetry/upload-bundle"] {
        Mock::given(method("POST"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
    }
    let mut state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    let consent = Consent::read(&state).unwrap().unwrap();

    state.set_game_telemetry(false).unwrap();
    assert!(!transport.consented());
    refuse_writes(&transport, &consent).await;
    for name in ["telemetry-queue.jsonl", "telemetry-queue.inflight.jsonl"] {
        assert!(!root.path().join(name).exists(), "{name} left behind");
    }

    // A write that raced the opt-out's delete. Its own writer deletes it
    // again; until then, nothing here sends it or adds to it.
    let queue = DiskQueue::new(root.path());
    queue.enqueue(&event("raced the opt-out")).unwrap();
    assert!(matches!(
        transport.upload_bundle(1).await,
        Err(TransportError::NotOptedIn)
    ));
    assert_eq!(transport.flush().await, Err(TransportError::NotOptedIn));
    refuse_writes(&transport, &consent).await;
    assert_eq!(queue.pending_count::<TelemetryEvent>().unwrap(), 1);
    assert_eq!(transport.shutdown().await, Err(TransportError::NotOptedIn));
    assert_eq!(queue.pending_count::<TelemetryEvent>().unwrap(), 1);
    server.verify().await;
}

/// Opt out, then back in, and a session on the new consent: the old
/// transport's late clean-up (a write it finished after the opt-out) does
/// not count as another change, so the new session keeps sending.
#[tokio::test]
async fn an_old_transports_late_undo_does_not_stop_a_newer_session() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    mount_mint(&server, "tok-2").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let mut state = opted_in(root.path());
    let old = open(&state, &server, install.path()).await;
    state.set_game_telemetry(false).unwrap();
    state.set_game_telemetry(true).unwrap();
    let new = open(&state, &server, install.path()).await;

    assert_eq!(
        old.consent.undo_if_withdrawn(),
        Err(TransportError::NotOptedIn)
    );
    assert!(new.consented());
    new.enqueue(event("on the new consent")).await.unwrap();
    assert_eq!(new.flush().await, Ok(1));
    assert_eq!(delivered(&server, "tok-2").await, ["on the new consent"]);
}

/// A queue an opted-out player still has at startup (an opt-out whose
/// delete failed) is deleted unsent.
#[tokio::test]
async fn startup_deletes_an_opted_out_players_queue() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    DiskQueue::new(root.path())
        .enqueue(&event("left-behind"))
        .unwrap();
    assert_eq!(recover_on_startup(&state).await, Recovery::Discarded);
    assert!(!root.path().join("telemetry-queue.jsonl").exists());
}

/// **Closing the launcher mid-flush loses nothing, and an enqueue never
/// waits for a flush.** The flush future is dropped while its chunk is on
/// the wire; an event queued meanwhile goes straight to disk; the next
/// launcher run finds all three and sends them in order.
#[tokio::test]
async fn a_flush_cut_off_mid_post_keeps_its_events_and_never_blocks_enqueue() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    {
        let slow = MockServer::start().await;
        mount_mint(&slow, "tok-slow").await;
        Mock::given(method("POST"))
            .and(path(CHUNK))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(30)))
            .mount(&slow)
            .await;
        let state = opted_in(root.path());
        let transport = open(&state, &slow, install.path()).await;
        transport.enqueue(event("a")).await.unwrap();
        transport.enqueue(event("b")).await.unwrap();
        let meanwhile = async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            tokio::time::timeout(Duration::from_millis(300), transport.enqueue(event("c"))).await
        };
        let (flushed, queued) = tokio::join!(
            tokio::time::timeout(Duration::from_secs(1), transport.flush()),
            meanwhile
        );
        assert!(flushed.is_err(), "the flush must be cut off mid-POST");
        assert_eq!(queued, Ok(Ok(())), "enqueue waited on the flush");
        assert_eq!(chunk_posts(&slow.received_requests().await.unwrap()), 1);
    }

    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(recover_on_startup(&state).await, Recovery::Pending(3));
    let server = MockServer::start().await;
    mount_mint(&server, "tok-new").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let transport = open(&state, &server, install.path()).await;
    // The cut-off chunk first, then what was queued behind it.
    assert_eq!(transport.flush().await, Ok(3));
    assert_eq!(chunk_posts(&server.received_requests().await.unwrap()), 2);
    assert_eq!(delivered(&server, "tok-new").await, ["a", "b", "c"]);
}

/// A refresh that hands back plain http to a host that is not a login
/// server is refused: the old endpoint and token stay, nothing is sent
/// there, and flushes hold off rather than refreshing again at once.
#[tokio::test]
async fn a_refresh_to_an_unchecked_http_endpoint_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    let expires = chrono::Utc::now().timestamp_millis() + 8 * 60 * 60 * 1000;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "session-1", "token": "tok-2", "expires_at_ms": expires,
            "upload_endpoint": "http://uploads.example.org/api",
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    transport.enqueue(event("kept")).await.unwrap();
    assert_eq!(
        transport.flush().await,
        Err(TransportError::EndpointRefused)
    );
    {
        let session = transport.telemetry.session.read().await;
        assert_eq!(
            session.upload_endpoint,
            format!("{}/api/telemetry", server.uri())
        );
        assert_eq!(session.token, "tok-1");
    }
    // Held off: no second refresh, no second chunk.
    assert_eq!(transport.flush().await, Ok(0));
    server.verify().await;
    assert_eq!(
        DiskQueue::new(root.path())
            .pending_count::<TelemetryEvent>()
            .unwrap(),
        1
    );
}

/// C10 survives a refresh: a refresh reply that advertises the loopback address
/// again is replaced by the login server's `/api/telemetry`, as at mint.
#[tokio::test]
async fn a_refresh_keeps_the_login_servers_upload_address() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    let expires = chrono::Utc::now().timestamp_millis() + 8 * 60 * 60 * 1000;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session/refresh"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "session-1", "token": "tok-2", "expires_at_ms": expires,
            "upload_endpoint": "http://127.0.0.1:8081/api/telemetry",
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
        })))
        .expect(1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let mut transport = open(&state, &server, install.path()).await;
    // A remote https login server: the loopback address in the refresh reply is misconfigured.
    transport.login_servers[0].url = "https://play.example".to_string();
    transport.renew().await.unwrap();
    assert_eq!(
        transport.telemetry.session.read().await.upload_endpoint,
        "https://play.example/api/telemetry"
    );
}

/// **A server that keeps refusing tokens is not hammered.** A refused
/// refresh and a re-mint the server answers 503 hold flushes off: the next
/// flush sends nothing to any route.
#[tokio::test]
async fn a_failed_renewal_holds_flushes_off() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "1"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
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

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    transport.enqueue(event("kept")).await.unwrap();
    assert_eq!(
        transport.flush().await,
        Err(TransportError::SessionUnavailable)
    );
    let before = server.received_requests().await.unwrap().len();
    for _ in 0..3 {
        assert_eq!(transport.flush().await, Ok(0));
    }
    assert_eq!(server.received_requests().await.unwrap().len(), before);
    server.verify().await;
    let hold = transport.auth_hold.lock().unwrap();
    assert_eq!(hold.next, AUTH_HOLD_FIRST * 2);
}

/// **A server that renews tokens and then refuses them is not hammered
/// either.** The upload refuses the token it was just renewed to: flushes
/// hold off rather than refreshing on every tick, and the waits keep
/// doubling across rounds.
#[tokio::test]
async fn a_token_refused_after_renewal_holds_flushes_off() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session/refresh"))
        .respond_with(reply(&server, "tok-2"))
        .expect(2)
        .mount(&server)
        .await;
    let refreshes = |requests: Vec<Request>| {
        requests
            .iter()
            .filter(|r| r.url.path() == "/api/auth/dev-session/refresh")
            .count()
    };

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    transport.enqueue(event("kept")).await.unwrap();
    assert_eq!(transport.flush().await, Err(TransportError::TokenRejected));
    assert_eq!(
        transport.auth_hold.lock().unwrap().next,
        AUTH_HOLD_FIRST * 2
    );
    for _ in 0..3 {
        assert_eq!(transport.flush().await, Ok(0));
    }
    assert_eq!(refreshes(server.received_requests().await.unwrap()), 1);

    // The hold runs out: one more round, and the next wait is longer.
    transport.auth_hold.lock().unwrap().until = None;
    assert_eq!(transport.flush().await, Err(TransportError::TokenRejected));
    assert_eq!(
        transport.auth_hold.lock().unwrap().next,
        AUTH_HOLD_FIRST * 4
    );
    server.verify().await;
}

/// Once the server takes events again the back-off starts from its first
/// step; a flush with nothing to send proves nothing and leaves it.
#[tokio::test]
async fn an_accepted_flush_resets_the_token_back_off() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    // A long-running back-off whose current wait has passed.
    transport.auth_hold.lock().unwrap().next = AUTH_HOLD_FIRST * 8;
    assert_eq!(transport.flush().await, Ok(0));
    assert_eq!(
        transport.auth_hold.lock().unwrap().next,
        AUTH_HOLD_FIRST * 8
    );
    transport.enqueue(event("sent")).await.unwrap();
    assert_eq!(transport.flush().await, Ok(1));
    assert_eq!(transport.auth_hold.lock().unwrap().next, AUTH_HOLD_FIRST);
    server.verify().await;
}

/// **Shutdown sends the whole backlog**, not the cadence's eight chunks.
#[tokio::test]
async fn shutdown_sends_more_than_eight_chunks() {
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, "tok-1").await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(200))
        .expect(MAX_CHUNKS_PER_FLUSH as u64 + 1)
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    let total = MAX_CHUNKS_PER_FLUSH as u64 * 1_000 + 1;
    for i in 0..total {
        transport.enqueue(event(&i.to_string())).await.unwrap();
    }
    assert_eq!(transport.shutdown().await, Ok(total));
    server.verify().await;
}

/// The doubling stops at the cap.
#[test]
fn the_token_back_off_doubles_up_to_its_cap() {
    let mut hold = AuthHold::new();
    let mut waits = Vec::new();
    for _ in 0..8 {
        waits.push(hold.next);
        hold.next = (hold.next * 2).min(AUTH_HOLD_MAX);
    }
    assert_eq!(waits[0], Duration::from_secs(60));
    assert_eq!(waits[1], Duration::from_secs(120));
    assert_eq!(*waits.last().unwrap(), AUTH_HOLD_MAX);
}

/// **Server text and tokens stay out of the logs and the errors.** Every
/// reply below carries a marker string; the chunk 500, the 401 and the
/// failed refresh are all logged (as codes), and neither the marker nor the
/// token appears in a log line or an error's text.
#[tokio::test]
async fn transport_errors_and_logs_carry_no_server_text_or_token() {
    const MARKER: &str = "MARKER-7f3a-server-text";
    const TOKEN: &str = "tok-secret-91c2";
    let lines = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    // With exactly one dispatcher alive, tracing works out a new callsite's
    // interest from the default of whichever thread hits it first; under
    // `cargo test` that is another test's thread with no subscriber, and the
    // callsite is cached as never enabled. A second live dispatcher makes it
    // ask every dispatcher, this capture included.
    let _second = tracing::Dispatch::new(Capture(Default::default()));
    let _capture =
        tracing::dispatcher::set_default(&tracing::Dispatch::new(Capture(lines.clone())));

    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let server = MockServer::start().await;
    mount_mint(&server, TOKEN).await;
    let marked = |status: u16| ResponseTemplate::new(status).set_body_string(MARKER);
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(marked(500))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(marked(401))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session/refresh"))
        .respond_with(marked(500))
        .mount(&server)
        .await;

    let state = opted_in(root.path());
    let transport = open(&state, &server, install.path()).await;
    transport.enqueue(event("e")).await.unwrap();
    let errors = [transport.flush().await, transport.flush().await];
    assert_eq!(
        errors,
        [
            Err(TransportError::Upload),
            Err(TransportError::SessionUnavailable)
        ]
    );
    let lines = lines.lock().unwrap().clone();
    assert!(
        lines.contains("telemetry flush failed") && lines.contains("not renewed"),
        "the capture saw nothing: {lines}"
    );
    for error in errors.iter().filter_map(|r| r.as_ref().err()) {
        let text = format!(
            "{error} {error:?} {}",
            serde_json::to_string(error).unwrap()
        );
        assert!(!text.contains(MARKER) && !text.contains(TOKEN), "{text}");
    }
    assert!(!lines.contains(MARKER), "server text logged: {lines}");
    assert!(!lines.contains(TOKEN), "token logged: {lines}");
}

/// Every event and span field from this repo's crates, as text.
struct Capture(std::sync::Arc<std::sync::Mutex<String>>);

impl Capture {
    fn push(&self, values: &dyn Fn(&mut dyn tracing::field::Visit)) {
        let mut text = String::new();
        values(&mut Fields(&mut text));
        let mut lines = self.0.lock().unwrap();
        lines.push_str(&text);
        lines.push('\n');
    }
}

struct Fields<'a>(&'a mut String);

impl tracing::field::Visit for Fields<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        use std::fmt::Write;
        let _ = write!(self.0, "{}={value:?} ", field.name());
    }
}

impl tracing::Subscriber for Capture {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target().starts_with("cimmeria") || metadata.target().starts_with("launcher.")
    }
    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        self.push(&|v| span.record(v));
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        self.push(&|v| values.record(v));
    }
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        self.push(&|v| event.record(v));
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}
