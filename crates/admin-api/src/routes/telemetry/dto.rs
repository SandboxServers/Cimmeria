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
    /// Lines parsed but not replayed because the session was over its
    /// event budget (`session_budget`). The launcher and the DLL ignore
    /// the body; the server's `launcher.ingest` warn is the report.
    pub suppressed: u64,
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
    /// The upload passed one of its budgets (expanded bytes, rows, zip
    /// entries, lines). Processing stopped at the first one hit.
    #[error("Upload exceeds the {what} limit ({limit})")]
    OverBudget { what: &'static str, limit: u64 },
    /// The request body failed part-way (broken framing, a peer that left).
    #[error("request body could not be read")]
    Body,
    /// Every upload slot of the route is in use.
    #[error("Telemetry ingest is busy, retry shortly")]
    Busy,
}

impl IngestError {
    /// `Retry-After` on an [`IngestError::Busy`] answer, in seconds: about
    /// two uploader flushes.
    pub(super) const BUSY_RETRY_AFTER_SECS: u64 = 5;

    /// The refusal's `reason` on its log row: a fixed string, never
    /// anything the caller sent.
    pub(super) fn reason(&self) -> &'static str {
        match self {
            IngestError::Auth(AuthError::KillSwitchActive) => "kill_switch",
            IngestError::Auth(AuthError::QuotaExceeded(_)) => "rate_limited",
            IngestError::Auth(AuthError::SecretMissing | AuthError::SecretTooShort { .. }) => {
                "secret_unusable"
            }
            IngestError::Auth(AuthError::Expired { .. }) => "token_expired",
            IngestError::Auth(_) => "bad_token",
            IngestError::MissingAuth => "missing_token",
            IngestError::TooLarge(_, _) => "body_too_large",
            IngestError::Gzip(_) => "bad_gzip",
            IngestError::Zip(_) => "bad_zip",
            IngestError::Multipart(_) => "bad_multipart",
            IngestError::Ndjson { .. } => "bad_ndjson",
            IngestError::OverBudget { .. } => "over_budget",
            IngestError::Body => "body_read_failed",
            IngestError::Busy => "busy",
        }
    }

    /// The budget a refusal names, for its log row.
    pub(super) fn budget(&self) -> Option<&'static str> {
        match self {
            IngestError::OverBudget { what, .. } => Some(what),
            IngestError::TooLarge(_, _) => Some("body bytes"),
            _ => None,
        }
    }

    /// The limit a refusal hit, for its log row.
    pub(super) fn limit(&self) -> Option<u64> {
        match self {
            IngestError::OverBudget { limit, .. } => Some(*limit),
            IngestError::TooLarge(_, cap) => Some(*cap as u64),
            _ => None,
        }
    }
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
            IngestError::OverBudget { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            IngestError::Body => StatusCode::BAD_REQUEST,
            IngestError::Busy => {
                let mut resp = (StatusCode::SERVICE_UNAVAILABLE, self.to_string()).into_response();
                resp.headers_mut().insert(
                    axum::http::header::RETRY_AFTER,
                    axum::http::HeaderValue::from(Self::BUSY_RETRY_AFTER_SECS),
                );
                return resp;
            }
        };
        (status, self.to_string()).into_response()
    }
}
