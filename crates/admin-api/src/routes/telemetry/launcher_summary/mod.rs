//! Launcher-summary ingest: `POST /api/telemetry/launcher-summary`.
//!
//! The desktop launcher reports, with the player's consent, how each
//! install, repair, uninstall, runtime setup or launch attempt ended: a
//! closed set of enums, a few bounded integers and two random ids. This
//! route validates a batch of those summaries, drops the ones it has
//! already seen, and writes one typed log row per accepted summary.
//!
//! The route is anonymous: no token, no mint step, no `Authorization`
//! header (one that is sent is never read). What it accepts instead is one
//! exact payload, and everything else is refused before a row is written.
//!
//! It is a separate route from the chunk and bundle uploads on purpose.
//! Those need a dev-session token and replay whatever the session sends;
//! this one needs none and can store nothing but the values listed under
//! [Trust](#trust).
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
//! `rejected` alone, and the valid elements beside it are still accepted.
//!
//! Anything that is not that payload is refused whole, with a static body,
//! in this order:
//!
//! | Status | When |
//! |---|---|
//! | 503 + `Retry-After` | `CIMMERIA_TELEMETRY_KILL_SWITCH=1`. |
//! | 429 + `Retry-After` | The peer address is over its allowance (12 requests per 3600 s window by default, `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`). Charged before the body is read or anything is parsed, so every refusal below counts, the 413 included. An IPv4-mapped IPv6 peer (`::ffff:a.b.c.d`) is counted as `a.b.c.d`. |
//! | 415 | `Content-Type` is not `application/json` (a `charset` parameter is allowed). |
//! | 400 | The URI has a query string, an empty one (`?`) included. |
//! | 413 | The body is over 64 KiB. Read by the handler, after the checks above, so a paused or over-quota caller is answered with nothing buffered. |
//! | 400 | The body is not the envelope: unparseable JSON (nesting past `serde_json`'s 128 levels included), a top level that is not an object, a key written twice in the envelope, an unknown or missing top-level key, a wrong `schema_version`, a bad `client_dropped`, 0 or more than 32 summaries. A gzip body, an NDJSON body and a game-telemetry event all end here. |
//!
//! A key written twice inside an element (at any depth: the element
//! itself, or an entry of its `phases`) rejects that element alone.
//!
//! The golden fixtures in
//! `crates/launcher/desktop/engine/src/storage/launcher_summary/fixtures/`
//! are shared with the launcher's exporter; both sides test against them.
//!
//! # Trust
//!
//! The route is anonymous, so anyone can post correctly shaped rows within
//! the rate limit. Rows are self-reported; they are useful for spotting
//! failure patterns and must never drive server state, alerts,
//! success-rate claims or SLOs. The strict schema means nothing but closed
//! enum values, bounded integers, UUIDs and a version triple can ever be
//! stored.
//!
//! No response body and no log row repeats anything the caller sent:
//! refusals are static text, and rows are built from parsed values.
//!
//! The request span around those rows belongs to the listener, not to this
//! module. Both routers in this crate, the public login port's
//! ([`crate::login_port_telemetry_router`]) and the admin listener's
//! ([`crate::build_router`]), use one span
//! (`crate::request_span::request_span`). It records the method, the path
//! and the HTTP version: never the query string, and never the scheme or
//! host of an absolute-form target (`POST http://host/api/… HTTP/1.1`),
//! which is served like the bare path.
//!
//! The handler still refuses a query string before it writes a row. That
//! check does not depend on the span: a router that mounts
//! [`launcher_summary_routes`] under some other trace layer gets a 400 and
//! no row of this module for such a request, whatever its span records.
//! The query is the only part of the request target the handler checks.
//!
//! # Module layout
//!
//! - `dto` — the closed wire enums, element validation, the response and
//!   the error type.
//! - `envelope` — the one pass over the body: the envelope checks and the
//!   repeated-key detection.
//! - `dedup` — the fixed-size set of accepted `event_id`s.
//! - `rows` — the three log rows and the two tracing targets.
//! - `handlers` — the axum handler and its synchronous, injectable core.
//!
//! Test-only: `tests/` covers the route, and `fixture_tests/` holds the
//! operator fixtures in `docs/operations/signoz/` to the rows `rows`
//! really writes.

mod dedup;
mod dto;
mod envelope;
mod handlers;
mod rows;

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod tests;

use axum::routing::post;
use axum::Router;

pub use rows::{LAUNCHER_SUMMARY_BATCH_TARGET, LAUNCHER_SUMMARY_TARGET};

/// Request-body cap. The launcher sends at most 48 KiB per request; the
/// cap bounds what one admitted request can make the server buffer. The
/// handler enforces it while it reads the body, after the kill switch and
/// the quota, so a request over it is charged to the quota like any other.
pub const MAX_SUMMARY_BODY_BYTES: usize = 64 * 1024;

/// The summary route alone. It is deliberately not part of
/// [`super::routes`]: each listener that serves it merges it by name
/// (`routes::api_routes` for the admin port,
/// [`crate::login_port_telemetry_router`] for the public login port), so
/// exposing it is one visible line per listener.
///
/// The handler reads the peer address for its quota, so the listener must
/// serve with `into_make_service_with_connect_info::<SocketAddr>()`.
///
/// There is no `DefaultBodyLimit` layer: that limit applies to the body
/// extractors, and the handler takes the raw request and enforces
/// [`MAX_SUMMARY_BODY_BYTES`] itself.
pub fn launcher_summary_routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new().route("/launcher-summary", post(handlers::ingest))
}
