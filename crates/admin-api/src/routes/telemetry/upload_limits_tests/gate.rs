//! The gate: the token before the body, the kill switch, the rate limits
//! and the concurrency slots.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use axum::http::header::RETRY_AFTER;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::routes::dev_session::AuthError;
use crate::routes::telemetry;
use crate::routes::telemetry::chunk::chunk_inner;
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};

use super::{chunk_request, run, small_chunk, Env, PEER};

/// **A chunk with no token is answered before its body arrives.** The
/// request declares a 1 MiB body and sends none of it. The 401 comes back
/// anyway: the token was checked before a byte of the body was read. A
/// handler that buffered the body first (the `Bytes` extractor it used to
/// take) would wait for it, and this test would end at its timeout.
#[test]
fn a_chunk_without_a_token_is_answered_before_its_body_arrives() {
    let _env = Env::install();
    let response = run(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app: Router = Router::new().nest("/api/telemetry", telemetry::routes());
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let head = format!(
            "POST /api/telemetry/upload-chunk HTTP/1.1\r\nHost: {addr}\r\n\
             Content-Length: 1048576\r\n\r\n"
        );
        stream.write_all(head.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        let mut buf = [0u8; 1024];
        let answered = tokio::time::timeout(Duration::from_secs(20), async {
            while !response.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf).await {
                    Ok(n) if n > 0 => response.extend_from_slice(&buf[..n]),
                    _ => break,
                }
            }
        })
        .await;
        assert!(answered.is_ok(), "no answer while the body was outstanding");
        String::from_utf8_lossy(&response).into_owned()
    });
    let status_line = response.lines().next().unwrap_or_default();
    assert!(status_line.contains(" 401 "), "{response:?}");
}

/// The kill switch stops uploads made with tokens issued before it was
/// thrown, not only new mints.
#[test]
fn the_kill_switch_refuses_uploads_from_issued_tokens() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        kill_switch: true,
        ..UploadPolicy::defaults()
    };
    let result = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-kill", small_chunk(1)),
        Instant::now(),
    ));
    let err = result.unwrap_err();
    assert!(
        matches!(err, IngestError::Auth(AuthError::KillSwitchActive)),
        "{err:?}"
    );
    assert_eq!(
        err.into_response().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

/// The happy path through the gate and the budgets: a small chunk is
/// replayed in full.
#[test]
fn a_small_chunk_is_accepted() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let counts = run(chunk_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        chunk_request("sess-limits-happy", small_chunk(3)),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((counts.accepted, counts.parsed_lines), (3, 3));
}

/// **The concurrency limit refuses.** With every chunk slot held, a chunk
/// is refused with 503 and a `Retry-After`; once the slot is free the same
/// chunk goes through.
#[test]
fn a_chunk_is_refused_while_every_slot_is_busy() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        chunk_slots: 1,
        ..UploadLimits::default()
    });
    let policy = UploadPolicy::defaults();
    let held = state.hold_chunk_slot();
    let err = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-busy", small_chunk(1)),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(matches!(err, IngestError::Busy), "{err:?}");
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(response.headers()[RETRY_AFTER], "5");

    drop(held);
    run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-busy", small_chunk(1)),
        Instant::now(),
    ))
    .expect("a free slot admits the chunk");
}

/// The per-session rate limit answers 429 with a `Retry-After`.
#[test]
fn a_session_over_its_upload_rate_is_refused() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        chunk_per_session: 2,
        ..UploadPolicy::defaults()
    };
    let now = Instant::now();
    for _ in 0..2 {
        run(chunk_inner(
            &state,
            &policy,
            PEER,
            chunk_request("sess-rate", small_chunk(1)),
            now,
        ))
        .unwrap();
    }
    let err = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-rate", small_chunk(1)),
        now,
    ))
    .unwrap_err();
    assert_eq!(err.reason(), "rate_limited");
    let response = err.into_response();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(response.headers().contains_key(RETRY_AFTER));
}

/// The per-address rate limit holds across sessions: a caller cannot
/// sidestep it by minting more tokens.
#[test]
fn an_address_over_its_upload_rate_is_refused_whatever_the_session() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        chunk_per_session: 0,
        chunk_per_ip: 2,
        ..UploadPolicy::defaults()
    };
    let now = Instant::now();
    for sid in ["sess-ip-a", "sess-ip-b"] {
        run(chunk_inner(
            &state,
            &policy,
            PEER,
            chunk_request(sid, small_chunk(1)),
            now,
        ))
        .unwrap();
    }
    let err = run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-ip-c", small_chunk(1)),
        now,
    ))
    .unwrap_err();
    assert_eq!(err.reason(), "rate_limited");
}
