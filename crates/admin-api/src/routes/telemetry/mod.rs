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
//! # Upload size and rate limits
//!
//! Both routes take the request with its body unread and run the gate in
//! [`upload_gate`] first: the kill switch (503), the token (401), a
//! per-session and a per-address rate limit (429 + `Retry-After`), and a
//! concurrency slot, server-wide and per address ([`upload_slots`]; 503 +
//! `Retry-After: 5`). Only then is the body read, within a deadline
//! ([`BODY_TIMEOUT`]). The compressed size and the bundle's multipart shape
//! refuse with 413; the expansion budgets (a chunk's decompressed bytes and
//! rows, a bundle's entries, expanded bytes and lines) stop processing
//! there and answer 200 with `truncated: true`, so an uploader that sent
//! too much does not retry the same oversized upload forever. Client
//! strings are cut to the caps in [`field_caps`] before they reach a log
//! event, and every refusal or truncation writes one throttled `warn`
//! ([`refusal_log`]) with the reason, never the payload.
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
//! - [`upload_gate`] — what runs before a body is read: kill switch, token,
//!   rate limits, concurrency slots; and the size budgets as a struct.
//! - [`upload_slots`] — the concurrency slots, server-wide and per address.
//! - [`chunk`] — the chunk handler: bounded read, expansion and parse,
//!   truncating at the expansion budgets.
//! - [`bundle`] — the bundle handler; [`bundle_unzip`] its bounded,
//!   newest-first unzip.
//! - [`field_caps`] — length caps on client strings, with a marker.
//! - [`refusal_log`] — the throttled refusal `warn`.
//! - [`replay`] — replaying each uploaded event through `tracing` with
//!   the session's identity and kind; the client-side rows land in the
//!   `cimmeria-client` SigNoz service.
//! - [`replay_native`] — one injected-DLL row, its IDs named (NT-40):
//!   content IDs from the NameBook, method indexes and message ids from the
//!   NT-30 wire tables, addresses from [`client_symbols`].
//! - [`entity_labels`] — naming entity IDs, which needs the cell: the
//!   row's time on the server clock, its space, and one query per chunk.
//! - [`client_symbols`] — SGW.exe function names for native addresses,
//!   from the committed `client_symbols.tsv`.
//! - [`session_budget`] — per-session accepted/suppressed totals and the
//!   runaway-client guard (a per-minute event budget past which only
//!   warn/error and boot events are replayed, up to an allowance of their
//!   own, reported at `warn`).
//! - [`launcher_summary`] — a third endpoint with its own router and rows:
//!   `POST /api/telemetry/launcher-summary`, the desktop launcher's attempt
//!   summaries. Unlike the two uploads it is anonymous (no token, a strict
//!   payload schema and a per-address quota of 12 a minute instead). Not part of
//!   [`routes`]; see [`launcher_summary_routes`].

mod bundle;
mod bundle_unzip;
mod chunk;
mod client_symbols;
mod dto;
mod entity_labels;
mod field_caps;
mod launcher_summary;
mod refusal_log;
mod replay;
mod replay_native;
mod session_budget;
mod upload_gate;
mod upload_slots;

#[cfg(test)]
mod entity_labels_tests;
#[cfg(test)]
mod replay_names_tests;
#[cfg(test)]
mod replay_tests;
#[cfg(test)]
mod session_budget_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod upload_limits_tests;

pub use client_symbols::load_client_symbols;
pub use entity_labels::{connect_entity_labels, EntityLabelLink};
pub use launcher_summary::{
    launcher_summary_routes, LAUNCHER_SUMMARY_BATCH_TARGET, LAUNCHER_SUMMARY_TARGET,
    MAX_SUMMARY_BODY_BYTES,
};
pub use replay::{replay_ndjson, replay_ndjson_named, ReplayCounts, ReplayError};

use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;

use bundle::upload_bundle;
use chunk::upload_chunk;

// Size budgets. Measured against what the uploaders send: the mint tells
// the launcher `chunk_max_bytes` = 1 MiB of NDJSON per chunk, and the DLL
// posts at most `max_batch` = 1,000 events (about 2,000 after a failed POST
// is retained), a few hundred bytes each, every 2 s.

/// Compressed chunk body. A 1 MiB NDJSON chunk gzips to well under 1 MiB,
/// but launchers released before the upload limits post their whole
/// backlog as one chunk and retry a refused one forever; this is the cap
/// they have always had, so no backlog that used to be accepted is
/// refused now. Only the first [`MAX_CHUNK_DECOMPRESSED_BYTES`] of it are
/// ever expanded.
const MAX_CHUNK_BYTES: usize = 16 * 1024 * 1024;

/// A chunk's NDJSON expanded at most this far: 8× the launcher's
/// `chunk_max_bytes` and 4× the DLL's largest retained batch. Past it the
/// chunk is cut at the last whole line and the rest dropped (a launcher
/// that posts a long backlog as one chunk sends more than this).
const MAX_CHUNK_DECOMPRESSED_BYTES: u64 = 8 * 1024 * 1024;

/// Rows replayed from one chunk: 5× the DLL's largest retained batch.
/// Further rows are counted and dropped.
const MAX_CHUNK_ROWS: usize = 10_000;

/// Compressed bundle zip. The launcher's logs are per-session and about
/// 50 MiB before zipping in normal flows; log text zips 5-10×.
const MAX_BUNDLE_BYTES: usize = 32 * 1024 * 1024;

/// Every file in a bundle together, expanded.
const MAX_BUNDLE_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;

/// Files replayed from a bundle zip, newest first. The client's session
/// logs rotate every minute, so a long session leaves a few hundred, and
/// launchers before the per-session bundle sent every past session too.
const MAX_BUNDLE_ENTRIES: usize = 1024;

/// Files past which a bundle is refused outright (413), from its end
/// record, before the archive is opened: opening it reads every entry's
/// header into memory.
const MAX_BUNDLE_ENTRIES_HARD: usize = 16 * 1024;

/// Lines a bundle replays, one log event each.
const MAX_BUNDLE_LINES: u64 = 250_000;

/// The bundle's `metadata` part: a dozen JSON fields.
const MAX_BUNDLE_METADATA_BYTES: usize = 16 * 1024;

/// Multipart parts in a bundle: the launcher sends two.
const MAX_BUNDLE_PARTS: usize = 8;

/// Chunks buffered, expanded or replayed at once, server-wide. Past it a
/// chunk is refused with 503 and retried on the uploader's next flush.
const CHUNK_SLOTS: usize = 4;

/// Bundles buffered or expanded at once, server-wide.
const BUNDLE_SLOTS: usize = 2;

/// Uploads of one route one peer address may have in flight at once.
const SLOTS_PER_PEER: usize = 1;

/// How long a request may take to deliver its body. A launcher sends a
/// chunk in well under a second and a bundle in a few.
const BODY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Generic over the router state for the same reason as
/// [`crate::routes::dev_session::routes`]: the handlers read none, so the
/// public login port can mount them too.
pub fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        // The chunk handler takes the raw request and reads the body
        // itself, capped, after the gate; `DefaultBodyLimit` does not
        // apply to it.
        .route("/upload-chunk", post(upload_chunk))
        // The bundle's `Multipart` honours `DefaultBodyLimit`: the zip
        // part's own cap plus room for the metadata part and the
        // multipart framing.
        .route(
            "/upload-bundle",
            post(upload_bundle).layer(DefaultBodyLimit::max(
                MAX_BUNDLE_BYTES + MAX_BUNDLE_METADATA_BYTES + 64 * 1024,
            )),
        )
}
