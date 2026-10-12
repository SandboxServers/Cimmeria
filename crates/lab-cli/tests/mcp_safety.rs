//! The MCP and status clients against a private stub: a timeout reads as a
//! timeout to the uat runner, and a redirect is never followed (the bearer
//! token must not leave the address the daemon file named).
#![allow(dead_code)]

#[path = "mcp/stub.rs"]
mod stub;

use std::time::Duration;

use cimmeria_lab_cli::commands::uat::record::{error_text, is_timeout};
use cimmeria_lab_cli::daemon::mcp::{McpClient, McpError};
use cimmeria_lab_cli::daemon::status::{fetch_value_with_timeout, Endpoint};
use serde_json::json;
use stub::{route, Route, Stub};

const TOKEN: &str = "tok-123";
const INIT_REPLY: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"stub","version":"1"}}}"#;
const CALL_REPLY: &str =
    r#"{"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":"{}"}]}}"#;

fn handshake() -> Vec<Route> {
    let mut init = route(
        "POST",
        "/mcp",
        Some(r#""method":"initialize""#),
        200,
        INIT_REPLY,
    );
    init.headers = vec![("Mcp-Session-Id", "sess-1")];
    vec![
        init,
        route(
            "POST",
            "/mcp",
            Some(r#""method":"notifications/initialized""#),
            202,
            "",
        ),
    ]
}

fn client_with(call: Route) -> (Stub, McpClient) {
    let mut routes = handshake();
    routes.push(call);
    let stub = Stub::start(routes);
    let client = McpClient::open(&stub.base_url(), Some(TOKEN)).expect("open");
    (stub, client)
}

/// What the uat runner does with a failed call: store `error_text`, and wait
/// for the lease only when `is_timeout` says it was a timeout.
fn assert_reads_as_timeout(result: Result<serde_json::Value, McpError>) {
    let error = result.expect_err("the call must time out");
    assert!(matches!(error, McpError::Unreachable(_)), "{error:?}");
    let text = error_text(&error);
    assert!(is_timeout(&text), "not read as a timeout: {text}");
    assert!(text.contains("timed out after"), "{text}");
}

#[test]
fn a_reply_that_never_starts_reads_as_a_timeout_to_the_uat_runner() {
    let mut slow = route(
        "POST",
        "/mcp",
        Some(r#""method":"tools/call""#),
        200,
        CALL_REPLY,
    );
    slow.delay_ms = 1500;
    let (_stub, mut client) = client_with(slow);
    assert_reads_as_timeout(client.call_tool("t", json!({}), Duration::from_millis(200)));
}

#[test]
fn a_body_that_stalls_reads_as_a_timeout_to_the_uat_runner() {
    let mut stalled = route(
        "POST",
        "/mcp",
        Some(r#""method":"tools/call""#),
        200,
        CALL_REPLY,
    );
    stalled.stall_ms = 1500;
    let (_stub, mut client) = client_with(stalled);
    assert_reads_as_timeout(client.call_tool("t", json!({}), Duration::from_millis(200)));
}

#[test]
fn a_refused_connection_does_not_read_as_a_timeout() {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let error = McpClient::open(&format!("http://127.0.0.1:{port}"), None)
        .err()
        .expect("nothing listens there");
    assert!(!is_timeout(&error_text(&error)), "{error}");
}

#[test]
fn mcp_does_not_follow_a_redirect() {
    let elsewhere = Stub::start(vec![route("POST", "/mcp", None, 200, INIT_REPLY)]);
    let mut moved = route("POST", "/mcp", None, 307, "");
    let target = format!("{}/mcp", elsewhere.base_url());
    moved.headers = vec![("Location", Box::leak(target.into_boxed_str()))];
    let stub = Stub::start(vec![moved]);
    match McpClient::open(&stub.base_url(), Some(TOKEN)) {
        Err(McpError::Http(307, _)) => {}
        other => panic!("expected Http(307), got {:?}", other.err()),
    }
    assert!(
        elsewhere.requests().is_empty(),
        "the redirect target was asked: {:?}",
        elsewhere.requests()
    );
}

#[test]
fn status_does_not_follow_a_redirect() {
    let body = r#"{"daemon":{"pid":1,"version":"1"},"instances":[]}"#;
    let elsewhere = Stub::start(vec![route("GET", "/status", None, 200, body)]);
    let mut moved = route("GET", "/status", None, 302, "");
    let target = format!("{}/status", elsewhere.base_url());
    moved.headers = vec![("Location", Box::leak(target.into_boxed_str()))];
    let stub = Stub::start(vec![moved]);
    let ep = Endpoint {
        base_url: stub.base_url(),
        token: Some(TOKEN.to_owned()),
    };
    assert!(fetch_value_with_timeout(&ep, Duration::from_secs(5)).is_none());
    assert!(
        elsewhere.requests().is_empty(),
        "the redirect target was asked: {:?}",
        elsewhere.requests()
    );
}
