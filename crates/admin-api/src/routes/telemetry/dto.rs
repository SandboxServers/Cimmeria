//! Wire types and HTTP response plumbing for the telemetry ingest endpoints.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::routes::dev_session::AuthError;

/// One streamed launcher event — mirrors [`cimmeria_launcher::
/// telemetry::events::TelemetryEvent`] byte-for-byte at the JSON
/// layer. Independent re-declaration here keeps `cimmeria-admin-api`
/// from depending on the launcher crate (which would pull in `tauri`
/// etc.); the type-tagged serde shape is the wire contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum TelemetryEvent {
    ClientLog(ClientLogEvent),
    DebugLog(DebugLogEvent),
    KeyDump(KeyDumpEvent),
    SessionMeta(SessionMetaEvent),
    /// Native event from the injected `cimmeria-client-telemetry`
    /// DLL (issue #417). Replayed under a distinct tracing target
    /// and a `service_name = cimmeria-client` field so SigNoz can
    /// slice client-side traces away from launcher / server events.
    ClientNative(ClientNativeEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClientLogEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub source_file: String,
    pub level: String,
    pub category: String,
    #[serde(default)]
    pub packet_no: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct DebugLogEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub source_file: String,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct KeyDumpEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub source_file: String,
    pub key_b64: String,
}

/// Mirror of [`cimmeria_launcher::telemetry::events::ClientNativeEvent`].
/// Wire shape pinned by `tests::client_native_event_matches_launcher_shape`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ClientNativeEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub target: String,
    pub level: String,
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct SessionMetaEvent {
    pub ts_ms: i64,
    pub seq: u64,
    pub kind: String,
    #[serde(default)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChunkResponse {
    pub accepted: u64,
    /// Echoed back so the launcher can log "we sent N, server accepted
    /// M". Drift between the two is the signal for a parse-error
    /// regression on either side.
    pub parsed_lines: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct BundleResponse {
    /// Number of files unpacked from the zip.
    pub files: u64,
    /// Number of lines replayed through tracing across all files.
    pub lines: u64,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum IngestError {
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
            IngestError::Auth(e) => return e.to_response(),
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
