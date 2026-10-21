use super::events::*;
use super::tests::{event, mount_mint, open, opted_in, CHUNK};
use super::*;
use crate::test_capture::{capture, Captured};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Every row the transport forwarded: the `launcher.telemetry` target only.
fn forwarded(captured: &[Captured]) -> Vec<&Captured> {
    captured
        .iter()
        .filter(|c| c.target == "launcher.telemetry")
        .collect()
}

/// Runs `future` on a fresh current-thread runtime, on this thread, so the
/// capture (thread-local) sees its events.
fn block_on<F: std::future::Future<Output = ()>>(future: F) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future);
}

#[test]
fn reason_maps_each_variant_to_its_closed_code() {
    let cases = [
        (TransportError::NotOptedIn, "not_opted_in"),
        (TransportError::Choice, "choice_unreadable"),
        (TransportError::SessionUnavailable, "session_unavailable"),
        (TransportError::EndpointRefused, "endpoint_refused"),
        (TransportError::Queue, "queue"),
        (TransportError::TokenRejected, "token_rejected"),
        (TransportError::BackingOff, "backing_off"),
        (TransportError::Upload, "upload"),
    ];
    for (code, want) in cases {
        assert_eq!(reason(code), want);
    }
}

#[test]
fn http_status_reads_the_status_the_server_sent() {
    // One row per status source; rustfmt would split each literal over five lines.
    #[rustfmt::skip]
    let cases = [
        (TelemetryError::Chunk(ChunkError::Status { status: 500, body: "MARKER".into() }), Some(500)),
        (TelemetryError::Chunk(ChunkError::Rejected { status: 413, body: "MARKER".into() }), Some(413)),
        (TelemetryError::Chunk(ChunkError::TokenRejected), Some(401)),
        (TelemetryError::Chunk(ChunkError::RateLimited { retry_after_secs: 1 }), Some(429)),
        (TelemetryError::Chunk(ChunkError::KillSwitch { retry_after_secs: 1 }), Some(503)),
        (TelemetryError::Auth(AuthError::Status { status: 500, body: "MARKER".into() }), Some(500)),
        (TelemetryError::Auth(AuthError::KillSwitch { retry_after_secs: 1 }), Some(503)),
        (TelemetryError::Auth(AuthError::ReplyTooLarge), None),
        (TelemetryError::Bundle(BundleError::Status { status: 502, body: "MARKER".into() }), Some(502)),
        (TelemetryError::Bundle(BundleError::TokenRejected), Some(401)),
        (TelemetryError::Bundle(BundleError::KillSwitch { retry_after_secs: 1 }), Some(503)),
        (TelemetryError::Bundle(BundleError::Empty), None),
    ];
    for (error, want) in cases {
        assert_eq!(http_status(&error), want);
    }
}

#[test]
fn a_streak_warns_once_then_debugs_and_resumes_once() {
    // Callsite interest: this dispatcher stops other tests' callsites from
    // being cached as disabled (see `transport/tests.rs:711-716`).
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let streak = Streak::default();
    let error = TelemetryError::Chunk(ChunkError::Status {
        status: 500,
        body: "MARKER".into(),
    });
    let ((), captured) = capture(|| {
        chunk_upload_failed(&streak, &error);
        chunk_upload_failed(&streak, &error);
        chunk_upload_resumed(&streak);
        chunk_upload_resumed(&streak);
    });
    let rows = forwarded(&captured);
    assert_eq!(rows.len(), 3, "{captured:?}");
    assert_eq!(rows[0].level, tracing::Level::WARN);
    assert_eq!(rows[0].field("event"), Some("chunk_upload_failed"));
    assert_eq!(rows[0].field("attempt"), Some("1"));
    assert_eq!(rows[0].field("http_status"), Some("500"));
    assert_eq!(rows[0].field("reason"), Some("upload"));
    assert_eq!(rows[1].level, tracing::Level::DEBUG);
    assert_eq!(rows[1].field("attempt"), Some("2"));
    assert_eq!(rows[2].level, tracing::Level::INFO);
    assert_eq!(rows[2].field("event"), Some("chunk_upload_resumed"));
    assert_eq!(rows[2].field("count"), Some("2"));
}

