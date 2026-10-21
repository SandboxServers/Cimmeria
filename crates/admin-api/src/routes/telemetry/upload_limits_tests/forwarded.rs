//! The per-address keys behind a loopback proxy (C18): the forwarded caller's
//! own slot and quota, and the address that never reaches a log row.

use std::net::IpAddr;
use std::time::Instant;

use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, HeaderValue};

use crate::routes::telemetry::chunk::chunk_inner;
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::replay_tests::{capture_async, Row};
use crate::routes::telemetry::upload_gate::{
    admit, Admitted, Route, UploadLimits, UploadPolicy, UploadState, Uploader,
};

use super::{chunk_request, launcher_token, run, small_chunk, token, Env, PEER};

/// One `admit` from `peer`, as the handler makes it: the player bearer token for
/// `sid` and, when given, an `X-Forwarded-For` header.
pub(super) fn admit_from(
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
pub(super) fn admit_with_token(
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
pub(super) fn busy_row(rows: &[Row]) -> &Row {
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
