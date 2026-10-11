//! Tests for startup recovery of a previous run's queue and for `seq`
//! stamping.

use super::*;
use crate::telemetry::events::{ClientLogEvent, TelemetryEvent};

fn ev(i: i64) -> TelemetryEvent {
    TelemetryEvent::ClientLog(ClientLogEvent {
        ts_ms: i,
        seq: i as u64,
        source_file: "x.log".into(),
        level: "info".into(),
        category: "raw".into(),
        packet_no: None,
        message: format!("e-{i}"),
    })
}

#[test]
fn recover_pending_returns_zero_on_empty_queue() {
    let dir = tempfile::tempdir().unwrap();
    let q = DiskQueue::new(dir.path());
    assert_eq!(recover_pending_at(&q), 0);
}

#[test]
fn recover_pending_drains_and_reenqueues() {
    let dir = tempfile::tempdir().unwrap();
    let q = DiskQueue::new(dir.path());
    for i in 0..3 {
        q.enqueue(&ev(i)).unwrap();
    }
    let n = recover_pending_at(&q);
    assert_eq!(n, 3);
    let drained: Vec<TelemetryEvent> = q.drain().unwrap();
    assert_eq!(drained.len(), 3);
}

/// A line that is not UTF-8 neither hides the queue from recovery nor
/// costs the events around it.
#[test]
fn recover_pending_counts_past_a_non_utf8_line() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let q = DiskQueue::new(dir.path());
    q.enqueue(&ev(0)).unwrap();
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join("telemetry-queue.jsonl"))
        .unwrap();
    f.write_all(b"\x80\x81 torn\n").unwrap();
    drop(f);
    q.enqueue(&ev(1)).unwrap();
    assert_eq!(recover_pending_at(&q), 2);
    assert_eq!(q.drain::<TelemetryEvent>().unwrap().len(), 2);
}

// stamp_seq rewrites the seq field on every variant and fills in
// ts_ms when zero. Pinned because the orchestrator's monotonic
// ordering guarantee depends on it.
#[test]
fn stamp_seq_writes_seq_on_every_variant() {
    let cases = vec![
        TelemetryEvent::ClientLog(ClientLogEvent {
            ts_ms: 0,
            seq: 0,
            source_file: "x".into(),
            level: "i".into(),
            category: "c".into(),
            packet_no: None,
            message: "m".into(),
        }),
        TelemetryEvent::DebugLog(events::DebugLogEvent {
            ts_ms: 0,
            seq: 0,
            source_file: "x".into(),
            level: "i".into(),
            message: "m".into(),
        }),
        TelemetryEvent::KeyDump(events::KeyDumpEvent {
            ts_ms: 0,
            seq: 0,
            source_file: "x".into(),
            key_b64: "k".into(),
        }),
        TelemetryEvent::SessionMeta(events::SessionMetaEvent {
            ts_ms: 0,
            seq: 0,
            kind: events::SessionMetaKind::Started,
            fields: serde_json::Map::new(),
        }),
        TelemetryEvent::ClientNative(events::ClientNativeEvent {
            ts_ms: 0,
            seq: 0,
            target: "client.frame_tick".into(),
            level: "info".into(),
            fields: serde_json::Map::new(),
        }),
    ];
    for mut ev in cases {
        stamp_seq(&mut ev, 42);
        let seq_after = match &ev {
            TelemetryEvent::ClientLog(e) => e.seq,
            TelemetryEvent::DebugLog(e) => e.seq,
            TelemetryEvent::KeyDump(e) => e.seq,
            TelemetryEvent::SessionMeta(e) => e.seq,
            TelemetryEvent::ClientNative(e) => e.seq,
        };
        assert_eq!(seq_after, 42);
    }
}
