//! `server_log_tail` tool logic — read the admin API's recent-log ring buffer.

use cimmeria_admin_api::ws::broadcast_layer::LogEntry;
use serde_json::{json, Value};

use crate::state::LabState;

/// Default number of log lines returned when the caller doesn't specify one.
pub const DEFAULT_LIMIT: usize = 100;

/// Filters applied to the buffered entries before the tail is taken. Every
/// set filter must match (AND); an unset filter matches everything.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// Level, case-insensitive (e.g. `"warn"`).
    pub level: Option<String>,
    /// Exact tracing target (`aoi.cinematic_hold`).
    pub target: Option<String>,
    /// Case-sensitive substring of the message.
    pub contains: Option<String>,
    /// Only entries with `timestamp_ms` strictly greater than this.
    pub since_ms: Option<u64>,
    /// Also return the client telemetry the server replays into its log
    /// (targets `client.native`, `launcher.client_log`, ...). Off by
    /// default: it floods the ring during a lab run.
    pub include_client: bool,
}

/// Keep only the entries that pass every set filter in `f`, in buffer order.
pub fn filter_entries(entries: Vec<LogEntry>, f: &LogFilter) -> Vec<LogEntry> {
    let want_level = f.level.as_ref().map(|lvl| lvl.to_uppercase());
    entries
        .into_iter()
        .filter(|e| {
            want_level
                .as_ref()
                .is_none_or(|lvl| e.level.eq_ignore_ascii_case(lvl))
                && f.target.as_ref().is_none_or(|t| e.target == *t)
                && f.contains.as_ref().is_none_or(|s| e.message.contains(s))
                && f.since_ms.is_none_or(|since| e.timestamp_ms > since)
        })
        .collect()
}

/// Return the last `limit` buffered log entries that pass `f`, optionally
/// filtered by level, target, message text and time. The buffer holds only the
/// most recent entries (see `broadcast_layer::BUFFER_CAPACITY` and
/// `RING_BYTE_BUDGET`; long messages are truncated), so this is a tail,
/// not a full history. Forwarded client telemetry is in a separate ring, read
/// only with `include_client`.
pub fn log_tail(state: &LabState, f: &LogFilter, limit: usize) -> Value {
    tail_reply(state.log_snapshot(reads_client_ring(f)), f, limit)
}

/// Whether a read must include the forwarded client ring: asked for, or the
/// target filter names one of its targets (an exact `target: "client.native"`
/// read would otherwise return nothing).
pub fn reads_client_ring(f: &LogFilter) -> bool {
    f.include_client
        || f.target
            .as_deref()
            .is_some_and(cimmeria_admin_api::ws::broadcast_layer::is_forwarded_client_target)
}

/// Build the `server_log_tail` reply from a buffer snapshot. `newest_ms` is
/// the newest timestamp in the whole buffer, taken before filtering, so a
/// caller can wait from it even when the filters match nothing.
pub fn tail_reply(entries: Vec<LogEntry>, f: &LogFilter, limit: usize) -> Value {
    let newest_ms = entries.iter().map(|e| e.timestamp_ms).max();
    let entries = filter_entries(entries, f);
    let total = entries.len();
    let start = total.saturating_sub(limit);
    let tail: Vec<Value> = entries[start..]
        .iter()
        .map(|e| {
            json!({
                "timestamp_ms": e.timestamp_ms,
                "level": e.level,
                "target": e.target,
                "message": e.message,
                "fields": e.fields,
            })
        })
        .collect();
    let mut reply = json!({ "count": tail.len(), "total_buffered": total, "entries": tail });
    if let Some(ms) = newest_ms {
        reply["newest_ms"] = json!(ms);
    }
    reply
}

#[cfg(test)]
#[path = "logs_tests.rs"]
mod tests;
