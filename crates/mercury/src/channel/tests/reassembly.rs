use super::*;
use std::net::SocketAddr;

fn test_addr() -> SocketAddr {
    "127.0.0.1:9000".parse().unwrap()
}

fn test_packet() -> Packet {
    use crate::packet::PacketFlags;
    use bytes::Bytes;

    Packet::new(PacketFlags::default(), 0, Bytes::from_static(&[0xDE, 0xAD]))
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

    // Complete channel B as well — verifies b didn't absorb a's fragments.
    let b1 = build_then_parse_fragment(51, 50, 51, b"BBB-2");
    let b_body = b
        .reassemble_parsed(&b1)
        .unwrap()
        .expect("b's bundle completes independently");
    assert_eq!(
        b_body.as_ref(),
        b"BBB-1BBB-2",
        "channel b must reassemble only its own fragments"
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

/// TX window must stay at 32 until the SGW.exe binary is patched to
/// widen the unpatched client's slot store. The client's `ChannelInternal`
/// at `ghidra://SGW.exe@0x0158c7b0` allocates a heap-resident slot hash
/// table sized from the runtime config field at `[ChannelInternal+0x2C]`,
/// defaulting to 32 in the unmodified binary; sending more than 32
/// in-flight reliable packets would let two seqs differing by 32 collide
/// on the same hash slot (mask at `[+0x44]` = `capacity - 1`) and let
/// `queueAckForPacket` at `ghidra://SGW.exe@0x0158cba0` phantom-ack the
/// older un-acked entry. (The original framing of this test as "32-bit
/// outstanding-ack bitmap" was wrong — there is no bitmap; the slot-store
/// hash-collision shape above is the corrected understanding.)
///
/// When that patch ships, this assertion can move to the next power of
/// two (likely 64 — a length-preserving 3-byte patch fits the rewrite at
/// VA `0x0158C801`) and the [`Channel::unsent_packets`] queue handles the
/// residual transient bursts past the new cap.
#[test]
fn tx_window_size_pinned_until_client_patch_widens_slot_store() {
    assert_eq!(
        consts::TX_WINDOW_SIZE,
        32,
        "TX_WINDOW_SIZE must equal the unpatched SGW client's default \
         slot-store capacity (32). Raising it before SGW.exe is patched \
         would cause the client's hash-table-indexed ack tracker to \
         phantom-ack older entries when two in-flight seqs collide on \
         the same slot."
    );
}

/// Registering the (TX_WINDOW_SIZE + 1)-th reliable packet via
/// `register_sent_packet` no longer errors — it routes to the
/// [`Channel::unsent_packets`] deferred-send queue. This replaces the
/// prior "downgrade reliable send to best-effort" path at the services
/// layer that silently lost retransmit bookkeeping for overflow packets.
#[test]
fn register_sent_packet_overflow_routes_to_unsent_queue() {
    let mut ch = Channel::new(test_addr());

    for seq in 0..(consts::TX_WINDOW_SIZE as u32) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }
    assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);
    assert!(ch.unsent_packets.is_empty(), "queue empty before overflow");

    // (TX_WINDOW_SIZE + 1)-th send must succeed AND land in the queue —
    // the bytes already went on the wire so we keep retransmit bookkeeping
    // for the case where the original send is lost.
    let mut pkt = test_packet();
    pkt.sequence = consts::TX_WINDOW_SIZE as u32;
    ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"overflow-raw"))
        .unwrap();

    assert_eq!(
        ch.tx_window.len(),
        consts::TX_WINDOW_SIZE,
        "TX window stayed at the cap — the overflow entry did not slip in"
    );
    assert_eq!(
        ch.unsent_packets.len(),
        1,
        "overflow entry was routed to the unsent-packets queue, not dropped"
    );
    assert_eq!(
        ch.unsent_packets[0].packet.sequence,
        consts::TX_WINDOW_SIZE as u32,
        "queued entry preserves the caller-allocated sequence"
    );
    assert_eq!(
        ch.unsent_packets[0].raw_bytes,
        bytes::Bytes::from_static(b"overflow-raw"),
        "queued entry preserves the on-wire bytes for future retransmit"
    );
}

/// Sequence ack covering a queued entry's seq drains it from the queue,
/// even before it has been promoted into the TX window. Queued entries
/// were sent on the wire when registered, so the peer can ack them
/// while they're still queued.
#[test]
fn acks_drain_queued_entries() {
    let mut ch = Channel::new(test_addr());

    // Fill TX window with seqs 0..32; queue holds seqs 32..40.
    for seq in 0..(consts::TX_WINDOW_SIZE as u32) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }
    for seq in consts::TX_WINDOW_SIZE as u32..(consts::TX_WINDOW_SIZE as u32 + 8) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"queued"))
            .unwrap();
    }
    assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);
    assert_eq!(ch.unsent_packets.len(), 8);

    // ACK covers everything up through the queued seqs. The drain reaches
    // into both deques, and after the TX window is empty the promote
    // step has nothing left to do.
    ch.process_acks(consts::TX_WINDOW_SIZE as u32 + 7).unwrap();
    assert!(
        ch.tx_window.is_empty(),
        "tx_window fully drained by cumulative ack"
    );
    assert!(
        ch.unsent_packets.is_empty(),
        "unsent_packets also drained by the same cumulative ack"
    );
}

