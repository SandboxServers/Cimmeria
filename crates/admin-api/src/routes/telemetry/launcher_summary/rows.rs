//! The log rows the launcher-summary ingest emits.
//!
//! Three row shapes, all INFO, all built from validated values only:
//!
//! | Target | `event` | One per |
//! |---|---|---|
//! | `launcher.summary` | `launcher_summary` | accepted summary |
//! | `launcher.summary` | `launcher_phase` | `phases` entry of an accepted summary |
//! | `launcher.ingest` | `launcher_summary_batch` | request that reached validation |
//!
//! The phase rows have their own `event` so a count of attempts
//! (`event = 'launcher_summary'`) never counts a timed phase as one.
//!
//! `launcher.summary` is what a player's machine reported about itself, so
//! the server routes it to the `cimmeria-client` index
//! (`CLIENT_TARGETS` in `crates/server/src/otel.rs`). `launcher.ingest` is
//! the server's own account of a request and stays in `cimmeria-server`.
//!
//! An optional value that is absent is omitted, never written as a
//! sentinel: `tracing` records nothing for a `None` field. No row carries
//! the token's session or subject, the peer address, or any string the
//! client sent.

use std::fmt;

use crate::routes::dev_session::SESSION_KIND_LAUNCHER_SUMMARY;

use super::dto::{ClientDropped, PhaseTiming, Summary, SCHEMA_VERSION};

/// Target of the per-summary and per-phase rows. The `tracing` calls below
/// spell it out because the server's source scan
/// (`logging/target_scan_tests.rs`) only reads literal targets; the row
/// tests pin the two together.
pub const LAUNCHER_SUMMARY_TARGET: &str = "launcher.summary";
/// Target of the per-request batch row.
pub const LAUNCHER_SUMMARY_BATCH_TARGET: &str = "launcher.ingest";

/// `event` of the one row per accepted summary.
pub(super) const EVENT_SUMMARY: &str = "launcher_summary";
/// `event` of the one row per timed phase of an accepted summary.
pub(super) const EVENT_PHASE: &str = "launcher_phase";
/// `event` of the one row per request.
pub(super) const EVENT_BATCH: &str = "launcher_summary_batch";

/// Server-derived duration band, so a dashboard can chart durations with
/// `count()` alone.
pub(super) fn duration_bucket(ms: u32) -> &'static str {
    match ms {
        0..1_000 => "lt_1s",
        1_000..10_000 => "lt_10s",
        10_000..60_000 => "lt_1m",
        60_000..300_000 => "lt_5m",
        300_000..1_800_000 => "lt_30m",
        _ => "ge_30m",
    }
}

/// The launcher version, written from the parsed integers.
struct Version((u16, u16, u16));

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (major, minor, patch) = self.0;
        write!(f, "{major}.{minor}.{patch}")
    }
}

/// Emit the row for one accepted summary, then one row per timed phase.
pub(super) fn emit_summary(summary: &Summary) {
    let version = Version(summary.launcher_version);
    tracing::info!(
        target: "launcher.summary",
        event = EVENT_SUMMARY,
        event_id = %summary.event_id,
        attempt_id = %summary.attempt_id,
        operation = summary.operation.as_str(),
        phase = summary.phase.as_str(),
        outcome = summary.outcome.as_str(),
        error_code = summary.error_code.map(|code| code.as_str()),
        duration_ms = summary.duration_ms,
        duration_bucket = summary.duration_ms.map(duration_bucket),
        retry_count = summary.retry_count,
        launcher_version = %version,
        os = summary.os.as_str(),
        arch = summary.arch.as_str(),
        schema_version = SCHEMA_VERSION,
        cimmeria.session_kind = SESSION_KIND_LAUNCHER_SUMMARY,
        "launcher summary"
    );
    for timing in &summary.phases {
        emit_phase(summary, timing, &version);
    }
}

fn emit_phase(summary: &Summary, timing: &PhaseTiming, version: &Version) {
    tracing::info!(
        target: "launcher.summary",
        event = EVENT_PHASE,
        attempt_id = %summary.attempt_id,
        operation = summary.operation.as_str(),
        phase = timing.phase.as_str(),
        duration_ms = timing.duration_ms,
        duration_bucket = duration_bucket(timing.duration_ms),
        launcher_version = %version,
        os = summary.os.as_str(),
        arch = summary.arch.as_str(),
        schema_version = SCHEMA_VERSION,
        cimmeria.session_kind = SESSION_KIND_LAUNCHER_SUMMARY,
        "launcher phase"
    );
}

/// What one request amounted to: the server's own counts, and what the
/// launcher says it dropped before sending.
pub(super) fn emit_batch(accepted: u32, duplicate: u32, rejected: u32, dropped: ClientDropped) {
    tracing::info!(
        target: "launcher.ingest",
        event = EVENT_BATCH,
        accepted,
        duplicate,
        rejected,
        client_dropped_overflow = dropped.overflow,
        client_dropped_expired = dropped.expired,
        client_dropped_rejected = dropped.rejected,
        "launcher summary batch"
    );
}
