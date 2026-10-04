//! Launcher-summary ingest: `POST /api/telemetry/launcher-summary`.
//!
//! The desktop launcher reports, with the player's consent, how each
//! install, repair, uninstall, runtime setup or launch attempt ended: a
//! closed set of enums, a few bounded integers and two random ids. This
//! route validates a batch of those summaries, drops the ones it has
//! already seen, and writes one typed log row per accepted summary.
//!
//! It is a separate route from the chunk and bundle uploads on purpose.
//! Those replay whatever a session sends under the `telemetry.write`
//! scope; a summary token carries only `launcher_summary.write`, so
//! neither kind of token works on the other's routes.
//!
//! # Contract (schema version 1)
//!
//! ```json
//! { "schema_version": 1,
//!   "client_dropped": { "overflow": 0, "expired": 0, "rejected": 0 },
//!   "summaries": [ { "event_id": "uuid", "attempt_id": "uuid",
//!     "operation": "install", "phase": "download", "outcome": "failed",
//!     "error_code": "install_failed", "duration_ms": 81234, "retry_count": 0,
//!     "phases": [ { "phase": "starting", "duration_ms": 12 } ],
//!     "launcher_version": "0.1.0", "os": "windows", "arch": "x86_64" } ] }
//! ```
//!
//! The answer is `200 { "results": ["accepted" | "duplicate" | "rejected", …] }`,
//! one verdict per element in order. An element that breaks a rule is
//! `rejected` alone. A request that breaks the envelope (malformed JSON, an
//! unknown top-level key, a wrong `schema_version`, a bad `client_dropped`,
//! 0 or more than 32 summaries) is a 400. The other statuses are 401 (no
//! token, a bad one, or one without the summary scope), 413 (over 64 KiB),
//! 429 and 503 (quota and kill switch, both with `Retry-After`).
//!
//! The golden fixtures in
//! `crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/`
//! are shared with the launcher's exporter; both sides test against them.
//!
//! # Trust
//!
//! The mint needs no credential, so every summary is self-reported and
//! forgeable within the quota. The rows are for counting what opted-in
//! launchers say happened. They must never drive server state, an alert
//! or a success-rate target.
//!
//! No response body and no log row repeats anything the caller sent:
//! refusals are static text, and rows are built from parsed values.
//!
//! The request span around those rows belongs to the listener, not to this
//! module. On the public login port
//! ([`crate::login_port_telemetry_router`]) it records the method, the
//! path and the HTTP version, never the query string. The admin listener's
//! router (`build_router` in `lib.rs`) still uses tower-http's default
//! span, which records the full URI: there a caller-chosen query string is
//! a field of the span around every row of the request.
//!
//! # Module layout
//!
//! - `dto` — the closed wire enums, the envelope and element validation,
//!   the response and the error type.
//! - `dedup` — the fixed-size set of accepted `event_id`s.
//! - `rows` — the three log rows and the two tracing targets.
//! - `handlers` — the axum handler and its synchronous, injectable core.
//!
//! Test-only: `tests/` covers the route, and `fixture_tests/` holds the
//! operator fixtures in `docs/operations/signoz/` to the rows `rows`
//! really writes.

mod dedup;
mod dto;
mod handlers;
mod rows;

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod tests;

use axum::extract::DefaultBodyLimit;
use axum::routing::post;
use axum::Router;

pub use rows::{LAUNCHER_SUMMARY_BATCH_TARGET, LAUNCHER_SUMMARY_TARGET};

/// Request-body cap. The launcher sends at most 48 KiB per request; the
/// cap bounds what one request can make the server buffer before its
/// token is checked.
pub const MAX_SUMMARY_BODY_BYTES: usize = 64 * 1024;

/// The summary route alone. It is deliberately not part of
/// [`super::routes`]: each listener that serves it merges it by name
/// (`routes::api_routes` for the admin port,
/// [`crate::login_port_telemetry_router`] for the public login port), so
/// exposing it is one visible line per listener.
///
/// The handler reads the peer address for its quota, so the listener must
/// serve with `into_make_service_with_connect_info::<SocketAddr>()`.
pub fn launcher_summary_routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/launcher-summary", post(handlers::ingest))
        .layer(DefaultBodyLimit::max(MAX_SUMMARY_BODY_BYTES))
}
