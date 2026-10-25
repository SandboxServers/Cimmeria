use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::game_telemetry::Identity;
use crate::test_capture::capture;
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const UPLOAD_ID: &str = "0123456789abcdef";

/// Answers every post as accepted and reports a fixed endpoint, as a sender would.
struct EndpointSink(Option<(String, String)>);

impl ChunkSink for EndpointSink {
    fn post(&mut self, _batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        std::future::ready(BatchOutcome::Posted)
    }

    fn endpoint(&self) -> Option<(String, String)> {
        self.0.clone()
    }
}

/// Refuses every post, and reports a fixed endpoint, as a sender would.
struct FailingSink(Option<(String, String)>);

impl ChunkSink for FailingSink {
    fn post(&mut self, _batch: &[TelemetryEvent]) -> impl Future<Output = BatchOutcome> + Send {
        std::future::ready(BatchOutcome::Failed)
    }

    fn endpoint(&self) -> Option<(String, String)> {
        self.0.clone()
    }
}

/// One upload of a single small row through `sink`; returns the outcome and the captured events.
fn upload_one<S: ChunkSink + Send>(
    sink: &mut S,
) -> (IngestOutcome, Vec<crate::test_capture::Captured>) {
    let lines = [LogLine {
        file: "launcher.log".into(),
        line_no: 1,
        end: 100,
        ts_ms: None,
        level: None,
        target: None,
        message: "line".into(),
        truncated: false,
    }];
    let events = rows(&lines, LAUNCHER_LOG_TARGET, UPLOAD_ID, "v", "win");
    let mut pacer = TokioPacer::new();
    // A second live dispatcher keeps the callsite interest from caching "never".
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    capture(|| {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(send_with(
                sink,
                &events,
                &mut pacer,
                &CancellationToken::new(),
                |_| {},
            ))
    })
}

#[test]
fn the_finish_line_names_the_scheme_and_host_of_the_last_post() {
    let mut sink = EndpointSink(Some(("https".into(), "play.example".into())));
    let (out, logged) = upload_one(&mut sink);
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 1 });
    let finished = logged
        .iter()
        .find(|e| e.field("event") == Some("log_upload_finished"))
        .expect("a finish event");
    assert_eq!(finished.field("scheme"), Some("https"));
    assert_eq!(finished.field("host"), Some("play.example"));
}

#[test]
fn the_finish_line_says_dash_when_no_endpoint_is_known() {
    let mut sink = EndpointSink(None);
    let (out, logged) = upload_one(&mut sink);
    assert_eq!(out, IngestOutcome::Sent { lines_sent: 1 });
    let finished = logged
        .iter()
        .find(|e| e.field("event") == Some("log_upload_finished"))
        .expect("a finish event");
    assert_eq!(finished.field("scheme"), Some("-"));
    assert_eq!(finished.field("host"), Some("-"));
}

#[test]
fn a_failed_finish_line_names_the_scheme_and_host_too() {
    let mut sink = FailingSink(Some(("https".into(), "play.example".into())));
    let (out, logged) = upload_one(&mut sink);
    assert_eq!(out, IngestOutcome::Failed);
    let finished = logged
        .iter()
        .find(|e| {
            e.field("event") == Some("log_upload_finished") && e.field("outcome") == Some("failed")
        })
        .expect("a failed finish event");
    assert_eq!(finished.field("scheme"), Some("https"));
    assert_eq!(finished.field("host"), Some("play.example"));
}

#[tokio::test]
async fn the_real_sink_reports_the_endpoint_its_sender_posted_to() {
    let server = MockServer::start().await;
    let expires_at_ms = chrono::Utc::now().timestamp_millis() + 3_600_000;
    Mock::given(method("POST"))
        .and(path("/api/auth/dev-session"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "session_id": "session-1", "token": "payload.sig", "expires_at_ms": expires_at_ms,
            "upload_endpoint": format!("{}/api/telemetry", server.uri()),
            "chunk_max_bytes": 1_048_576, "flush_interval_ms": 2_000,
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/telemetry/upload-chunk"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let sender = LauncherSender::logs();
    let target = LauncherTarget {
        identity: Identity {
            install_id: Uuid::from_u128(7),
            machine_id: "0123456789abcdef".into(),
        },
        servers: vec![LoginServer {
            name: "fixture".into(),
            url: server.uri(),
        }],
    };
    let lines = [LogLine {
        file: "launcher.log".into(),
        line_no: 1,
        end: 100,
        ts_ms: None,
        level: None,
        target: None,
        message: "line".into(),
        truncated: false,
    }];
    let events = rows(&lines, LAUNCHER_LOG_TARGET, UPLOAD_ID, "v", "win");
    let mut sink = SenderSink {
        sender: &sender,
        target: &target,
    };
    assert_eq!(sink.endpoint(), None);
    assert_eq!(sink.post(&events).await, BatchOutcome::Posted);
    assert_eq!(
        sink.endpoint(),
        Some(("http".to_owned(), "127.0.0.1".to_owned()))
    );
}
