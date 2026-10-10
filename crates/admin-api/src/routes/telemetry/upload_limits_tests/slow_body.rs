//! Requests whose body never arrives, over a real socket: the bundle route
//! checks the token first, and a body that takes too long is refused
//! instead of holding an upload slot.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request};
use axum::routing::post;
use axum::{Json, Router};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::routes::telemetry;
use crate::routes::telemetry::bundle::bundle_inner;
use crate::routes::telemetry::chunk::chunk_inner;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};

use super::{run, token, Env};

/// Serve `app` on loopback with connect info, send `head` (the request
/// line and headers, with whatever part of the body it includes) and
/// return the response head, or `None` if none came within `wait`.
async fn answer_to(
    app: Router,
    head: impl Fn(SocketAddr) -> String,
    wait: Duration,
) -> Option<String> {
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
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(head(addr).as_bytes()).await.unwrap();
    let mut response = Vec::new();
    let mut buf = [0u8; 1024];
    tokio::time::timeout(wait, async {
        while !response.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buf).await {
                Ok(n) if n > 0 => response.extend_from_slice(&buf[..n]),
                _ => break,
            }
        }
    })
    .await
    .ok()?;
    Some(String::from_utf8_lossy(&response).into_owned())
}

fn status_of(response: &str) -> &str {
    response.lines().next().unwrap_or_default()
}

/// The two routes over a state of the test's own, with a 1 s body
/// deadline. The state is leaked: the router needs it for `'static`.
fn short_deadline_routes() -> Router {
    let state: &'static UploadState = Box::leak(Box::new(UploadState::new(UploadLimits {
        body_timeout: Duration::from_secs(1),
        ..UploadLimits::default()
    })));
    Router::new()
        .route(
            "/upload-chunk",
            post(
                move |ConnectInfo(peer): ConnectInfo<SocketAddr>, req: Request| async move {
                    chunk_inner(
                        state,
                        &UploadPolicy::defaults(),
                        peer.ip(),
                        req,
                        Instant::now(),
                    )
                    .await
                    .map(Json)
                },
            ),
        )
        .route(
            "/upload-bundle",
            post(
                move |ConnectInfo(peer): ConnectInfo<SocketAddr>, req: Request| async move {
                    bundle_inner(
                        state,
                        &UploadPolicy::defaults(),
                        peer.ip(),
                        req,
                        Instant::now(),
                    )
                    .await
                    .map(Json)
                },
            ),
        )
}

/// **A bundle with no token is answered before its body arrives**, like a
/// chunk: the request declares a 1 MiB multipart body and sends none of
/// it, and the 401 comes back anyway.
#[test]
fn a_bundle_without_a_token_is_answered_before_its_body_arrives() {
    let _env = Env::install();
    let response = run(answer_to(
        Router::new().nest("/api/telemetry", telemetry::routes()),
        |addr| {
            format!(
                "POST /api/telemetry/upload-bundle HTTP/1.1\r\nHost: {addr}\r\n\
                 Content-Type: multipart/form-data; boundary=B\r\n\
                 Content-Length: 1048576\r\n\r\n"
            )
        },
        Duration::from_secs(20),
    ))
    .expect("no answer while the body was outstanding");
    assert!(status_of(&response).contains(" 401 "), "{response:?}");
}

/// **A chunk body that does not arrive in time is refused** (400,
/// `body_read_failed`) instead of holding its slot. The request has a
/// valid token, declares 100 bytes and sends none.
#[test]
fn a_chunk_body_that_never_arrives_is_refused_at_the_deadline() {
    let _env = Env::install();
    let response = run(answer_to(
        short_deadline_routes(),
        |addr| {
            format!(
                "POST /upload-chunk HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {}\r\n\
                 Content-Length: 100\r\n\r\n",
                token("sess-slow-chunk")
            )
        },
        Duration::from_secs(15),
    ))
    .expect("the body deadline never fired");
    assert!(status_of(&response).contains(" 400 "), "{response:?}");
}

/// The same for a bundle whose multipart body stops after its first
/// boundary.
#[test]
fn a_bundle_body_that_stalls_is_refused_at_the_deadline() {
    let _env = Env::install();
    let response = run(answer_to(
        short_deadline_routes(),
        |addr| {
            format!(
                "POST /upload-bundle HTTP/1.1\r\nHost: {addr}\r\nAuthorization: Bearer {}\r\n\
                 Content-Type: multipart/form-data; boundary=B\r\n\
                 Content-Length: 4096\r\n\r\n--B\r\n",
                token("sess-slow-bundle")
            )
        },
        Duration::from_secs(15),
    ))
    .expect("the body deadline never fired");
    assert!(status_of(&response).contains(" 400 "), "{response:?}");
}
