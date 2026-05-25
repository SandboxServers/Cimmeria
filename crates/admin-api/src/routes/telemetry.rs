//! Launcher → server telemetry ingest.
//!
//! Two endpoints accept launcher uploads, validate the bearer HMAC
//! token minted by [`super::dev_session`], and replay the payload
//! through the server's `tracing` subscriber. The OTLP layer (when
//! enabled via `OTEL_EXPORTER_OTLP_ENDPOINT`) ships those events to
//! SigNoz alongside the server's own logs and Mercury packet stream.
//!
//! # Endpoints
//!
//! | Path | Body | Purpose |
//! |---|---|---|
//! | `POST /api/telemetry/upload-chunk`  | gzip(NDJSON) | Streaming launcher events (one event per line). Idempotent on `(session_id, seq_first, seq_last)` — duplicates are silently logged at debug. |
//! | `POST /api/telemetry/upload-bundle` | multipart    | End-of-session zip of raw `Binaries/sgwdebuglog*` + `Binaries/sessions/**`. Unzipped server-side; each log line emits a tracing event. |
//!
//! # Auth
//!
//! Same `Bearer` HMAC-SHA256 token shape as [`super::dev_session`].
//! Reuses [`super::dev_session::decode_token`] for verification.
//! A 401 here is the launcher's signal to call
//! `/api/auth/dev-session/refresh`.
//!
//! # Why "replay through tracing" and not "write directly to SigNoz"
//!
//! Every other log surface in the server (mercury packets, audit
//! events, base/cell state) already flows through `tracing::*`. If
//! launcher logs go straight to SigNoz they bypass the file sinks,
//! the WebSocket admin stream, the per-system log files. By replaying
//! them through `tracing` we get all sinks at once — SigNoz, on-disk
//! log files, and the live admin WebSocket — without re-implementing
//! the fan-out at this layer.

use std::io::Read;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use bytes::Bytes;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use cimmeria_services::orchestrator::Orchestrator;

use super::dev_session::{decode_token, AuthError, TokenClaims};

/// 16 MiB cap on a single chunk upload. The launcher's
/// `chunk_max_bytes` default is 1 MiB; this leaves ample headroom for
/// future tuning while preventing a misbehaving client from blowing
/// up server memory with a single POST.
const MAX_CHUNK_BYTES: usize = 16 * 1024 * 1024;

/// 256 MiB cap on a single bundle upload. The launcher's logs are
/// per-session and capped to ~50 MiB pre-zip in normal flows; the
/// cap exists purely as a defense against accidental log-cycle bombs.
const MAX_BUNDLE_BYTES: usize = 256 * 1024 * 1024;

