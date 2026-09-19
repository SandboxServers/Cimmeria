//! `server_log_tail` tool logic — read the admin API's recent-log ring buffer.

use serde_json::{json, Value};

use crate::state::LabState;

/// Default number of log lines returned when the caller doesn't specify one.
pub const DEFAULT_LIMIT: usize = 100;

/// Return the last `limit` buffered log entries, optionally filtered to a
/// single level (case-insensitive, e.g. `"warn"`). The buffer holds only the
/// most recent entries (see `LogBuffer::BUFFER_CAPACITY`), so this is a tail,
/// not a full history.
pub fn log_tail(state: &LabState, limit: usize, level: Option<&str>) -> Value {
    let mut entries = state.log_snapshot();
    if let Some(lvl) = level {
        let want = lvl.to_uppercase();
        entries.retain(|e| e.level.eq_ignore_ascii_case(&want));
    }
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
    json!({ "count": tail.len(), "total_buffered": total, "entries": tail })
}
