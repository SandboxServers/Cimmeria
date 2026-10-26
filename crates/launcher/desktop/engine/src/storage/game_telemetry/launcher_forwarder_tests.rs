use super::launcher_event::LauncherRecord;
use super::launcher_forwarder::{consent_changed, consent_epoch, ForwardSource, Forwarder};
use super::launcher_queue::FLUSH_TARGET;
use super::launcher_session::{LauncherSender, LauncherTarget, SendOutcome};
use super::transport::TelemetryEvent;
use super::Identity;
use cimmeria_launcher_core::telemetry::events::ClientLogEvent;
use serde_json::{Map, Value};
use std::cell::Cell;
use std::time::{Duration, Instant};
use uuid::Uuid;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHUNK: &str = "/api/telemetry/upload-chunk";

/// A host stand-in. `events` maps each record to one client log event.
struct FakeSource {
    target: Option<LauncherTarget>,
    epoch: Cell<u64>,
    opted_in: bool,
    bump_on_events: bool,
}

impl ForwardSource for FakeSource {
    fn events(&self, records: &[LauncherRecord]) -> Option<Vec<TelemetryEvent>> {
        if !self.opted_in {
            return None;
        }
        if self.bump_on_events {
            self.epoch.set(self.epoch.get() + 1);
        }
        Some(records.iter().map(|_| event()).collect())
    }

    fn target(&self) -> Option<LauncherTarget> {
        self.target.clone()
    }

    fn epoch(&self) -> u64 {
        self.epoch.get()
    }
}

/// A client log event; the forwarder never looks inside it.
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

fn target(url: &str) -> LauncherTarget {
    LauncherTarget {
        identity: Identity {
            install_id: Uuid::from_u128(7),
            machine_id: "0123456789abcdef".into(),
        },
        servers: vec![crate::client_setup::login_servers::LoginServer {
            name: "fixture".into(),
            url: url.into(),
        }],
    }
}

/// An opted-in source whose target is the mock server.
fn source(url: &str) -> FakeSource {
    FakeSource {
        target: Some(target(url)),
        epoch: Cell::new(0),
        opted_in: true,
        bump_on_events: false,
    }
}

fn record(target: &'static str, name: Option<&str>) -> LauncherRecord {
    let mut fields = Map::new();
    if let Some(name) = name {
        fields.insert("event".into(), Value::String(name.to_string()));
    }
    LauncherRecord {
        ts_ms: 1,
        level: "info".into(),
        target,
        fields,
    }
}

fn sentinel() -> LauncherRecord {
    record(FLUSH_TARGET, None)
}

/// Mounts the launcher mint route for one session, which only matches a launcher-kind body.
async fn mount_mint(server: &MockServer) {
    let expires_at_ms = chrono::Utc::now().timestamp_millis() + 3_600_000;
    let reply = ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "session_id": "session-1", "token": "payload.sig", "expires_at_ms": expires_at_ms,
        "upload_endpoint": format!("{}/api/telemetry", server.uri()),
        "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
    }));
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .and(body_partial_json(
            serde_json::json!({"session_kind": "launcher", "machine_id": ""}),
        ))
        .respond_with(reply)
        .expect(1)
        .mount(server)
        .await;
}

/// Mounts a chunk route that answers `status` once, for a send that must happen.
async fn mount_chunk(server: &MockServer, status: u16) {
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(status))
        .expect(1)
        .mount(server)
        .await;
}

/// Mounts a catch-all POST route that must never be hit.
async fn mount_no_post(server: &MockServer) {
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(server)
        .await;
}

#[tokio::test]
async fn a_throttled_event_is_kept_then_sent() {
    let server = MockServer::start().await;
    mount_mint(&server).await;
    Mock::given(method("POST"))
        .and(path(CHUNK))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "60"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    mount_chunk(&server, 200).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let src = source(&server.uri());
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![record("client.launcher.adoption", Some("step"))],
        t0,
    );

    let first = forwarder
        .pump(&src, t0 + Duration::from_secs(2))
        .await
        .expect("the event is due at the coalescing point");
    assert_eq!(first.outcome, SendOutcome::Dropped);
    assert_eq!(first.lost_events, 0);
    assert_eq!(forwarder.queued(), 1);

    forwarder.sender().clear_hold();
    let second = forwarder
        .pump(&src, t0 + Duration::from_secs(10))
        .await
        .expect("the kept event is due again after the spacing");
    assert_eq!(second.outcome, SendOutcome::Sent);
    assert_eq!(second.sent_events, 1);
}

