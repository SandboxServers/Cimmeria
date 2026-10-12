//! SV-02: `server_log_tail` filters combine (AND), `since_ms` is strictly
//! after, and `newest_ms` is taken before filtering.

use cimmeria_admin_api::ws::broadcast_layer::LogEntry;
use serde_json::json;

use super::{filter_entries, reads_client_ring, tail_reply, LogFilter};

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

#[test]
fn limit_takes_the_newest_matches_not_the_newest_entries() {
    let f = LogFilter {
        contains: Some("released".to_string()),
        ..LogFilter::default()
    };
    // Matches are 10, 30, 40; the newest two buffer entries (40, 50) hold one.
    let reply = tail_reply(fixture(), &f, 2);
    let got: Vec<u64> = reply["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["timestamp_ms"].as_u64().unwrap())
        .collect();
    assert_eq!(got, vec![30, 40]);
}

#[test]
fn an_empty_buffer_has_no_newest_ms() {
    let reply = tail_reply(vec![], &LogFilter::default(), 100);
    assert_eq!(reply["count"], 0);
    assert!(reply.get("newest_ms").is_none());
}

#[test]
fn level_ands_with_the_other_filters() {
    let mut entries = fixture();
    entries[3].level = "WARN".to_string();
    let f = LogFilter {
        level: Some("info".to_string()),
        contains: Some("released".to_string()),
        ..LogFilter::default()
    };
    assert_eq!(timestamps(&filter_entries(entries, &f)), vec![10, 30]);
}

/// An exact client target filter must reach the client ring, or
/// `server_log_tail {target: "client.native"}` silently returns nothing now
/// that the forwarded events live in a separate ring.
#[test]
fn a_client_target_filter_reads_the_client_ring() {
    let by_target = |t: &str| LogFilter {
        target: Some(t.to_string()),
        ..LogFilter::default()
    };
    assert!(reads_client_ring(&by_target("client.native")));
    assert!(reads_client_ring(&by_target("launcher.client_log")));
    assert!(!reads_client_ring(&by_target("aoi.zone")));
    assert!(!reads_client_ring(&LogFilter::default()));
    assert!(reads_client_ring(&LogFilter {
        include_client: true,
        ..LogFilter::default()
    }));
}
