//! The gate: the token before the body, the kill switch, the rate limits
//! and the concurrency slots.

use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use axum::http::header::{AUTHORIZATION, RETRY_AFTER};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::routes::dev_session::AuthError;
use crate::routes::telemetry;
use crate::routes::telemetry::chunk::chunk_inner;
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::replay_tests::{capture_async, Row};
use crate::routes::telemetry::upload_gate::{
    admit, Admitted, Route, UploadLimits, UploadPolicy, UploadState, Uploader,
};

use super::{chunk_request, launcher_token, run, small_chunk, token, Env, PEER};

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
    let (result, rows) = run(capture_async(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-busy", small_chunk(1)),
        Instant::now(),
    )));
    let err = result.unwrap_err();
    assert!(matches!(err, IngestError::Busy { .. }), "{err:?}");
    assert_eq!(busy_row(&rows).fields["pool"], "route");
    assert_eq!(
        busy_row(&rows).level,
        tracing::Level::INFO,
        "busy is back-pressure, not a fault"
    );
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

/// **A refusal that is not busy still logs at WARN.** A session over its
/// upload rate is refused `rate_limited`, and that row stays a `warn`: only
/// `busy` drops to INFO.
#[test]
fn a_non_busy_refusal_still_logs_at_warn() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        chunk_per_session: 1,
        ..UploadPolicy::defaults()
    };
    let now = Instant::now();
    let (result, rows) = run(capture_async(async {
        chunk_inner(
            &state,
            &policy,
            PEER,
            chunk_request("sess-warn", small_chunk(1)),
            now,
        )
        .await
        .unwrap();
        chunk_inner(
            &state,
            &policy,
            PEER,
            chunk_request("sess-warn", small_chunk(1)),
            now,
        )
        .await
    }));
    let err = result.unwrap_err();
    assert_eq!(err.reason(), "rate_limited");
    let row = rows
        .iter()
        .find(|r| r.fields.get("reason").map(String::as_str) == Some("rate_limited"))
        .expect("a rate_limited refusal is logged");
    assert_eq!(row.level, tracing::Level::WARN, "{row:?}");
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

/// One `admit` from `peer`, as the handler makes it: the player bearer token for
/// `sid` and, when given, an `X-Forwarded-For` header.
fn admit_from(
    state: &UploadState,
    policy: &UploadPolicy,
    route: Route,
    peer: IpAddr,
    forwarded: Option<&str>,
    sid: &str,
) -> Result<Admitted, IngestError> {
    admit_with_token(state, policy, route, peer, forwarded, &token(sid))
}

/// One `admit` from `peer` with the bearer `token` given, and when given an
/// `X-Forwarded-For` header. A launcher caller passes its own token here.
fn admit_with_token(
    state: &UploadState,
    policy: &UploadPolicy,
    route: Route,
    peer: IpAddr,
    forwarded: Option<&str>,
    token: &str,
) -> Result<Admitted, IngestError> {
    let mut headers = HeaderMap::new();
    let bearer = format!("Bearer {token}");
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&bearer).unwrap());
    if let Some(forwarded) = forwarded {
        headers.insert("x-forwarded-for", HeaderValue::from_str(forwarded).unwrap());
    }
    let mut who = Uploader::anonymous(peer);
    admit(state, policy, route, &headers, &mut who, Instant::now())
}

/// Behind the board proxy (C18) a loopback peer's `X-Forwarded-For` names the
/// caller, so three callers behind one proxy address each pass the address
/// quota and each hold a peer slot at once, on both routes. Reverted, all
/// three share the proxy's slot and the second is refused `Busy`.
#[test]
fn behind_a_loopback_proxy_each_forwarded_caller_holds_its_own_slot() {
    let _env = Env::install();
    let loopback: IpAddr = "127.0.0.1".parse().unwrap();
    for route in [Route::Chunk, Route::Bundle] {
        // The route-wide pool stays out of the way: only the peer slot is
        // under test.
        let state = UploadState::new(UploadLimits {
            chunk_slots: 8,
            bundle_slots: 8,
            ..UploadLimits::default()
        });
        let policy = UploadPolicy {
            chunk_per_session: 0,
            chunk_per_ip: 2,
            ..UploadPolicy::defaults()
        };
        let callers = ["198.51.100.7", "198.51.100.8", "198.51.100.9"];
        let mut held = Vec::new();
        for (sid, caller) in ["sess-xff-a", "sess-xff-b", "sess-xff-c"]
            .iter()
            .zip(callers)
        {
            let admitted = admit_from(&state, &policy, route, loopback, Some(caller), sid);
            held.push(admitted.expect("a distinct caller is admitted"));
        }
        assert_eq!(held.len(), 3, "{route:?}");
    }
}

/// A remote peer cannot choose its caller: its rotating `X-Forwarded-For` is
/// ignored, so the third request from it is refused by the address quota, on
/// both the chunk and the bundle route.
#[test]
fn a_remote_peer_cannot_rotate_past_the_address_quota() {
    let _env = Env::install();
    for route in [Route::Chunk, Route::Bundle] {
        let state = UploadState::new(UploadLimits::default());
        let policy = match route {
            Route::Chunk => UploadPolicy {
                chunk_per_session: 0,
                chunk_per_ip: 2,
                ..UploadPolicy::defaults()
            },
            Route::Bundle => UploadPolicy {
                bundle_per_session: 0,
                bundle_per_ip: 2,
                ..UploadPolicy::defaults()
            },
        };
        for (sid, caller) in ["sess-spoof-a", "sess-spoof-b"]
            .iter()
            .zip(["198.51.100.7", "198.51.100.8"])
        {
            let admitted = admit_from(&state, &policy, route, PEER, Some(caller), sid);
            admitted.expect("within the address quota");
        }
        let err = admit_from(
            &state,
            &policy,
            route,
            PEER,
            Some("198.51.100.9"),
            "sess-spoof-c",
        )
        .unwrap_err();
        assert_eq!(err.reason(), "rate_limited", "{route:?}");
    }
}