#[tokio::test]
async fn withdrawn_consent_drops_the_queue_before_any_send() {
    let server = MockServer::start().await;
    mount_no_post(&server).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let mut src = source(&server.uri());
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![
            record("client.launcher.adoption", Some("step")),
            record("client.launcher.adoption", Some("step")),
        ],
        t0,
    );
    src.target = None;
    assert!(forwarder.pump(&src, t0).await.is_none());
    assert_eq!(forwarder.queued(), 0);
}

#[tokio::test]
async fn a_new_epoch_drops_the_queue_before_any_send() {
    let server = MockServer::start().await;
    mount_no_post(&server).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let src = source(&server.uri());
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![
            record("client.launcher.adoption", Some("step")),
            record("client.launcher.adoption", Some("step")),
        ],
        t0,
    );
    src.epoch.set(src.epoch.get() + 1);
    assert!(forwarder.pump(&src, t0).await.is_none());
    assert_eq!(forwarder.queued(), 0);
}

#[tokio::test]
async fn a_flush_sentinel_sends_at_once_and_finishes_the_flush() {
    let server = MockServer::start().await;
    mount_mint(&server).await;
    mount_chunk(&server, 200).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let src = source(&server.uri());
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![sentinel(), record("client.launcher.adoption", Some("step"))],
        t0,
    );
    assert_eq!(forwarder.next_wake(t0), Some(Duration::ZERO));

    let pumped = forwarder
        .pump(&src, t0)
        .await
        .expect("the flush sends without waiting");
    assert_eq!(pumped.outcome, SendOutcome::Sent);
    assert!(forwarder.take_flush_done());
    assert!(!forwarder.take_flush_done());
}

#[tokio::test]
async fn a_forbidden_chunk_loses_its_events() {
    let server = MockServer::start().await;
    mount_mint(&server).await;
    mount_chunk(&server, 403).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let src = source(&server.uri());
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![record("client.launcher.adoption", Some("step"))],
        t0,
    );

    let pumped = forwarder
        .pump(&src, t0 + Duration::from_secs(2))
        .await
        .expect("the event is due at the coalescing point");
    assert_eq!(pumped.outcome, SendOutcome::Dropped);
    assert_eq!(pumped.lost_events, 1);
    assert_eq!(forwarder.queued(), 0);
}

#[test]
fn consent_changed_moves_the_epoch_forward() {
    // Other engine tests bump the same global, so only the direction is pinned.
    let before = consent_epoch();
    consent_changed();
    let after = consent_epoch();
    assert!(after > before);
}

#[tokio::test]
async fn the_epoch_is_read_before_the_events_are_built() {
    let server = MockServer::start().await;
    mount_no_post(&server).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let mut src = source(&server.uri());
    src.bump_on_events = true;
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![record("client.launcher.adoption", Some("step"))],
        t0,
    );
    assert!(forwarder
        .pump(&src, t0 + Duration::from_secs(2))
        .await
        .is_none());
    assert_eq!(forwarder.queued(), 0);
}

#[tokio::test]
async fn a_flush_trigger_on_an_empty_queue_is_not_urgent() {
    let server = MockServer::start().await;
    mount_no_post(&server).await;
    let mut forwarder = Forwarder::new(LauncherSender::new());
    let mut src = source(&server.uri());
    src.opted_in = false;
    let t0 = Instant::now();
    forwarder.on_records(
        &src,
        vec![record("client.launcher.launch", Some("launch_finished"))],
        t0,
    );
    src.opted_in = true;
    forwarder.on_records(
        &src,
        vec![record("client.launcher.adoption", Some("step"))],
        t0,
    );
    assert_eq!(forwarder.next_wake(t0), Some(Duration::from_secs(2)));
    assert!(forwarder.pump(&src, t0).await.is_none());
}
