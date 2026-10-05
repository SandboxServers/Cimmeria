//! The route on a real listener: it is mounted on the admin router, it
//! takes the launcher's request with no token, and its body limit is
//! 64 KiB to the byte. These are the only tests here that open a socket;
//! the login-port mount is checked beside its merge, in `login_port.rs`.
//!
//! They go through the process-wide ingest state and read the environment
//! without the env lock. Under nextest each test has a process of its own
//! and gets the handler's first answer. Under `cargo test` another test may
//! be holding the kill switch on (503), or the tests in this process may
//! together have used up loopback's allowance (429); either still comes
//! from the handler, which is what these tests are about.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use cimmeria_services::orchestrator::Orchestrator;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::super::{launcher_summary_routes, MAX_SUMMARY_BODY_BYTES};
use super::{batch, element};

const PATH: &str = "/api/telemetry/launcher-summary";

/// Serve `app` on an ephemeral loopback port with connect info, as both
/// production listeners do.
async fn serve(app: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    addr
}

/// Send one request over raw HTTP/1.1 and return the status code and the
/// response body. `content_type` is the only header that is not framing:
/// no request here carries an `Authorization`.
async fn send(
    addr: SocketAddr,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    body: &[u8],
) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let content_type = content_type
        .map(|value| format!("Content-Type: {value}\r\n"))
        .unwrap_or_default();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n{content_type}\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    // A server that has already refused the request may close before the
    // last bytes are written; the status line is what matters.
    let _ = stream.write_all(&request).await;
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    let response = String::from_utf8_lossy(&response);
    let status = response
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in response to {method} {path}: {response:?}"));
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    (status, body)
}

/// The status of a JSON request whose body is `body_len` spaces.
async fn status(addr: SocketAddr, method: &str, path: &str, body_len: usize) -> u16 {
    let body = vec![b' '; body_len];
    send(addr, method, path, Some("application/json"), &body)
        .await
        .0
}

/// True if another test in this process got in the way: see the module
/// comment. Only the handler answers 503 or 429 here.
fn paused_or_over_quota(status: u16) -> bool {
    status == 503 || status == 429
}

/// A body of spaces is not the envelope, so the handler's own answer to it
/// is a 400.
fn handler_ran(status: u16) -> bool {
    status == 400 || paused_or_over_quota(status)
}

/// The admin router serves the summary route under `/api/telemetry`,
/// beside the chunk upload. The orchestrator is only constructed, never
/// started: the telemetry handlers read no router state.
///
/// Dropping the merge in `routes::api_routes` makes the first status a 404.
#[tokio::test]
async fn the_admin_router_answers_the_summary_route() {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    let app = Router::new()
        .nest("/api", crate::routes::api_routes())
        .with_state(orchestrator);
    let addr = serve(app).await;

    let summary = status(addr, "POST", PATH, 0).await;
    assert!(handler_ran(summary), "launcher-summary: {summary}");
    assert_eq!(status(addr, "GET", PATH, 0).await, 405);
    // Control: the upload route it is merged beside still answers, and
    // still wants its token; a path nobody mounted is a 404.
    assert_eq!(
        status(addr, "POST", "/api/telemetry/upload-chunk", 0).await,
        401
    );
    assert_eq!(
        status(addr, "POST", "/api/telemetry/launcher-summaries", 0).await,
        404
    );
}

/// **End to end, no token.** The launcher's request (a JSON body and a
/// `Content-Type`, nothing else) is a 200 with its verdict over a real
/// socket, and the same body sent as `text/plain` or with no content type
/// is the handler's 415. This is the one place the axum handler itself is
/// shown to pass the request's real headers and peer to the ingest.
#[tokio::test]
async fn an_anonymous_request_is_served_over_a_socket() {
    let app = Router::new().nest("/api/telemetry", launcher_summary_routes());
    let addr = serve(app).await;
    let body = batch(vec![element(0x50c)]).to_string().into_bytes();

    for media_type in [Some("text/plain"), None] {
        let (status, text) = send(addr, "POST", PATH, media_type, &body).await;
        if !paused_or_over_quota(status) {
            assert_eq!(status, 415, "{media_type:?}: {text}");
            assert_eq!(text, "Content-Type must be application/json");
        }
    }
    let (status, text) = send(addr, "POST", PATH, Some("application/json"), &body).await;
    if !paused_or_over_quota(status) {
        assert_eq!(status, 200, "{text}");
        assert_eq!(text, r#"{"results":["accepted"]}"#);
    }
}

/// A body of exactly 64 KiB reaches the handler (which refuses it as not
/// JSON); one byte more is a 413 from the body limit before the handler
/// runs. The limit travels with `launcher_summary_routes`, so the router
/// here is that alone, nested as both listeners nest it.
#[tokio::test]
async fn the_body_limit_is_64_kib_to_the_byte() {
    assert_eq!(MAX_SUMMARY_BODY_BYTES, 65_536);
    let app = Router::new().nest("/api/telemetry", launcher_summary_routes());
    let addr = serve(app).await;

    let at_limit = status(addr, "POST", PATH, MAX_SUMMARY_BODY_BYTES).await;
    assert!(handler_ran(at_limit), "64 KiB: {at_limit}");
    let over = status(addr, "POST", PATH, MAX_SUMMARY_BODY_BYTES + 1).await;
    assert_eq!(over, 413, "64 KiB + 1");
}
