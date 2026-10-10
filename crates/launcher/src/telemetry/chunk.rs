//! Gzipped NDJSON chunk POST.
//!
//! `<upload_endpoint>/upload-chunk` — one POST carries N events
//! (newline-delimited JSON), gzipped, with the bearer token from
//! the dev-session auth. Idempotent by `(session_id, seq_first, seq_last)`
//! on the server side so a retried POST is a no-op.

use std::io::Write;

use flate2::write::GzEncoder;
use flate2::Compression;
use thiserror::Error;

use super::events::TelemetryEvent;

#[derive(Debug, Error)]
pub enum ChunkError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Server returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("Token rejected (401) — needs refresh")]
    TokenRejected,
    #[error("Kill switch active (503) — retry after {retry_after_secs}s")]
    KillSwitch { retry_after_secs: u64 },
    #[error("Rate limited (429) — retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    /// A 4xx other than 401, 408 and 429 (413 too large, 400 not gzip,
    /// ...): the server will never take this chunk. The caller drops it
    /// rather than re-queueing it, or it would be refused on every retry.
    #[error("Chunk rejected ({status}): {body}")]
    Rejected { status: u16, body: String },
}

impl ChunkError {
    /// The wait a 429 or 503 asked for, if it was one.
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            ChunkError::KillSwitch { retry_after_secs }
            | ChunkError::RateLimited { retry_after_secs } => Some(*retry_after_secs),
            _ => None,
        }
    }
}

/// Rows per chunk. The server replays up to 10,000 rows of one chunk; the
/// launcher stays well under it so a chunk is never cut there.
pub const MAX_CHUNK_ROWS: usize = 1_000;

/// Split `events` into consecutive batches of at most `max_rows` rows and
/// at most `max_bytes` of NDJSON each (the mint's `chunk_max_bytes`, before
/// gzip). An event larger than `max_bytes` on its own goes alone. Returns
/// index ranges into `events`, in order.
pub fn split_batches(
    events: &[TelemetryEvent],
    max_bytes: u64,
    max_rows: usize,
) -> Result<Vec<std::ops::Range<usize>>, ChunkError> {
    let max_rows = max_rows.max(1);
    let mut batches = Vec::new();
    let (mut start, mut bytes) = (0usize, 0u64);
    for (i, ev) in events.iter().enumerate() {
        let len = serde_json::to_vec(ev)?.len() as u64 + 1;
        let rows = i - start;
        if rows > 0 && (rows >= max_rows || bytes + len > max_bytes) {
            batches.push(start..i);
            start = i;
            bytes = 0;
        }
        bytes += len;
    }
    if start < events.len() {
        batches.push(start..events.len());
    }
    Ok(batches)
}

/// Serialize events to NDJSON, gzip, return the compressed bytes.
/// Exposed so the bundle path can reuse the same compression shape.
pub fn encode_ndjson_gzip(events: &[TelemetryEvent]) -> Result<Vec<u8>, ChunkError> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    for ev in events {
        let line = serde_json::to_vec(ev)?;
        enc.write_all(&line)?;
        enc.write_all(b"\n")?;
    }
    Ok(enc.finish()?)
}

/// POST one chunk to `<upload_endpoint>/upload-chunk`. Empty event
/// slice short-circuits to Ok(()) without a network call so the
/// uploader's "drain and POST" loop has no surprising side effects
/// when the queue was empty.
pub async fn post_chunk(
    http: &reqwest::Client,
    upload_endpoint: &str,
    token: &str,
    events: &[TelemetryEvent],
) -> Result<(), ChunkError> {
    if events.is_empty() {
        return Ok(());
    }
    let body = encode_ndjson_gzip(events)?;
    let url = join_endpoint(upload_endpoint, "upload-chunk");
    let resp = http
        .post(&url)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/x-ndjson")
        .header(reqwest::header::CONTENT_ENCODING, "gzip")
        .body(body)
        .send()
        .await?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    if status.as_u16() == 401 {
        return Err(ChunkError::TokenRejected);
    }
    if status.as_u16() == 503 {
        let retry_after = parse_retry_after(&resp);
        return Err(ChunkError::KillSwitch {
            retry_after_secs: retry_after,
        });
    }
    if status.as_u16() == 429 {
        return Err(ChunkError::RateLimited {
            retry_after_secs: parse_retry_after(&resp),
        });
    }
    let body = resp.text().await.unwrap_or_default();
    // 401 and 429 are handled above; 408 is a timeout worth retrying.
    // Every other 4xx says this chunk itself is unacceptable.
    if status.is_client_error() && status.as_u16() != 408 {
        return Err(ChunkError::Rejected {
            status: status.as_u16(),
            body,
        });
    }
    Err(ChunkError::Status {
        status: status.as_u16(),
        body,
    })
}

/// Join `<endpoint>` and `<path>` with exactly one `/` between them,
/// preserving any query string on `endpoint` (rare but supported).
fn join_endpoint(endpoint: &str, path: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    format!("{trimmed}/{path}")
}

