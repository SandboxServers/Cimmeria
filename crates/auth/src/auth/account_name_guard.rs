//! Regression guard: a malformed `AccountName` must be rejected before it
//! reaches the request span, a log event or the login-audit stream.
//!
//! The name is client-controlled, and since #1289 decodes XML character
//! references, so a newline can arrive as `&#10;` as well as raw. Before the
//! fix, `handle_user_auth` recorded the name into the span right after
//! parsing and stored it in the `PlaintextRequiresTls` audit row, both before
//! the format check ran. Moving the check back behind either use trips this.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::Router;
use tokio::sync::broadcast;
use tower::ServiceExt; // oneshot

use super::handlers::handle_user_auth;
use super::{HandlerState, ShardInfo};
use crate::audit::{LoginEvent, LoginEventBuffer};
use crate::test_support::LogCapture;

/// Marker that appears only inside the malformed account name.
const MARKER: &str = "forgedline";

fn state_with_audit() -> (
    Arc<HandlerState>,
    LoginEventBuffer,
    broadcast::Receiver<LoginEvent>,
) {
    let (tx, rx) = broadcast::channel(16);
    let buffer = LoginEventBuffer::new();
    let state = Arc::new(HandlerState {
        shards: vec![ShardInfo {
            name: "TestShard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        }],
        sessions: Arc::new(Mutex::new(HashMap::new())),
        pending_logins: Arc::new(Mutex::new(HashMap::new())),
        developer_mode: true,
        db: None,
        login_tx: Some(tx),
        login_buffer: Some(buffer.clone()),
    });
    (state, buffer, rx)
}

/// POST a Phase 1 request over the plain-HTTP router shape (no TLS marker),
/// with a plaintext password so the `PlaintextRequiresTls` audit path is the
/// one a too-late name check would reach.
async fn post_plaintext_over_http(state: Arc<HandlerState>, raw_account_name: &str) -> String {
    let app = Router::new()
        .route("/SGWLogin/UserAuth", post(handle_user_auth))
        .with_state(state);
    let body = format!(
        r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="{raw_account_name}" Password="my-plaintext-password" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#
    );
    let request = Request::builder()
        .method("POST")
        .uri("/SGWLogin/UserAuth")
        .header("Content-Type", "text/xml")
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:5555".parse::<SocketAddr>().unwrap(),
        ))
        .body(Body::from(body))
        .unwrap();
    let response = app.oneshot(request).await.expect("request");
    assert_eq!(response.status(), StatusCode::OK, "auth always returns 200");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn malformed_account_name_never_reaches_span_log_or_audit() {
    let capture = LogCapture::install();

    // ── Malformed: a decoded newline (and a decoded tab) in the name ────
    for raw in [format!("evil&#10;{MARKER}"), format!("evil&#9;{MARKER}")] {
        let (state, buffer, mut rx) = state_with_audit();
        let xml = post_plaintext_over_http(state, &raw).await;
        assert!(
            xml.contains("The specified account name is invalid."),
            "a malformed account name must be refused by the name check, got: {xml}"
        );
        assert!(
            buffer.snapshot().is_empty(),
            "a malformed account name must not reach the audit buffer ({raw:?})"
        );
        assert!(
            rx.try_recv().is_err(),
            "a malformed account name must not be broadcast as an audit event ({raw:?})"
        );
    }

    for event in capture.all() {
        let leaked = event.message.as_deref().is_some_and(|m| m.contains(MARKER))
            || event.fields.values().any(|v| v.contains(MARKER));
        assert!(
            !leaked,
            "unvalidated account name reached a {} event on target `{}`: {event:?}",
            event.level, event.target
        );
    }
    let rejected = capture
        .find_event(
            tracing::Level::INFO,
            "account name fails the format check",
            "malformed_account_name",
        )
        .expect("the rejection must log reason=malformed_account_name");
    assert!(
        rejected.fields.contains_key("account_name_len"),
        "the rejection logs the name's length, not the name"
    );

    // ── Positive control: a valid name still reaches span and audit ─────
    // Proves the capture and the audit buffer are live, so the empty results
    // above come from the guard, not from a dead harness.
    let (state, buffer, _rx) = state_with_audit();
    let xml = post_plaintext_over_http(state, "valid-name").await;
    assert!(
        xml.contains("The specified password is invalid."),
        "a valid name with plaintext over HTTP hits the TLS gate, got: {xml}"
    );
    let events = buffer.snapshot();
    assert_eq!(events.len(), 1, "the TLS-gate rejection is audited once");
    assert_eq!(events[0].account_name, "valid-name");
    assert!(
        capture.span_recorded("account_name", "valid-name"),
        "a validated name is recorded into the span"
    );
}