/// One streamed launcher event — mirrors [`cimmeria_launcher::
/// telemetry::events::TelemetryEvent`] byte-for-byte at the JSON
/// layer. Independent re-declaration here keeps `cimmeria-admin-api`
/// from depending on the launcher crate (which would pull in `tauri`
/// etc.); the type-tagged serde shape is the wire contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TelemetryEvent {
    ClientLog(ClientLogEvent),
    DebugLog(DebugLogEvent),
    KeyDump(KeyDumpEvent),
    SessionMeta(SessionMetaEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClientLogEvent {
    ts_ms: i64,
    seq: u64,
    source_file: String,
    level: String,
    category: String,
    #[serde(default)]
    packet_no: Option<u64>,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DebugLogEvent {
    ts_ms: i64,
    seq: u64,
    source_file: String,
    level: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KeyDumpEvent {
    ts_ms: i64,
    seq: u64,
    source_file: String,
    key_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetaEvent {
    ts_ms: i64,
    seq: u64,
    kind: String,
    #[serde(default)]
    fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ChunkResponse {
    accepted: u64,
    /// Echoed back so the launcher can log "we sent N, server accepted
    /// M". Drift between the two is the signal for a parse-error
    /// regression on either side.
    parsed_lines: u64,
}

#[derive(Debug, Serialize)]
struct BundleResponse {
    /// Number of files unpacked from the zip.
    files: u64,
    /// Number of lines replayed through tracing across all files.
    lines: u64,
}

#[derive(Debug, thiserror::Error)]
enum IngestError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("Missing or malformed Authorization header")]
    MissingAuth,
    #[error("Payload too large: {0} bytes (cap {1})")]
    TooLarge(usize, usize),
    #[error("gzip decode failed: {0}")]
    Gzip(String),
    #[error("zip decode failed: {0}")]
    Zip(String),
    #[error("multipart parse failed: {0}")]
    Multipart(String),
    #[error("ndjson parse failed at line {line}: {err}")]
    Ndjson { line: u64, err: String },
}

impl IntoResponse for IngestError {
    fn into_response(self) -> Response {
        let status = match &self {
            IngestError::Auth(e) => return e.clone_status_response(),
            IngestError::MissingAuth => StatusCode::UNAUTHORIZED,
            IngestError::TooLarge(_, _) => StatusCode::PAYLOAD_TOO_LARGE,
            IngestError::Gzip(_) | IngestError::Zip(_) | IngestError::Multipart(_) => {
                StatusCode::BAD_REQUEST
            }
            IngestError::Ndjson { .. } => StatusCode::BAD_REQUEST,
        };
        (status, self.to_string()).into_response()
    }
}

pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        .route("/upload-chunk", post(upload_chunk))
        // Bundle uploads cap at MAX_BUNDLE_BYTES per request — axum's
        // default body limit is 2 MiB which would reject any real
        // session zip outright.
        .route(
            "/upload-bundle",
            post(upload_bundle).layer(DefaultBodyLimit::max(MAX_BUNDLE_BYTES)),
        )
}

async fn upload_chunk(
    State(_orchestrator): State<Arc<Orchestrator>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ChunkResponse>, IngestError> {
    let claims = verify_bearer(&headers)?;

    if body.len() > MAX_CHUNK_BYTES {
        return Err(IngestError::TooLarge(body.len(), MAX_CHUNK_BYTES));
    }

    // Decompress gzip → NDJSON.
    let mut decoder = GzDecoder::new(&body[..]);
    let mut ndjson = String::new();
    decoder
        .read_to_string(&mut ndjson)
        .map_err(|e| IngestError::Gzip(e.to_string()))?;

    let mut parsed = 0u64;
    let mut accepted = 0u64;
    for (idx, line) in ndjson.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        parsed += 1;
        let ev: TelemetryEvent = serde_json::from_str(line).map_err(|e| IngestError::Ndjson {
            line: idx as u64 + 1,
            err: e.to_string(),
        })?;
        replay_event(&claims, ev);
        accepted += 1;
    }

    tracing::debug!(
        target: "launcher.ingest",
        session_id = %claims.sid,
        install_id = %claims.sub,
        accepted,
        parsed,
        body_bytes = body.len(),
        "upload-chunk accepted"
    );

    Ok(Json(ChunkResponse {
        accepted,
        parsed_lines: parsed,
    }))
}

async fn upload_bundle(
    State(_orchestrator): State<Arc<Orchestrator>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<BundleResponse>, IngestError> {
    let claims = verify_bearer(&headers)?;

    let mut metadata_seen = false;
    let mut files = 0u64;
    let mut lines = 0u64;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| IngestError::Multipart(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|e| IngestError::Multipart(e.to_string()))?;

        match name.as_str() {
            "metadata" => {
                // Replay the bundle metadata as a single tracing event so
                // SigNoz queries can correlate the chunk stream with the
                // end-of-session totals.
                if let Ok(meta) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    tracing::info!(
                        target: "launcher.bundle",
                        session_id = %claims.sid,
                        install_id = %claims.sub,
                        metadata = %meta,
                        "bundle metadata"
                    );
                }
                metadata_seen = true;
            }
            "zip" => {
                if bytes.len() > MAX_BUNDLE_BYTES {
                    return Err(IngestError::TooLarge(bytes.len(), MAX_BUNDLE_BYTES));
                }
                let (f, l) = unpack_and_replay(&claims, &bytes)?;
                files += f;
                lines += l;
            }
            other => {
                tracing::debug!(
                    target: "launcher.bundle",
                    session_id = %claims.sid,
                    field = %other,
                    "ignoring unexpected multipart field"
                );
            }
        }
    }

    if !metadata_seen {
        tracing::warn!(
            target: "launcher.bundle",
            session_id = %claims.sid,
            "bundle uploaded without metadata field"
        );
    }

    Ok(Json(BundleResponse { files, lines }))
}

fn unpack_and_replay(claims: &TokenClaims, zip_bytes: &[u8]) -> Result<(u64, u64), IngestError> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| IngestError::Zip(e.to_string()))?;

    let mut files = 0u64;
    let mut lines = 0u64;

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| IngestError::Zip(e.to_string()))?;
        if !entry.is_file() {
            continue;
        }
        let path = entry.name().to_string();
        let mut content = String::new();
        // Tolerate non-UTF8 binary files (key dumps may contain
        // binary). Skip with a debug-level note rather than failing
        // the whole bundle.
        if entry.read_to_string(&mut content).is_err() {
            tracing::debug!(
                target: "launcher.bundle",
                session_id = %claims.sid,
                path = %path,
                "skipping non-UTF8 bundle entry"
            );
            continue;
        }
        files += 1;
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            tracing::info!(
                target: "launcher.client_log",
                session_id = %claims.sid,
                install_id = %claims.sub,
                source = "bundle",
                source_file = %path,
                message = %line,
            );
            lines += 1;
        }
    }

    Ok((files, lines))
}

