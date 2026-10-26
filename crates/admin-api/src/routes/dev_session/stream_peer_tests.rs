//! The stream's address (D-LTL4, amending D-LT19): the client telemetry
//! registry keys a mint's or refresh's stream on the forwarded caller behind
//! a loopback proxy, not on the proxy's loopback peer, so world entry's
//! address join finds the Deck's stream. The address is held in memory only;
//! the log rows carry two booleans and no address.

use std::time::Instant;

use axum::http::HeaderMap;
use cimmeria_services::auth::client_telemetry::{client_telemetry_for, ClientTelemetry};

use super::handlers::{mint_with_headers, refresh_with_headers, DevSessionRequest, Tables};
use super::tests::{capture, ip, policy, request, EnvGuard, NOW_UNIX};

const INSTALL: &str = "3f2504e0-4f89-41d3-9a0c-0305e82c3301";

// The stream registry is process-wide: each test uses its own machine ids and
// its own addresses (192.0.2.0/24, which no other test streams from).

fn request_for(machine_id: &str) -> DevSessionRequest {
    DevSessionRequest {
        machine_id: machine_id.to_string(),
        ..request(INSTALL)
    }
}

/// One `X-Forwarded-For` header naming `caller`, as the board proxy sets it.
fn forwarded(caller: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("x-forwarded-for", caller.parse().unwrap());
    headers
}

/// The stream's `session_id` when a lookup at `at` finds exactly that session.
fn present_at(at: &str, now_unix: i64) -> Option<String> {
    match client_telemetry_for(None, Some(ip(at)), now_unix) {
        ClientTelemetry::Present { session_id, .. } => Some(session_id),
        _ => None,
    }
}

// A mint from the board proxy's loopback peer, forwarding `192.0.2.41`, records
// the stream at the caller's address; the loopback address names no stream.
#[test]
fn a_mint_behind_a_loopback_proxy_records_the_stream_at_the_forwarded_caller() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let resp = mint_with_headers(
        &t,
        &policy(10, 10, 10),
        ip("127.0.0.1"),
        &forwarded("192.0.2.41"),
        request_for("stream-peer-mint-1"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    assert_eq!(
        present_at("192.0.2.41", NOW_UNIX).as_deref(),
        Some(resp.session_id.as_str())
    );
    assert_ne!(
        client_telemetry_for(None, Some(ip("127.0.0.1")), NOW_UNIX).session_id(),
        Some(resp.session_id.as_str()),
        "the proxy's loopback peer must not hold the stream"
    );
}

// A refresh that reaches the server through the proxy moves the stream to the
// forwarded caller it came from, the launcher's latest address.
#[test]
fn a_refresh_behind_a_loopback_proxy_records_the_stream_at_the_forwarded_caller() {
    let _g = EnvGuard::install();
    let t = Tables::new();
    let p = policy(10, 10, 10);
    let minted = mint_with_headers(
        &t,
        &p,
        ip("192.0.2.42"),
        &HeaderMap::new(),
        request_for("stream-peer-refresh-1"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    let refreshed = refresh_with_headers(
        &t,
        &p,
        ip("127.0.0.1"),
        &forwarded("192.0.2.43"),
        &minted.token,
        Instant::now(),
        NOW_UNIX + 60,
    )
    .unwrap();
    assert_eq!(refreshed.session_id, minted.session_id);
    assert_eq!(
        present_at("192.0.2.43", NOW_UNIX + 60).as_deref(),
        Some(minted.session_id.as_str())
    );
}

// Spoof guard: a remote peer cannot name its stream's address with a header.
#[test]
fn a_remote_peer_cannot_move_its_stream_with_a_forwarded_header() {
    let _g = EnvGuard::install();
    let resp = mint_with_headers(
        &Tables::new(),
        &policy(10, 10, 10),
        ip("192.0.2.44"),
        &forwarded("192.0.2.45"),
        request_for("stream-peer-spoof-1"),
        Instant::now(),
        NOW_UNIX,
    )
    .unwrap();
    assert_eq!(
        present_at("192.0.2.44", NOW_UNIX).as_deref(),
        Some(resp.session_id.as_str())
    );
    assert_eq!(
        client_telemetry_for(None, Some(ip("192.0.2.45")), NOW_UNIX).session_id(),
        None,
        "the forged address must hold no stream"
    );
}

// Fallbacks: without a usable header a loopback peer is its own address. The
// loopback address is shared with other tests in this process, so the check
// accepts a stream of ours or an `ambiguous` count; the mint must not be absent.
#[test]
fn a_loopback_peer_without_a_usable_header_records_at_loopback() {
    let _g = EnvGuard::install();
    for headers in [HeaderMap::new(), forwarded("not-an-ip")] {
        let resp = mint_with_headers(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("127.0.0.1"),
            &headers,
            request_for("stream-peer-fallback-1"),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap();
        let found = client_telemetry_for(None, Some(ip("127.0.0.1")), NOW_UNIX);
        assert!(
            matches!(&found, ClientTelemetry::Ambiguous { .. })
                || found.session_id() == Some(resp.session_id.as_str()),
            "the stream must sit at loopback, got {}",
            found.as_str()
        );
    }
}

// Log fields: two booleans, and no row names the forwarded address.
#[test]
fn a_mint_logs_booleans_for_the_proxy_hop_and_no_forwarded_address() {
    let _g = EnvGuard::install();
    let (_, rows) = capture(|| {
        mint_with_headers(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("127.0.0.1"),
            &forwarded("192.0.2.46"),
            request_for("stream-peer-log-1"),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap()
    });
    let minted = rows
        .iter()
        .find(|(_, f)| f.contains_key("caller_forwarded"))
        .expect("the mint INFO row");
    assert_eq!(minted.1["caller_forwarded"], "true");
    assert_eq!(minted.1["peer_loopback"], "true");
    let text = format!("{rows:?}");
    assert!(!text.contains("192.0.2"), "an address in logs: {text}");

    let (_, rows) = capture(|| {
        mint_with_headers(
            &Tables::new(),
            &policy(10, 10, 10),
            ip("192.0.2.47"),
            &HeaderMap::new(),
            request_for("stream-peer-log-2"),
            Instant::now(),
            NOW_UNIX,
        )
        .unwrap()
    });
    let minted = rows
        .iter()
        .find(|(_, f)| f.contains_key("caller_forwarded"))
        .expect("the mint INFO row");
    assert_eq!(minted.1["caller_forwarded"], "false");
    assert_eq!(minted.1["peer_loopback"], "false");
}