#[test]
fn the_other_helpers_log_their_event_and_codes() {
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let bundle_error = TelemetryError::Bundle(BundleError::Status {
        status: 502,
        body: "MARKER".into(),
    });
    let ((), captured) = capture(|| {
        token_renew_started();
        token_refresh_failed("remint", TransportError::SessionUnavailable, None);
        token_refresh_failed("refresh", TransportError::BackingOff, Some(503));
        proactive_refresh_failed(&Streak::default(), TransportError::Upload, None);
        session_reminted();
        bundle_upload_failed(&bundle_error);
        queue_dropped(7);
        dropped_counter_reset_failed();
        queue_discarded();
        queue_discard_failed("choice_changed");
    });
    let rows = forwarded(&captured);
    let events: Vec<_> = rows.iter().map(|r| r.field("event").unwrap()).collect();
    assert_eq!(
        events,
        [
            "token_renew_started",
            "token_refresh_failed",
            "token_refresh_failed",
            "token_refresh_failed",
            "session_reminted",
            "bundle_upload_failed",
            "queue_dropped",
            "dropped_counter_reset_failed",
            "queue_discarded",
            "queue_discard_failed",
        ]
    );
    assert_eq!(rows[0].level, tracing::Level::INFO);
    assert_eq!(rows[0].field("reason"), Some("token_rejected"));
    // No status: the key is absent, not an empty value.
    assert_eq!(rows[1].field("action"), Some("remint"));
    assert_eq!(rows[1].field("reason"), Some("session_unavailable"));
    assert_eq!(rows[1].field("http_status"), None);
    assert_eq!(rows[1].field("held"), Some("true"));
    assert_eq!(rows[2].level, tracing::Level::WARN);
    assert_eq!(rows[2].field("http_status"), Some("503"));
    assert_eq!(rows[2].field("reason"), Some("backing_off"));
    assert_eq!(rows[3].field("action"), Some("refresh_proactive"));
    assert_eq!(rows[3].field("held"), Some("false"));
    assert_eq!(rows[3].field("attempt"), Some("1"));
    assert_eq!(rows[5].field("http_status"), Some("502"));
    assert_eq!(rows[6].field("count"), Some("7"));
    assert_eq!(rows[7].field("reason"), Some("queue"));
    assert_eq!(rows[8].level, tracing::Level::INFO);
    assert_eq!(rows[9].field("action"), Some("choice_changed"));
    for row in &rows {
        assert!(row.fields.values().all(|v| !v.contains("MARKER")));
    }
}

#[test]
fn a_flush_the_server_refuses_then_takes_logs_the_streak() {
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let ((), captured) = capture(|| {
        block_on(async {
            let server = MockServer::start().await;
            mount_mint(&server, "tok-1").await;
            Mock::given(method("POST"))
                .and(path(CHUNK))
                .respond_with(ResponseTemplate::new(500).set_body_string("MARKER"))
                .up_to_n_times(2)
                .mount(&server)
                .await;
            Mock::given(method("POST"))
                .and(path(CHUNK))
                .respond_with(ResponseTemplate::new(200))
                .mount(&server)
                .await;

            let state = opted_in(root.path());
            let transport = open(&state, &server, install.path()).await;
            transport.enqueue(event("one")).await.unwrap();
            assert_eq!(transport.flush().await, Err(TransportError::Upload));
            assert_eq!(transport.flush().await, Err(TransportError::Upload));
            assert_eq!(transport.flush().await, Ok(1));
        });
    });
    let rows = forwarded(&captured);
    assert_eq!(rows.len(), 3, "{captured:?}");
    assert_eq!(rows[0].level, tracing::Level::WARN);
    assert_eq!(rows[0].field("event"), Some("chunk_upload_failed"));
    assert_eq!(rows[0].field("http_status"), Some("500"));
    assert_eq!(rows[0].field("reason"), Some("upload"));
    assert_eq!(rows[0].field("attempt"), Some("1"));
    assert_eq!(rows[1].level, tracing::Level::DEBUG);
    assert_eq!(rows[1].field("attempt"), Some("2"));
    assert_eq!(rows[2].level, tracing::Level::INFO);
    assert_eq!(rows[2].field("event"), Some("chunk_upload_resumed"));
    assert_eq!(rows[2].field("count"), Some("2"));
    for row in &captured {
        assert!(row.fields.values().all(|v| !v.contains("MARKER")));
    }
}

#[test]
fn a_bundle_the_server_refuses_logs_one_warn() {
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let root = tempfile::tempdir().unwrap();
    let install = tempfile::tempdir().unwrap();
    let sessions =
        cimmeria_launcher_core::install_layout::binaries_dir(install.path()).join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    std::fs::write(sessions.join("a.log"), "a session log\n").unwrap();
    let ((), captured) = capture(|| {
        block_on(async {
            let server = MockServer::start().await;
            mount_mint(&server, "tok-1").await;
            Mock::given(method("POST"))
                .and(path("/api/telemetry/upload-bundle"))
                .respond_with(ResponseTemplate::new(500).set_body_string("MARKER"))
                .mount(&server)
                .await;

            let state = opted_in(root.path());
            let transport = open(&state, &server, install.path()).await;
            assert_eq!(
                transport.upload_bundle(1).await.err(),
                Some(TransportError::Upload)
            );
        });
    });
    let rows = forwarded(&captured);
    assert_eq!(rows.len(), 1, "{captured:?}");
    assert_eq!(rows[0].level, tracing::Level::WARN);
    assert_eq!(rows[0].field("event"), Some("bundle_upload_failed"));
    assert_eq!(rows[0].field("http_status"), Some("500"));
    assert_eq!(rows[0].field("reason"), Some("upload"));
    assert!(rows[0].fields.values().all(|v| !v.contains("MARKER")));
}
