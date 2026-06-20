use std::io::Read;

use axum::http::HeaderMap;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;

use crate::routes::dev_session::TokenClaims;

use super::dto::{ClientNativeEvent, IngestError, TelemetryEvent};
use super::handlers::{replay_client_native, verify_bearer};

fn gzip_lines(lines: &[&str]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    for l in lines {
        enc.write_all(l.as_bytes()).unwrap();
        enc.write_all(b"\n").unwrap();
    }
    enc.finish().unwrap()
}

/// Pin the wire-format contract: launcher chunks are gzip(NDJSON)
/// with one `TelemetryEvent` per line. The server must parse the
/// launcher's exact serialization shape — any deserialization
/// drift here is a wire-break with the launcher crate.
#[test]
fn telemetry_event_deserializes_launcher_json_shape() {
    let raw = r#"{"type":"client_log","ts_ms":1700000000000,"seq":42,"source_file":"x.log","level":"info","category":"raw","message":"hello"}"#;
    let ev: TelemetryEvent = serde_json::from_str(raw).unwrap();
    match ev {
        TelemetryEvent::ClientLog(e) => {
            assert_eq!(e.seq, 42);
            assert_eq!(e.message, "hello");
            assert!(e.packet_no.is_none());
        }
        other => panic!("expected ClientLog, got {other:?}"),
    }
}

/// gzip+NDJSON round-trip — the chunk decode path must handle the
/// same encoding the launcher emits.
#[test]
fn ndjson_gzip_decodes_multiple_lines() {
    let payload = gzip_lines(&[
        r#"{"type":"client_log","ts_ms":1,"seq":1,"source_file":"a","level":"info","category":"raw","message":"one"}"#,
        r#"{"type":"client_log","ts_ms":2,"seq":2,"source_file":"a","level":"info","category":"raw","message":"two"}"#,
    ]);
    let mut decoder = GzDecoder::new(&payload[..]);
    let mut ndjson = String::new();
    decoder.read_to_string(&mut ndjson).unwrap();
    let lines: Vec<&str> = ndjson.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in lines {
        let _: TelemetryEvent = serde_json::from_str(line).unwrap();
    }
}

/// Bearer extraction: leading "Bearer " + token. Empty token
/// (Bearer with nothing after) must surface as MissingAuth, not
/// as a downstream BadPayload — clearer ops triage.
#[test]
fn verify_bearer_rejects_empty_token() {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_static("Bearer "),
    );
    let err = verify_bearer(&headers).unwrap_err();
    assert!(matches!(err, IngestError::MissingAuth));
}

/// No Authorization header → MissingAuth (401). This is the
/// launcher's signal to mint a fresh token if one was never set.
#[test]
fn verify_bearer_rejects_missing_header() {
    let headers = HeaderMap::new();
    let err = verify_bearer(&headers).unwrap_err();
    assert!(matches!(err, IngestError::MissingAuth));
}

/// The injected-DLL event shape (`type = "client_native"`)
/// must deserialize on the server using the exact JSON layout
/// the launcher's `ClientNativeEvent` struct produces. If the
/// two re-declarations drift, this test fails loudly — the
/// wire contract between launcher and admin-api is the only
/// thing that keeps the independent enums in sync.
#[test]
fn client_native_event_matches_launcher_shape() {
    let raw = r#"{
        "type": "client_native",
        "ts_ms": 1700000000000,
        "seq": 42,
        "target": "client.frame_tick",
        "level": "info",
        "fields": { "frame_no": 99, "stall_ms": 1.5 }
    }"#;
    let ev: TelemetryEvent = serde_json::from_str(raw).unwrap();
    match ev {
        TelemetryEvent::ClientNative(e) => {
            assert_eq!(e.seq, 42);
            assert_eq!(e.target, "client.frame_tick");
            assert_eq!(e.level, "info");
            assert_eq!(e.fields["frame_no"], 99);
            assert_eq!(e.fields["stall_ms"], 1.5);
        }
        other => panic!("expected ClientNative, got {other:?}"),
    }
}

/// `fields` is optional on the wire — an empty hook with no
/// payload (e.g. a coarse "FEngineLoop::Tick fired" sample)
/// should round-trip without forcing a `"fields": {}` clause
/// every time.
#[test]
fn client_native_event_accepts_missing_fields() {
    let raw = r#"{
        "type": "client_native",
        "ts_ms": 0,
        "seq": 0,
        "target": "client.frame_tick",
        "level": "trace"
    }"#;
    let ev: TelemetryEvent = serde_json::from_str(raw).unwrap();
    match ev {
        TelemetryEvent::ClientNative(e) => assert!(e.fields.is_empty()),
        other => panic!("expected ClientNative, got {other:?}"),
    }
}

/// Replay must not panic on an unknown level string — falls
/// through to info. This is the load-bearing forward-compat
/// guarantee: a future DLL version that emits a new level the
/// server doesn't know about must not crash the ingest path.
#[test]
fn replay_client_native_tolerates_unknown_level() {
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-1".into(),
        sid: "session-1".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
    };
    let ev = ClientNativeEvent {
        ts_ms: 0,
        seq: 0,
        target: "client.weird".into(),
        level: "verbose-but-unknown".into(),
        fields: serde_json::Map::new(),
    };
    // Just must not panic.
    replay_client_native(&claims, ev);
}

/// Verify-bearer round-trip via the shared `dev_session::load_secret`.
/// Uses the process-wide lock from `dev_session::env_lock()` so this
/// test can run alongside `dev_session::tests` without one stomping
/// the env var the other is reading.
#[test]
fn verify_bearer_accepts_token_minted_by_dev_session() {
    use crate::routes::dev_session::{encode_token, env_lock, TokenClaims};

    let _g = env_lock().lock().unwrap_or_else(|p| p.into_inner());
    let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
    // 64-byte secret in hex form.
    std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", "a".repeat(128));

    // Mint a token using the dev_session encoder, then verify
    // through the telemetry verify_bearer path. If the loaders
    // ever drift, this test breaks loud.
    let secret = vec![0xaau8; 64];
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-1".into(),
        sid: "session-1".into(),
        iat: chrono::Utc::now().timestamp(),
        exp: chrono::Utc::now().timestamp() + 3600,
        scope: vec!["telemetry.write".into()],
    };
    let token = encode_token(&claims, &secret).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        axum::http::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    let verified = verify_bearer(&headers).expect("token from dev_session must verify");
    assert_eq!(verified.sid, "session-1");

    match prev {
        Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
        None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
    }
}
