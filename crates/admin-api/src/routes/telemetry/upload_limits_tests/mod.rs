//! Upload size and rate limits on the chunk and bundle routes: the gate
//! runs before the body is read, each budget refuses at its first hit, the
//! priority allowance holds, client strings are capped, and refusals are
//! logged through a throttle.
//!
//! - `gate` — kill switch, token before body, rate limits, slots.
//! - `chunk` — the chunk budgets and the priority allowance.
//! - `bundle` — the bundle budgets.
//! - `field_caps` — the string caps.
//! - `refusal_log` — the refusal throttle.
//! - `slow_body` — token before body (bundle) and the body deadline, over a socket.

mod bundle;
mod chunk;
mod field_caps;
mod gate;
mod refusal_log;
mod slow_body;

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::MutexGuard;

use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::routes::dev_session::{encode_token, env_lock, TokenClaims};

const ENV_SECRET: &str = "CIMMERIA_TELEMETRY_HMAC_SECRET";
const ENV_KILL_SWITCH: &str = "CIMMERIA_TELEMETRY_KILL_SWITCH";

/// The peer every in-process request comes from.
pub(super) const PEER: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 7));

/// Holds the crate's env lock with a usable HMAC secret and the kill
/// switch off; restores both on drop. Tests that hold one are plain
/// `#[test]`s that own a runtime, so the guard never spans an `.await`.
pub(super) struct Env {
    _lock: MutexGuard<'static, ()>,
    prev_secret: Option<String>,
    prev_kill: Option<String>,
}

impl Env {
    pub(super) fn install() -> Self {
        let lock = env_lock().lock().unwrap_or_else(|p| p.into_inner());
        let prev_secret = std::env::var(ENV_SECRET).ok();
        let prev_kill = std::env::var(ENV_KILL_SWITCH).ok();
        std::env::set_var(ENV_SECRET, "5c".repeat(64));
        std::env::remove_var(ENV_KILL_SWITCH);
        Self {
            _lock: lock,
            prev_secret,
            prev_kill,
        }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, prev) in [
            (ENV_SECRET, self.prev_secret.take()),
            (ENV_KILL_SWITCH, self.prev_kill.take()),
        ] {
            match prev {
                Some(v) => std::env::set_var(name, v),
                None => std::env::remove_var(name),
            }
        }
    }
}

pub(super) fn run<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

pub(super) fn claims(sid: &str) -> TokenClaims {
    let now = chrono::Utc::now().timestamp();
    TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-limits-test".into(),
        sid: sid.into(),
        iat: now,
        exp: now + 3600,
        scope: vec!["telemetry.write".into()],
        kind: None,
    }
}

/// A token the `Env` secret verifies.
pub(super) fn token(sid: &str) -> String {
    encode_token(&claims(sid), &[0x5c; 64]).unwrap()
}

pub(super) fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(bytes).unwrap();
    enc.finish().unwrap()
}

pub(super) fn chunk_request(sid: &str, body: Vec<u8>) -> Request {
    Request::builder()
        .method("POST")
        .uri("/upload-chunk")
        .header(AUTHORIZATION, format!("Bearer {}", token(sid)))
        .body(Body::from(body))
        .unwrap()
}

/// A valid gzip(NDJSON) chunk of `rows` info-level client log rows.
pub(super) fn small_chunk(rows: u64) -> Vec<u8> {
    let lines: Vec<String> = (0..rows)
        .map(|seq| {
            format!(
                r#"{{"type":"client_log","ts_ms":1,"seq":{seq},"source_file":"a.log","level":"info","category":"raw","message":"m"}}"#
            )
        })
        .collect();
    gzip(lines.join("\n").as_bytes())
}

pub(super) fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in entries {
        zw.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zw.write_all(data).unwrap();
    }
    zw.finish().unwrap().into_inner()
}

/// The launcher's bundle request: a metadata part and a zip part.
pub(super) fn bundle_request(sid: &str, zip: &[u8]) -> Request {
    let mut body = Vec::new();
    body.extend_from_slice(
        b"--B\r\nContent-Disposition: form-data; name=\"metadata\"\r\n\r\n{\"session_id\":\"s\"}\r\n",
    );
    body.extend_from_slice(
        b"--B\r\nContent-Disposition: form-data; name=\"zip\"; filename=\"s.zip\"\r\n\
          Content-Type: application/zip\r\n\r\n",
    );
    body.extend_from_slice(zip);
    body.extend_from_slice(b"\r\n--B--\r\n");
    Request::builder()
        .method("POST")
        .uri("/upload-bundle")
        .header(AUTHORIZATION, format!("Bearer {}", token(sid)))
        .header(CONTENT_TYPE, "multipart/form-data; boundary=B")
        .body(Body::from(body))
        .unwrap()
}
