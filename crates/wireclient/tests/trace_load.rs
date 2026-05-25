//! Loader contract test — wireclient must consume the JSONL shape that
//! `tools/pcap_to_session.py` produces.
//!
//! Fixture: the first 5 events of the Castle Cellblock capture
//! (`2026-05-24_17-18.pcap`), hand-extracted to keep the corpus tiny in-tree.
//! Producer/consumer drift surfaces here, not at full-corpus load time.

use std::path::PathBuf;

use cimmeria_wireclient::session_trace::{Direction, Trace};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn loads_castle_cellblock_head_fixture() {
    let trace = Trace::from_jsonl_path(fixture_path("castle_cellblock_head.jsonl"))
        .expect("trace must load");

    // Header pins what the consumer relies on.
    assert_eq!(trace.header.label, "castle-cellblock-head");
    assert_eq!(trace.header.packet_count, 5);
    assert_eq!(trace.header.schema_version, 1);
    assert_eq!(trace.header.session_key_hex.len(), 64);

    // Event count must match the header — a producer that drops events
    // would otherwise pass silently.
    assert_eq!(
        trace.events.len(),
        trace.header.packet_count as usize,
        "events ({}) must equal header.packet_count ({})",
        trace.events.len(),
        trace.header.packet_count,
    );

    // Direction filter sanity.
    assert_eq!(trace.c2s().count(), 2);
    assert_eq!(trace.s2c().count(), 3);

    // First packet is the unencrypted baseAppLogin.
    let p1 = &trace.events[0];
    assert_eq!(p1.packet_no, 1);
    assert_eq!(p1.direction, Direction::C2s);
    assert_eq!(p1.seq, Some(1));
    assert_eq!(p1.flags, 0x41);
    assert_eq!(p1.messages[0].msg_id, 0x00);
    assert_eq!(p1.messages[0].name.as_deref(), Some("baseAppLogin"));

    // Third packet is the time-sync triple.
    let p3 = &trace.events[2];
    assert_eq!(p3.seq, Some(2));
    assert_eq!(p3.messages.len(), 3);
    assert_eq!(p3.messages[0].msg_id, 0x02);
    assert_eq!(p3.messages[1].msg_id, 0x0D);
    assert_eq!(p3.messages[2].msg_id, 0x03);

    // Body decoding round-trips back to bytes.
    let body = p3.messages[1].body_bytes().unwrap();
    assert_eq!(body.len(), 8, "tickSync body is 8 bytes (tick + tick_rate)");
    // tick = 0 (LE u32), tick_rate = 100 (LE u32 = 0x64)
    assert_eq!(&body[0..4], &[0, 0, 0, 0]);
    assert_eq!(&body[4..8], &[0x64, 0, 0, 0]);
}
