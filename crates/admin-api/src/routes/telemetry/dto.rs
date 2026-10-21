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
    /// The chunk passed an expansion budget (decompressed bytes or rows):
    /// only the rows before it were parsed and replayed, the rest were
    /// dropped. Additive; uploaders that predate it ignore it.
    pub truncated: bool,
    /// Rows skipped because they were not UTF-8 or not an event the server
    /// knows. Additive.
    pub bad_rows: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct BundleResponse {
    /// Number of files unpacked from the zip.
    pub files: u64,
    /// Number of lines replayed through tracing across all files.
    pub lines: u64,
    /// Entries not replayed because they are not logs (key dumps, session
    /// JSON, and the like). Additive.
    #[serde(default)]
    pub skipped_not_log: u64,
    /// The bundle passed an expansion budget (entries, expanded bytes or
    /// lines): replay stopped there, newest files first. Additive.
    pub truncated: bool,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum IngestError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("Missing or malformed Authorization header")]
    MissingAuth,
    /// The token is valid but may not do this: a launcher token on the bundle
    /// route.
    #[error("Forbidden: {0}")]
    Forbidden(&'static str),
    #[error("Payload too large: {0} bytes (cap {1})")]
    TooLarge(usize, usize),
    #[error("gzip decode failed: {0}")]
    Gzip(String),
    #[error("zip decode failed: {0}")]
    Zip(String),
    #[error("multipart parse failed: {0}")]
    Multipart(String),
    /// The upload passed one of its budgets (expanded bytes, rows, zip
    /// entries, lines). Processing stopped at the first one hit.
    #[error("Upload exceeds the {what} limit ({limit})")]
    OverBudget { what: &'static str, limit: u64 },
    /// The request body failed part-way (broken framing, a peer that left).
    #[error("request body could not be read")]
    Body,
    /// A pool of upload slots is in use: `pool` is `peer_share` (the
    /// address's share) or `route` (the route's slots), and `limit` its size.
    #[error("Telemetry ingest is busy, retry shortly")]
    Busy { pool: &'static str, limit: usize },
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
            IngestError::Forbidden(_) => "forbidden",
            IngestError::TooLarge(_, _) => "body_too_large",
            IngestError::Gzip(_) => "bad_gzip",
            IngestError::Zip(_) => "bad_zip",
            IngestError::Multipart(_) => "bad_multipart",
            IngestError::OverBudget { .. } => "over_budget",
            IngestError::Body => "body_read_failed",
            IngestError::Busy { .. } => "busy",
        }
    }

    /// `true` for a refusal of the uploader's credential: no token, a token
    /// that does not verify, or one that expired. It describes the
    /// uploader's state, not the server's, so its row is SigNoz-only (see
    /// [`super::refusal_log::AUTH_REFUSED_EVENT`]). The kill switch and an
    /// unusable secret are server state and the rate limit is a budget, so
    /// those are not credential refusals.
    ///
    /// Every variant is listed, with no wildcard, so a new [`AuthError`] or
    /// [`IngestError`] variant fails to compile here until someone decides
    /// whether it posts to Discord.
    pub(super) fn is_credential_refusal(&self) -> bool {
        match self {
            IngestError::Auth(e) => match e {
                // The token the uploader sent: absent below, unreadable,
                // forged, expired, unscoped or past its lifetime cap.
                AuthError::BadPayload(_)
                | AuthError::BadSignature
                | AuthError::Expired { .. }
                | AuthError::MissingScope { .. }
                | AuthError::SessionLifetimeExceeded { .. }
                | AuthError::BadInstallId(_)
                | AuthError::BadField { .. }
                | AuthError::Json(_) => true,
                // Server configuration, an operator's switch, or a budget.
                AuthError::SecretMissing
                | AuthError::SecretTooShort { .. }
                | AuthError::KillSwitchActive
                | AuthError::NoUploadEndpoint
                | AuthError::QuotaExceeded(_) => false,
            },
            IngestError::MissingAuth | IngestError::Forbidden(_) => true,
            IngestError::TooLarge(_, _)
            | IngestError::Gzip(_)
            | IngestError::Zip(_)
            | IngestError::Multipart(_)
            | IngestError::OverBudget { .. }
            | IngestError::Body
            | IngestError::Busy { .. } => false,
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

    /// The limit a refusal hit, for its log row: the budget, or the size of
    /// the busy pool.
    pub(super) fn limit(&self) -> Option<u64> {
        match self {
            IngestError::OverBudget { limit, .. } => Some(*limit),
            IngestError::TooLarge(_, cap) => Some(*cap as u64),
            IngestError::Busy { limit, .. } => Some(*limit as u64),
            _ => None,
        }
    }

    /// The busy pool a refusal names, for its log row.
    pub(super) fn pool(&self) -> Option<&'static str> {
        match self {
            IngestError::Busy { pool, .. } => Some(pool),
            _ => None,
        }
    }
}

impl IntoResponse for IngestError {
    fn into_response(self) -> Response {
        let status = match &self {
            IngestError::Auth(e) => return e.to_response(),
            IngestError::MissingAuth => StatusCode::UNAUTHORIZED,
            IngestError::Forbidden(_) => StatusCode::FORBIDDEN,
            IngestError::TooLarge(_, _) => StatusCode::PAYLOAD_TOO_LARGE,
            IngestError::Gzip(_) | IngestError::Zip(_) | IngestError::Multipart(_) => {
                StatusCode::BAD_REQUEST
            }
            IngestError::OverBudget { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            IngestError::Body => StatusCode::BAD_REQUEST,
            IngestError::Busy { .. } => {
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
