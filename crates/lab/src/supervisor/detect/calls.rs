//! The `lab.call` rows the daemon has written lately, and `log_error`: a tool
//! that fails three times within a minute. `server::handler` records a row
//! for every call; the rows live in a process-wide ring of [`CALLS_CAP`].

use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use serde_json::json;

use super::Fired;

/// Rows kept across all instances; the oldest go first.
const CALLS_CAP: usize = 256;
/// Errors of one tool that must fall within this many ms to fire.
const ERROR_WINDOW_MS: i64 = 60_000;
/// How many errors of one tool fire the rule.
const ERRORS_TO_FIRE: usize = 3;

/// One finished `tools/call`, as the `lab.call` log line says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRow {
    /// Daemon wall ms when the call finished.
    pub ms: i64,
    pub instance: String,
    pub tool: String,
    /// `ok`, `error` or `refused`.
    pub outcome: &'static str,
}

static CALLS: Mutex<VecDeque<CallRow>> = Mutex::new(VecDeque::new());

/// Keep a finished call for the detectors, dropping the oldest past the cap.
pub fn record(row: CallRow) {
    let mut calls = CALLS.lock().unwrap_or_else(|e| e.into_inner());
    if calls.len() == CALLS_CAP {
        calls.pop_front();
    }
    calls.push_back(row);
}

/// The recent calls of one instance, oldest first.
pub fn rows_for(instance: &str) -> Vec<CallRow> {
    CALLS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter(|r| r.instance == instance)
        .cloned()
        .collect()
}

/// The recent calls of one instance with ms >= from_ms, oldest first.
pub fn rows_since(instance: &str, from_ms: i64) -> Vec<CallRow> {
    rows_for(instance)
        .into_iter()
        .filter(|r| r.ms >= from_ms)
        .collect()
}

/// `log_error`: per tool, three `error` rows within 60 s fire once, at the
/// third. Rows dated after `now_ms` are ignored. The window restarts after a
/// firing, so the next one needs three more errors.
pub fn log_error(calls: &[CallRow], now_ms: i64) -> Vec<Fired> {
    let mut seen: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    let mut out = Vec::new();
    for row in calls
        .iter()
        .filter(|r| r.outcome == "error" && r.ms <= now_ms)
    {
        let times = seen.entry(row.tool.as_str()).or_default();
        times.push(row.ms);
        if times.len() < ERRORS_TO_FIRE {
            continue;
        }
        let first = times[times.len() - ERRORS_TO_FIRE];
        if row.ms - first <= ERROR_WINDOW_MS {
            out.push(Fired {
                name: "log_error",
                at_ms: row.ms,
                frame: None,
                detail: json!({ "tool": row.tool, "count": ERRORS_TO_FIRE }),
            });
            times.clear();
        } else {
            times.remove(0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ms: i64, tool: &str, outcome: &'static str) -> CallRow {
        CallRow {
            ms,
            instance: "default".into(),
            tool: tool.into(),
            outcome,
        }
    }

    #[test]
    fn log_error_three_in_a_minute() {
        let three = [
            row(0, "client_move_to", "error"),
            row(20_000, "client_move_to", "error"),
            row(50_000, "client_move_to", "error"),
        ];
        assert_eq!(log_error(&three, 60_000).len(), 1);
        assert!(log_error(&three[..2], 60_000).is_empty());
        let spread = [
            row(0, "client_move_to", "error"),
            row(35_000, "client_move_to", "error"),
            row(70_000, "client_move_to", "error"),
        ];
        assert!(log_error(&spread, 80_000).is_empty());
    }

    #[test]
    fn calls_ring_caps_at_256() {
        for k in 0..300 {
            record(CallRow {
                ms: k,
                instance: "ring-test".into(),
                tool: "lab_timeline".into(),
                outcome: "ok",
            });
        }
        let rows = rows_for("ring-test");
        assert!(rows.len() <= CALLS_CAP);
        assert_eq!(rows.last().map(|r| r.ms), Some(299));
    }

    #[test]
    fn rows_since_drops_old_rows() {
        for ms in [100, 200, 300] {
            record(CallRow {
                ms,
                instance: "since-test".into(),
                tool: "lab_timeline".into(),
                outcome: "error",
            });
        }
        let kept: Vec<i64> = rows_since("since-test", 200).iter().map(|r| r.ms).collect();
        assert_eq!(kept, [200, 300]);
    }
}