/// A launcher chunk is keyed on the forwarded caller too (C18): behind a
/// loopback proxy two callers each pass the launcher address quota, and a
/// remote peer rotating its header is refused by it.
#[test]
fn a_launcher_chunk_keys_its_address_quota_on_the_forwarded_caller() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits {
        launcher_chunk_slots: 8,
        ..UploadLimits::default()
    });
    let policy = UploadPolicy {
        launcher_chunk_per_session: 0,
        launcher_chunk_per_install: 0,
        launcher_chunk_per_ip: 1,
        ..UploadPolicy::defaults()
    };
    let loopback: IpAddr = "127.0.0.1".parse().unwrap();
    for (sid, caller) in ["sess-lx-a", "sess-lx-b"]
        .iter()
        .zip(["198.51.100.7", "198.51.100.8"])
    {
        let token = launcher_token(sid, "install-launcher-test");
        let admitted = admit_with_token(
            &state,
            &policy,
            Route::Chunk,
            loopback,
            Some(caller),
            &token,
        );
        admitted.expect("a distinct forwarded caller is admitted");
    }

    let spoof = launcher_token("sess-lx-c", "install-launcher-test");
    admit_with_token(
        &state,
        &policy,
        Route::Chunk,
        PEER,
        Some("198.51.100.9"),
        &spoof,
    )
    .expect("the remote peer's first chunk is within the address quota");
    let spoof = launcher_token("sess-lx-d", "install-launcher-test");
    let err = admit_with_token(
        &state,
        &policy,
        Route::Chunk,
        PEER,
        Some("198.51.100.10"),
        &spoof,
    )
    .unwrap_err();
    assert_eq!(err.reason(), "rate_limited");
}

/// The refusal row for a busy upload: the one `reason = "busy"` row logged.
fn busy_row(rows: &[Row]) -> &Row {
    rows.iter()
        .find(|r| r.fields.get("reason").map(String::as_str) == Some("busy"))
        .expect("a busy refusal is logged")
}

/// **One address holds at most one chunk slot.** While `PEER` has a chunk
/// in flight its next one is refused as busy, another address's goes
/// through, and once the first is done `PEER` is admitted again.
#[test]
fn an_address_holds_at_most_one_chunk_slot() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy::defaults();
    let other = std::net::IpAddr::V4(std::net::Ipv4Addr::new(198, 51, 100, 9));
    let held = state.hold_peer_slot(Route::Chunk, PEER);
    let (result, rows) = run(capture_async(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-peer-a", small_chunk(1)),
        Instant::now(),
    )));
    let err = result.unwrap_err();
    assert!(matches!(err, IngestError::Busy { .. }), "{err:?}");
    assert_eq!(busy_row(&rows).fields["pool"], "peer_share");
    assert_eq!(
        busy_row(&rows).level,
        tracing::Level::INFO,
        "busy is back-pressure, not a fault"
    );
    run(chunk_inner(
        &state,
        &policy,
        other,
        chunk_request("sess-peer-b", small_chunk(1)),
        Instant::now(),
    ))
    .expect("another address has its own share");

    drop(held);
    run(chunk_inner(
        &state,
        &policy,
        PEER,
        chunk_request("sess-peer-a", small_chunk(1)),
        Instant::now(),
    ))
    .expect("the address's slot is free again");
    assert_eq!(
        state.peers_in_flight(),
        0,
        "finished uploads release their share"
    );
}

/// Directive 6 (D-LT19), the chunk leg: behind a loopback proxy that forwarded
/// `198.51.100.7`, a chunk that is admitted and one refused on the address
/// quota leave rows, and no row names the forwarded address.
#[test]
fn the_forwarded_address_is_in_no_chunk_row() {
    const FORWARDED: &str = "198.51.100.7";
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let policy = UploadPolicy {
        chunk_per_session: 0,
        chunk_per_ip: 1,
        ..UploadPolicy::defaults()
    };
    let loopback: IpAddr = "127.0.0.1".parse().unwrap();
    let forwarded_chunk = |sid: &str| {
        let mut req = chunk_request(sid, small_chunk(1));
        req.headers_mut()
            .insert("x-forwarded-for", HeaderValue::from_static(FORWARDED));
        req
    };
    let (first, mut rows) = run(capture_async(chunk_inner(
        &state,
        &policy,
        loopback,
        forwarded_chunk("sess-leak-a"),
        Instant::now(),
    )));
    first.expect("the first chunk is admitted");
    let (second, refused_rows) = run(capture_async(chunk_inner(
        &state,
        &policy,
        loopback,
        forwarded_chunk("sess-leak-b"),
        Instant::now(),
    )));
    let err = second.unwrap_err();
    assert_eq!(err.reason(), "rate_limited");
    rows.extend(refused_rows);
    assert!(!rows.is_empty(), "the refused chunk should leave a row");
    for row in &rows {
        assert!(
            !row.target.contains(FORWARDED),
            "a row target names the forwarded address: {}",
            row.target
        );
        for (key, value) in &row.fields {
            assert!(
                !key.contains(FORWARDED) && !value.contains(FORWARDED),
                "a chunk row carries the forwarded address: {key}={value}"
            );
        }
    }
}