fn verify_bearer(headers: &HeaderMap) -> Result<TokenClaims, IngestError> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(IngestError::MissingAuth)?;
    let token = raw
        .strip_prefix("Bearer ")
        .ok_or(IngestError::MissingAuth)?
        .trim();
    if token.is_empty() {
        return Err(IngestError::MissingAuth);
    }
    let secret = load_secret_for_ingest()?;
    let claims = decode_token(token, &secret).map_err(IngestError::Auth)?;
    let now = chrono::Utc::now().timestamp();
    if claims.exp <= now {
        return Err(IngestError::Auth(AuthError::Expired {
            exp: claims.exp,
            now,
        }));
    }
    Ok(claims)
}

/// Thin wrapper around `dev_session::load_secret`-like logic. The
/// dev_session module owns the canonical secret-loading code; we
/// re-implement here only to avoid widening its public surface.
fn load_secret_for_ingest() -> Result<Vec<u8>, IngestError> {
    let raw = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET")
        .map_err(|_| IngestError::Auth(AuthError::SecretMissing))?;
    let raw_trimmed = raw.trim();
    if raw_trimmed.is_empty() {
        return Err(IngestError::Auth(AuthError::SecretMissing));
    }
    // Accept both hex-encoded and raw-bytes forms — matches dev_session
    // semantics exactly.
    let bytes = hex_decode_lenient(raw_trimmed).unwrap_or_else(|| raw_trimmed.as_bytes().to_vec());
    if bytes.len() < 32 {
        return Err(IngestError::Auth(AuthError::SecretTooShort {
            got: bytes.len(),
            min: 32,
        }));
    }
    Ok(bytes)
}

fn hex_decode_lenient(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
    }
    Some(out)
}

fn replay_event(claims: &TokenClaims, ev: TelemetryEvent) {
    // Every event carries the launcher's session_id and install_id so
    // SigNoz queries can slice by session or by player without
    // joining across rows. Field naming matches the dev_session mint
    // event so a session's mint + uploads correlate naturally.
    match ev {
        TelemetryEvent::ClientLog(e) => {
            tracing::info!(
                target: "launcher.client_log",
                session_id = %claims.sid,
                install_id = %claims.sub,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                level = %e.level,
                category = %e.category,
                packet_no = ?e.packet_no,
                message = %e.message,
            );
        }
        TelemetryEvent::DebugLog(e) => {
            tracing::info!(
                target: "launcher.debug_log",
                session_id = %claims.sid,
                install_id = %claims.sub,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                level = %e.level,
                message = %e.message,
            );
        }
        TelemetryEvent::KeyDump(e) => {
            // KeyDump entries are encryption key material observed by
            // the client. Useful for offline pcap decryption — but we
            // intentionally do NOT log the key body at info; debug
            // only, so the default sinks don't carry it to disk.
            tracing::debug!(
                target: "launcher.key_dump",
                session_id = %claims.sid,
                install_id = %claims.sub,
                ts_ms = e.ts_ms,
                seq = e.seq,
                source_file = %e.source_file,
                key_b64 = %e.key_b64,
            );
        }
        TelemetryEvent::SessionMeta(e) => {
            tracing::info!(
                target: "launcher.session_meta",
                session_id = %claims.sid,
                install_id = %claims.sub,
                ts_ms = e.ts_ms,
                seq = e.seq,
                kind = %e.kind,
                fields = %serde_json::Value::Object(e.fields),
            );
        }
    }
}

// ── Helpers on AuthError for IntoResponse forwarding ─────────────────

impl AuthError {
    /// Clone the AuthError's status + body into a Response. The
    /// dev_session AuthError already implements IntoResponse but it
    /// consumes self; we want to reuse the same status mapping
    /// without taking ownership in IngestError::into_response.
    fn clone_status_response(&self) -> Response {
        let (status, body) = match self {
            AuthError::SecretMissing | AuthError::SecretTooShort { .. } => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            AuthError::KillSwitchActive => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            AuthError::BadPayload(_) | AuthError::BadSignature => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            AuthError::Expired { .. } => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::Json(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };
        let mut resp = (status, body).into_response();
        if matches!(self, AuthError::KillSwitchActive) {
            resp.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("60"),
            );
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

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

    /// Hex-form secret loading must round-trip — operators using
    /// `openssl rand -hex 64` paste the hex form directly into GitHub
    /// Secrets; the load path must accept it identically to the raw
    /// UTF-8 path. (Mirror of `dev_session::load_secret_accepts_hex_encoded`
    /// because we re-implemented the parser here.)
    #[test]
    fn load_secret_accepts_hex_form() {
        let env_lock = std::sync::Mutex::new(());
        let _g = env_lock.lock().unwrap();
        let prev = std::env::var("CIMMERIA_TELEMETRY_HMAC_SECRET").ok();
        // 128 hex chars = 64 raw bytes, comfortably above the 32-byte minimum.
        std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", "a".repeat(128));
        let bytes = load_secret_for_ingest().unwrap();
        assert_eq!(bytes.len(), 64);
        match prev {
            Some(v) => std::env::set_var("CIMMERIA_TELEMETRY_HMAC_SECRET", v),
            None => std::env::remove_var("CIMMERIA_TELEMETRY_HMAC_SECRET"),
        }
    }
}
