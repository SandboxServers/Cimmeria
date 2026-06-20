//! Launcher → server telemetry ingest.
//!
//! Two endpoints accept launcher uploads, validate the bearer HMAC
//! token minted by [`crate::routes::dev_session`], and replay the payload
//! through the server's `tracing` subscriber. The OTLP layer (when
//! enabled via `OTEL_EXPORTER_OTLP_ENDPOINT`) ships those events to
//! SigNoz alongside the server's own logs and Mercury packet stream.
//!
//! # Endpoints
//!
//! | Path | Body | Purpose |
//! |---|---|---|
//! | `POST /api/telemetry/upload-chunk`  | gzip(NDJSON) | Streaming launcher events (one event per line). At-least-once delivery — the launcher retries on failure but the server does NOT dedupe by `(session_id, seq)`, so a duplicate retry appears as duplicate rows in SigNoz. |
//! | `POST /api/telemetry/upload-bundle` | multipart    | End-of-session zip of raw `Binaries/sgwdebuglog*` + `Binaries/sessions/**`. Unzipped server-side on a blocking thread; each log line emits a tracing event. |
//!
//! # Auth
//!
//! Same `Bearer` HMAC-SHA256 token shape as [`crate::routes::dev_session`].
//! Reuses [`crate::routes::dev_session::decode_token`] for verification.
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
//!
//! # Module layout
//!
//! - [`dto`] — wire types (event enum, response/error types) and the
//!   `IntoResponse` plumbing.
//! - [`handlers`] — the two axum handlers plus the unzip / verify /
//!   replay helpers they call.

mod dto;
mod handlers;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;

use cimmeria_services::orchestrator::Orchestrator;

use handlers::{upload_bundle, upload_chunk};

/// 16 MiB cap on a single chunk upload. The launcher's
/// `chunk_max_bytes` default is 1 MiB; this leaves ample headroom for
/// future tuning while preventing a misbehaving client from blowing
/// up server memory with a single POST.
const MAX_CHUNK_BYTES: usize = 16 * 1024 * 1024;

/// 256 MiB cap on a single bundle upload. The launcher's logs are
/// per-session and capped to ~50 MiB pre-zip in normal flows; the
/// cap exists purely as a defense against accidental log-cycle bombs.
const MAX_BUNDLE_BYTES: usize = 256 * 1024 * 1024;

/// Upper bound on the *uncompressed* output of a single gzip chunk.
/// Defends against a gzip bomb where a small compressed payload
/// expands to multi-GB output and exhausts memory. The dev-session
/// mint accepts any caller (v1 trust model), so this cap is the
/// load-bearing defense if an attacker forges a chunk.
///
/// 256 MiB = 16× the compressed `MAX_CHUNK_BYTES` cap — generous
/// enough that legitimate event streams never approach it, tight
/// enough that an attacker can't realistically allocate it.
const MAX_CHUNK_DECOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;

/// Upper bound on the uncompressed size of a single file inside a
/// bundle zip. Same rationale as `MAX_CHUNK_DECOMPRESSED_BYTES` —
/// guards against a zip bomb (a single deeply-compressed entry that
/// expands to multiple GB).
const MAX_BUNDLE_ENTRY_DECOMPRESSED_BYTES: u64 = 256 * 1024 * 1024;

pub fn routes() -> Router<Arc<Orchestrator>> {
    Router::new()
        // Both routes override axum's 2 MiB default body limit. Our
        // own size checks at the handler layer enforce the real cap;
        // the DefaultBodyLimit just lets the larger payloads through
        // to the handler so our cap message is what the client sees
        // (instead of axum's generic 413).
        .route(
            "/upload-chunk",
            post(upload_chunk).layer(DefaultBodyLimit::max(MAX_CHUNK_BYTES)),
        )
        .route(
            "/upload-bundle",
            post(upload_bundle).layer(DefaultBodyLimit::max(MAX_BUNDLE_BYTES)),
        )
}
