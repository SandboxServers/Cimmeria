//! SV-02: `server_log_tail` filters combine (AND), `since_ms` is strictly
//! after, and `newest_ms` is taken before filtering.

use cimmeria_admin_api::ws::broadcast_layer::LogEntry;
use serde_json::json;

use super::{filter_entries, tail_reply, LogFilter};

fn entry(timestamp_ms: u64, target: &str, message: &str) -> LogEntry {
    LogEntry {
        timestamp_ms,
        level: "INFO".to_string(),
        target: target.to_string(),
        message: message.to_string(),
        fields: json!({}),
    }
}

/// Five entries: two targets, three messages, timestamps 10 to 50.
fn fixture() -> Vec<LogEntry> {
    vec![
        entry(10, "aoi.cinematic_hold", "hold released"),
        entry(20, "aoi.cinematic_hold", "hold started"),
        entry(30, "aoi.zone", "hold released"),
        entry(40, "aoi.cinematic_hold", "hold released"),
        entry(50, "aoi.cinematic_hold", "hold queued"),
    ]
}

fn timestamps(entries: &[LogEntry]) -> Vec<u64> {
    entries.iter().map(|e| e.timestamp_ms).collect()
}

#[test]
fn filters_combine_target_text_and_time() {
    let f = LogFilter {
        target: Some("aoi.cinematic_hold".to_string()),
        contains: Some("released".to_string()),
        since_ms: Some(20),
        ..LogFilter::default()
    };
    let kept = filter_entries(fixture(), &f);
    assert_eq!(timestamps(&kept), vec![40]);
}

#[test]
fn since_is_strictly_after() {
    let f = LogFilter {
        since_ms: Some(40),
        ..LogFilter::default()
    };
    let kept = filter_entries(fixture(), &f);
    assert_eq!(timestamps(&kept), vec![50]);
}

#[test]
fn newest_ms_ignores_filters() {
    let f = LogFilter {
        target: Some("no.such.target".to_string()),
        ..LogFilter::default()
    };
    let reply = tail_reply(fixture(), &f, 100);
    assert_eq!(reply["count"], 0);
    assert_eq!(reply["newest_ms"], 50);
}
