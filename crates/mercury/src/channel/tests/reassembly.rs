use super::*;
use std::net::SocketAddr;

fn test_addr() -> SocketAddr {
    "127.0.0.1:9000".parse().unwrap()
}

fn test_packet() -> Packet {
    use crate::packet::PacketFlags;
    use bytes::Bytes;

    Packet::new(
        PacketFlags::default(),
        0,
        Bytes::from_static(&[0xDE, 0xAD]),
    )
}

// ── Per-channel fragment reassembly ──────────────────────────────

/// Build then parse a fragmented Mercury packet — same helper shape
/// `unpacker::tests` uses, duplicated here because the inner module
/// is private.
fn build_then_parse_fragment(
    seq: u32,
    frag_begin: u32,
    frag_end: u32,
    body: &[u8],
) -> crate::packet::ParsedPacket {
    use crate::packet::{build_outgoing_fragmented, parse_incoming};
    let raw = build_outgoing_fragmented(0, body, seq, frag_begin, frag_end, &[]);
    parse_incoming(&raw).unwrap()
}

#[test]
fn reassemble_parsed_passes_through_non_fragmented() {
    use crate::packet::{build_outgoing, parse_incoming, FLAG_HAS_SEQUENCE};
    let mut ch = Channel::new(test_addr());
    let raw = build_outgoing(FLAG_HAS_SEQUENCE, b"hello", Some(7), &[], None);
    let parsed = parse_incoming(&raw).unwrap();

    let body = ch
        .reassemble_parsed(&parsed)
        .unwrap()
        .expect("non-fragmented should pass through");
    assert_eq!(body.as_ref(), b"hello");
}

#[test]
fn reassemble_parsed_completes_3_fragment_bundle() {
    let mut ch = Channel::new(test_addr());
    let f0 = build_then_parse_fragment(10, 10, 12, b"AAA");
    let f1 = build_then_parse_fragment(11, 10, 12, b"BBB");
    let f2 = build_then_parse_fragment(12, 10, 12, b"CCC");

    assert!(ch.reassemble_parsed(&f0).unwrap().is_none());
    assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
    let body = ch
        .reassemble_parsed(&f2)
        .unwrap()
        .expect("third fragment completes");
    assert_eq!(body.as_ref(), b"AAABBBCCC");
}

#[test]
fn reassemble_parsed_bumps_last_received() {
    // Receive-side observation must move last_received so the
    // peer-silence detector sees fragment activity as keepalive-
    // equivalent. Without this, a peer streaming a large bundle of
    // fragments would still look idle until the bundle assembled.
    let mut ch = Channel::new(test_addr());
    let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
    ch.last_received = baseline;
    ch.last_sent = baseline;

    let f0 = build_then_parse_fragment(20, 20, 21, b"part-one");
    ch.reassemble_parsed(&f0).unwrap();

    assert!(
        ch.last_received > baseline,
        "fragment receive must move last_received"
    );
    assert_eq!(
        ch.last_sent, baseline,
        "fragment receive must NOT move last_sent"
    );
}

#[test]
fn reassemble_parsed_isolates_per_channel_state() {
    // Two channels with overlapping fragment seq ranges must NOT
    // share reassembly buffers — that's the whole point of putting
    // the assembler on the channel rather than the Nub.
    let mut a = Channel::new("127.0.0.1:8001".parse().unwrap());
    let mut b = Channel::new("127.0.0.1:8002".parse().unwrap());

    let a0 = build_then_parse_fragment(50, 50, 51, b"a-part-1");
    let a1 = build_then_parse_fragment(51, 50, 51, b"a-part-2");
    // b uses the SAME seq range (50..=51) — an unscoped assembler
    // would conflate b's fragments with a's, flush a partial bundle
    // early, or error on conflicting total_frags.
    let b0 = build_then_parse_fragment(50, 50, 51, b"BBB-1");

    assert!(a.reassemble_parsed(&a0).unwrap().is_none());
    // b's fragment must not affect a's pending state.
    assert!(b.reassemble_parsed(&b0).unwrap().is_none());
    let a_body = a
        .reassemble_parsed(&a1)
        .unwrap()
        .expect("a's bundle completes");
    assert_eq!(
        a_body.as_ref(),
        b"a-part-1a-part-2",
        "channel a must reassemble its own fragments without b's interference"
    );
}