/// Cumulative ack against the TX window frees slots that are then
/// filled by promoting the oldest queued entries. The total in-flight
/// count drops by exactly the number of acked seqs.
#[test]
fn acks_promote_queued_entries_into_freed_window() {
    let mut ch = Channel::new(test_addr());

    // Fill TX window with seqs 0..32; queue holds seqs 32..40.
    for seq in 0..(consts::TX_WINDOW_SIZE as u32) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }
    for seq in consts::TX_WINDOW_SIZE as u32..(consts::TX_WINDOW_SIZE as u32 + 8) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"queued"))
            .unwrap();
    }

    // ACK covers the first 8 TX-window entries (seqs 0..8). 8 slots free,
    // all 8 queued entries promote in.
    ch.process_acks(7).unwrap();
    assert_eq!(
        ch.tx_window.len(),
        consts::TX_WINDOW_SIZE,
        "TX window refilled to capacity by promoted entries"
    );
    assert!(
        ch.unsent_packets.is_empty(),
        "all queued entries promoted into the freed slots"
    );

    // Promoted entries are at the back of the window, preserving the
    // caller-allocated order.
    let promoted_seqs: Vec<u32> = ch
        .tx_window
        .iter()
        .rev()
        .take(8)
        .map(|e| e.packet.sequence)
        .rev()
        .collect();
    assert_eq!(
        promoted_seqs,
        (consts::TX_WINDOW_SIZE as u32..(consts::TX_WINDOW_SIZE as u32 + 8)).collect::<Vec<_>>(),
        "queued seqs appear at the tail of the TX window after promotion"
    );
}

/// At the [`consts::MAX_UNSENT_PACKETS`] cap the next overflow registration
/// returns an error. This is the channel-likely-dead signal — the peer
/// has stopped acking entirely, the queue cannot grow further, and the
/// inactivity timer will reap the channel shortly.
#[test]
fn unsent_queue_cap_returns_error() {
    let mut ch = Channel::new(test_addr());

    // Fill TX window first.
    for seq in 0..(consts::TX_WINDOW_SIZE as u32) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }
    // Fill the unsent queue to capacity.
    for i in 0..(consts::MAX_UNSENT_PACKETS as u32) {
        let mut pkt = test_packet();
        pkt.sequence = consts::TX_WINDOW_SIZE as u32 + i;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"queued"))
            .unwrap();
    }
    assert_eq!(ch.unsent_packets.len(), consts::MAX_UNSENT_PACKETS);

    // One past the cap: must error and not insert.
    let mut pkt = test_packet();
    pkt.sequence = consts::TX_WINDOW_SIZE as u32 + consts::MAX_UNSENT_PACKETS as u32;
    let result = ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"over-cap"));
    assert!(
        result.is_err(),
        "registration past the unsent-queue cap must error"
    );
    assert_eq!(
        ch.unsent_packets.len(),
        consts::MAX_UNSENT_PACKETS,
        "queue size unchanged — the over-cap entry was not inserted"
    );
}

/// Promoting a queued entry preserves its original `last_sent` timestamp.
/// This is the property that lets `check_timeouts` retransmit a stale
/// queued entry on the next tick after promotion — bytes went on the
/// wire at queue time, so the RTO clock should keep counting from there.
#[test]
fn promoted_entry_preserves_last_sent_for_retransmit_timing() {
    use std::time::Duration;

    let mut ch = Channel::new(test_addr());

    // Fill TX window with seqs 0..32.
    for seq in 0..(consts::TX_WINDOW_SIZE as u32) {
        let mut pkt = test_packet();
        pkt.sequence = seq;
        ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"raw"))
            .unwrap();
    }
    // Queue one entry, then backdate its `last_sent` to simulate having
    // sat in the queue for 500 ms.
    let mut pkt = test_packet();
    pkt.sequence = consts::TX_WINDOW_SIZE as u32;
    ch.register_sent_packet(pkt, bytes::Bytes::from_static(b"queued"))
        .unwrap();
    let backdated = std::time::Instant::now() - Duration::from_millis(500);
    ch.unsent_packets[0].last_sent = backdated;

    // ACK frees a window slot, queued entry promotes.
    ch.process_acks(0).unwrap();

    // Find the promoted entry (will be at the tail of the window).
    let promoted = ch
        .tx_window
        .iter()
        .find(|e| e.packet.sequence == consts::TX_WINDOW_SIZE as u32)
        .expect("queued entry was promoted into the TX window");
    assert_eq!(
        promoted.last_sent, backdated,
        "promoted entry must keep its original last_sent so the next \
         check_timeouts pass can retransmit if it sat in the queue past RTO"
    );
}