fn parse_retry_after(resp: &reqwest::Response) -> u64 {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::events::{ClientLogEvent, TelemetryEvent};
    use std::io::Read;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ev(seq: u64) -> TelemetryEvent {
        TelemetryEvent::ClientLog(ClientLogEvent {
            ts_ms: seq as i64,
            seq,
            source_file: "x.log".into(),
            level: "info".into(),
            category: "Mercury".into(),
            packet_no: Some(seq),
            message: format!("event-{seq}"),
        })
    }

    // Gzip roundtrip pinned: encode → decode produces the exact
    // newline-delimited JSON the server expects. A regression that
    // forgot the trailing newline would surface here.
    #[test]
    fn encode_ndjson_gzip_roundtrips_to_n_newline_delimited_lines() {
        let events = vec![ev(0), ev(1), ev(2)];
        let gz = encode_ndjson_gzip(&events).unwrap();
        let mut dec = flate2::read::GzDecoder::new(gz.as_slice());
        let mut out = String::new();
        dec.read_to_string(&mut out).unwrap();
        let lines: Vec<&str> = out.split('\n').filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 3);
        for (line, ev) in lines.iter().zip(events.iter()) {
            let back: TelemetryEvent = serde_json::from_str(line).unwrap();
            assert_eq!(&back, ev);
        }
    }

    #[test]
    fn encode_ndjson_gzip_empty_input_produces_valid_gzip() {
        let gz = encode_ndjson_gzip(&[]).unwrap();
        let mut dec = flate2::read::GzDecoder::new(gz.as_slice());
        let mut out = String::new();
        dec.read_to_string(&mut out).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn join_endpoint_collapses_trailing_slash() {
        assert_eq!(
            join_endpoint("https://x.example/api/", "upload-chunk"),
            "https://x.example/api/upload-chunk"
        );
        assert_eq!(
            join_endpoint("https://x.example/api", "upload-chunk"),
            "https://x.example/api/upload-chunk"
        );
    }

    // Empty event slice short-circuits — verified by the lack of any
    // mock-server expectation in this test. If we accidentally fire a
    // request, the test would fail with a connection error to the
    // bare hostname.
    #[tokio::test]
    async fn post_chunk_empty_events_skips_network() {
        let http = reqwest::Client::new();
        post_chunk(&http, "https://no.such.host.invalid./api", "token", &[])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn post_chunk_sends_gzip_with_bearer_and_content_type() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-chunk"))
            .and(header("authorization", "Bearer my-token"))
            .and(header("content-type", "application/x-ndjson"))
            .and(header("content-encoding", "gzip"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        post_chunk(
            &http,
            &format!("{}/api", server.uri()),
            "my-token",
            &[ev(0)],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn post_chunk_401_surfaces_token_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-chunk"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = post_chunk(&http, &format!("{}/api", server.uri()), "t", &[ev(0)])
            .await
            .unwrap_err();
        assert!(matches!(err, ChunkError::TokenRejected));
    }

    // 503 carries the kill-switch shape with Retry-After honored.
    #[tokio::test]
    async fn post_chunk_503_surfaces_kill_switch_with_retry_after() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-chunk"))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "120"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = post_chunk(&http, &format!("{}/api", server.uri()), "t", &[ev(0)])
            .await
            .unwrap_err();
        match err {
            ChunkError::KillSwitch { retry_after_secs } => assert_eq!(retry_after_secs, 120),
            other => panic!("expected KillSwitch, got {other:?}"),
        }
    }

    // A 4xx other than 401/408/429 is a rejection the flush drops; 429 is
    // a back-off; 408 is an ordinary retryable status.
    #[tokio::test]
    async fn post_chunk_classifies_client_errors() {
        for (status, retry) in [
            (413u16, None),
            (400, None),
            (422, None),
            (429, Some("30")),
            (408, None),
        ] {
            let server = MockServer::start().await;
            let mut resp = ResponseTemplate::new(status);
            if let Some(r) = retry {
                resp = resp.insert_header("retry-after", r);
            }
            Mock::given(method("POST"))
                .and(path("/api/upload-chunk"))
                .respond_with(resp)
                .mount(&server)
                .await;
            let http = reqwest::Client::new();
            let err = post_chunk(&http, &format!("{}/api", server.uri()), "t", &[ev(0)])
                .await
                .unwrap_err();
            match (status, err) {
                (400 | 413 | 422, ChunkError::Rejected { status: s, .. }) => {
                    assert_eq!(s, status);
                }
                (408, ChunkError::Status { status: 408, .. }) => {}
                (429, e @ ChunkError::RateLimited { .. }) => {
                    assert_eq!(e.retry_after_secs(), Some(30));
                }
                (s, other) => panic!("{s}: unexpected {other:?}"),
            }
        }
    }

    // Other 5xx surfaces as Status, not KillSwitch — so the uploader's
    // backoff strategy can distinguish "ingest paused" from "server
    // hiccup."
    #[tokio::test]
    async fn post_chunk_5xx_other_than_503_surfaces_as_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/upload-chunk"))
            .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = post_chunk(&http, &format!("{}/api", server.uri()), "t", &[ev(0)])
            .await
            .unwrap_err();
        match err {
            ChunkError::Status { status, body } => {
                assert_eq!(status, 500);
                assert_eq!(body, "oops");
            }
            other => panic!("expected Status, got {other:?}"),
        }
    }
}
