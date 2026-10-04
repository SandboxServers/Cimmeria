//! The route on a real listener: it is mounted on the admin router, and
//! its body limit is 64 KiB to the byte. These are the only tests here that
//! open a socket; the login-port mount is checked beside its merge, in
//! `login_port.rs`.
//!
//! They go through the process-wide ingest state and read the environment
//! without the env lock, so they assert only what holds whatever another
//! test has set: with no `Authorization` the handler answers 401, or 503
//! while a kill-switch test is running.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use cimmeria_services::orchestrator::Orchestrator;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::super::{launcher_summary_routes, MAX_SUMMARY_BODY_BYTES};

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

/// Send one request with a body of `body_len` bytes over raw HTTP/1.1 and
/// return the status code.
async fn status(addr: SocketAddr, method: &str, path: &str, body_len: usize) -> u16 {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Content-Length: {body_len}\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    request.resize(request.len() + body_len, b' ');
    // A server that has already refused the request may close before the
    // last bytes are written; the status line is what matters.
    let _ = stream.write_all(&request).await;
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    let response = String::from_utf8_lossy(&response);
    response
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in response to {method} {path}: {response:?}"))
}

fn handler_ran(status: u16) -> bool {
    status == 401 || status == 503
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
    // Control: the upload route it is merged beside still answers, and a
    // path nobody mounted is a 404.
    assert_eq!(
        status(addr, "POST", "/api/telemetry/upload-chunk", 0).await,
        401
    );
    assert_eq!(
        status(addr, "POST", "/api/telemetry/launcher-summaries", 0).await,
        404
    );
}

/// A body of exactly 64 KiB reaches the handler (which refuses it for its
/// missing token); one byte more is a 413 from the body limit before the
/// handler runs. The limit travels with `launcher_summary_routes`, so the
/// router here is that alone, nested as both listeners nest it.
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