/// Arrival-triggered eviction at the channel layer: a new fragmented
/// bundle whose sequence range overlaps an in-progress reassembly
/// (and keys on a different `first_seq`) evicts the older bundle.
/// The SGW client emits the matching log line at binary address
/// `0x01b18868`; per `mercury-wire-format` spec §2.4.1 R13 + §2.10 S6
/// the eviction path is the ONLY abandonment signal besides
/// channel teardown.
#[test]
fn channel_evicts_overlapping_in_progress_reassembly_on_new_bundle() {
    let mut ch = Channel::new(test_addr());

    // Bundle A: range [40..=42]. Half-arrives.
    let a0 = build_then_parse_fragment(40, 40, 42, b"abandoned");
    ch.reassemble_parsed(&a0).unwrap();

    // Bundle B: range [42..=44]. Overlaps A at seq 42. Receiving B's
    // first fragment must evict A. Completing B verifies the channel
    // doesn't reuse stale A bytes in B's reassembled output.
    let b0 = build_then_parse_fragment(42, 42, 44, b"fresh-0");
    let b1 = build_then_parse_fragment(43, 42, 44, b"fresh-1");
    let b2 = build_then_parse_fragment(44, 42, 44, b"fresh-2");
    assert!(ch.reassemble_parsed(&b0).unwrap().is_none());
    assert!(ch.reassemble_parsed(&b1).unwrap().is_none());
    let body = ch
        .reassemble_parsed(&b2)
        .unwrap()
        .expect("bundle B completes after evicting overlapping A");
    assert_eq!(body.as_ref(), b"fresh-0fresh-1fresh-2");
}

/// Inverse invariant for the removed periodic-sweep contract: an
/// orphan partial reassembly that never receives its remaining
/// fragments — and never sees an overlapping new bundle — MUST
/// persist on the channel indefinitely (until the channel itself
/// is destroyed). Pin so a future regression that re-introduces
/// any time-based eviction surfaces here.
///
/// The deleted `cleanup_stale_fragments_drops_partial_bundles` test
/// asserted the *opposite* behavior; this test pins the new contract.
#[test]
fn channel_keeps_orphan_partial_reassembly_indefinitely() {
    let mut ch = Channel::new(test_addr());
    let f0 = build_then_parse_fragment(40, 40, 42, b"only-one");
    assert!(ch.reassemble_parsed(&f0).unwrap().is_none());

    // Time passing alone must NOT reap the entry — the orphan persists.
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Subsequent re-receipt of f0 is a duplicate; the assembler
    // dedups and keeps the same partial state. Completing the bundle
    // requires the remaining fragments to arrive (which they do
    // here): f1 + f2 complete the message using the still-held f0.
    let f1 = build_then_parse_fragment(41, 40, 42, b"two");
    let f2 = build_then_parse_fragment(42, 40, 42, b"three");
    assert!(
        ch.reassemble_parsed(&f0).unwrap().is_none(),
        "f0 duplicate is deduped, partial state survives"
    );
    assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
    let body = ch
        .reassemble_parsed(&f2)
        .unwrap()
        .expect("orphan partial reassembly completes when remaining fragments arrive");
    assert_eq!(
        body.as_ref(),
        b"only-onetwothree",
        "the original f0 payload must be retained — not silently swapped",
    );
}

#[test]
fn sliding_window_rejects_overflow() {
    let mut ch = Channel::new(test_addr());

    // Fill the TX window to capacity.
    for _ in 0..consts::TX_WINDOW_SIZE {
        ch.send_packet(test_packet()).unwrap();
    }
    assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);

    // The (TX_WINDOW_SIZE + 1)-th packet must be rejected.
    let result = ch.send_packet(test_packet());
    assert!(result.is_err());

    // Window size unchanged — the rejected packet was not inserted.
    assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);
}

/// TX window must be 32 (not 45) to match the SGW client's 32-bit
/// outstanding-ack bitmap. Pin the constant value so a future regression
/// that bumps it back to 45 (or any other value > 32) re-introduces
/// the phantom-ack collision class: `seq=0` and `seq=32` would land on
/// the same bitmap bit (`seq & 0x1F == 0` for both), letting the client
/// phantom-ack both when only one arrived.
#[test]
fn tx_window_size_matches_client_bitmap_width_for_phantom_ack_safety() {
    assert_eq!(
        consts::TX_WINDOW_SIZE,
        32,
        "TX_WINDOW_SIZE must equal the SGW client's 32-bit outstanding-ack \
         bitmap width. Bumping above 32 would let two in-flight seqs \
         differing by 32 collide on the same bitmap bit."
    );
}

/// Back-pressure: registering the 33rd in-flight reliable packet must
/// fail, matching the constant pinned above. Same shape as
/// `sliding_window_rejects_overflow` but exercises the
/// `register_sent_packet` path used by the services-layer shadow
/// migration. The migration helper hits the same cap.
#[test]
fn register_sent_packet_back_pressure_at_tx_window_size() {
    let mut ch = Channel::new(test_addr());

    for seq in 0..(consts::TX_WINDOW_SIZE as u32) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::new()).unwrap();
    }
    assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);

    // 33rd registration (seq=32) must back-pressure.
    let mut pkt = test_packet();
    pkt.sequence = consts::TX_WINDOW_SIZE as u32;
    let result = ch.register_sent_packet(pkt, bytes::Bytes::new());
    assert!(result.is_err(), "33rd in-flight reliable must be rejected");
    assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);
}
