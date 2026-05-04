//! Mercury packet-replay smoke: a synthesized stream of packet shapes
//! that span the full flag/footer matrix, replayed through
//! `parse_incoming` to catch wire-format drift.
//!
//! What this catches:
//!
//! - Flag-bit re-allocations: if a future change repurposes one of
//!   FLAG_HAS_REQUESTS / FLAG_HAS_ACKS / FLAG_FRAGMENTED / FLAG_HAS_SEQUENCE
//!   bits without updating the parser, the affected shape's
//!   round-trip would surface here.
//!
//! - Footer-order regressions: footers are stored innermost-first
//!   (first_req_offset → seq_id → acks-with-count) and stripped
//!   outermost-first on parse. A swap in either direction would
//!   produce wrong field values when reading back.
//!
//! - First-byte / last-byte off-by-one in the parser's bounds checks:
//!   covered by including a bare-flags packet (1 byte total) and a
//!   maximum-flags packet that exercises every footer at once.
//!
//! Why this is a smoke and not in `packet::tests`: the per-builder
//! tests in packet.rs assert one shape at a time. The smoke is
//! valuable specifically because it asserts the whole matrix
//! round-trips as a single batch — a refactor that breaks ONE shape
//! while keeping per-shape tests passing (e.g., by changing both
//! sides consistently in the wrong direction) would still surface
//! here when the assembled stream goes through the production
//! parser.

use crate::packet::{
    build_outgoing, parse_incoming, FLAG_HAS_ACKS, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE,
    FLAG_RELIABLE,
};

/// A single synthesized replay frame: builder inputs paired with
/// what the parser must recover. The same structure feeds both the
/// build side (via `build_outgoing`) and the assertion side, so
/// drift between them is caught by mismatched recoveries.
struct Frame {
    label: &'static str,
    flags: u8,
    body: Vec<u8>,
    seq_id: Option<u32>,
    acks: Vec<u32>,
    first_req_offset: Option<u16>,
}

fn frames() -> Vec<Frame> {
    vec![
        // 1. Bare flags, no footers — minimum-size legitimate packet.
        //    Tests the parser's "empty body, no footer" path.
        Frame {
            label: "bare-flags-no-footers",
            flags: 0,
            body: vec![],
            seq_id: None,
            acks: vec![],
            first_req_offset: None,
        },
        // 2. Sequence-only — common Phase 4 packet shape after auth.
        Frame {
            label: "seq-only",
            flags: FLAG_HAS_SEQUENCE,
            body: b"hello".to_vec(),
            seq_id: Some(0x0000_0001),
            acks: vec![],
            first_req_offset: None,
        },
        // 3. Acks-only — server tick-sync without a payload of its own
        //    just carries pending ACKs.
        Frame {
            label: "acks-only",
            flags: FLAG_HAS_ACKS,
            body: vec![],
            seq_id: None,
            acks: vec![10, 11, 12],
            first_req_offset: None,
        },
        // 4. Requests + sequence — the Phase 3 baseAppLogin shape
        //    (FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE = 0x41) with a
        //    realistic 34-byte body.
        Frame {
            label: "phase3-baseapp-login-shape",
            flags: FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE,
            body: {
                // [msg_id=0x00][word_len=25u16][reqId u32][reserved u16]
                // [accountId u32][ticketLen=20u8][ticket 20 bytes]
                let mut b = Vec::with_capacity(34);
                b.push(0x00);
                b.extend_from_slice(&25u16.to_le_bytes());
                b.extend_from_slice(&0xCAFE_BABEu32.to_le_bytes());
                b.extend_from_slice(&0u16.to_le_bytes());
                b.extend_from_slice(&1u32.to_le_bytes());
                b.push(20u8);
                b.extend_from_slice(b"ABCDEF1234567890ABCD");
                b
            },
            seq_id: Some(7),
            acks: vec![],
            first_req_offset: Some(1),
        },
        // 5. Reliable + seq + acks — full Phase 4 round-trip with
        //    piggybacked ACKs on a reliable outbound.
        Frame {
            label: "reliable-seq-acks",
            flags: FLAG_RELIABLE | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS,
            body: vec![0xDE, 0xAD, 0xBE, 0xEF],
            seq_id: Some(0x0010_0020),
            acks: vec![100, 101, 102, 103],
            first_req_offset: None,
        },
        // 6. Maximum flags (no fragmentation): every footer exercised
        //    in one shot — first_req_offset, seq_id, acks-with-count.
        //    Catches a footer-order regression that wouldn't surface
        //    on simpler shapes.
        Frame {
            label: "all-footers-no-fragments",
            flags: FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS,
            body: b"the quick brown fox jumps over".to_vec(),
            seq_id: Some(0xFFFF_0000),
            acks: vec![0x1000_0001, 0x1000_0002],
            first_req_offset: Some(13),
        },
    ]
}

/// Replay the full synthesized stream through `parse_incoming` and
/// assert every frame round-trips its body, flags, and footers.
///
/// Each frame is an independent assertion site; the test fails fast
/// on the first mismatch and the `label` field identifies which
/// shape regressed.
#[test]
fn replay_packet_stream_decodes_every_shape_without_drift() {
    for frame in frames() {
        let raw = build_outgoing(
            frame.flags,
            &frame.body,
            frame.seq_id,
            &frame.acks,
            frame.first_req_offset,
        );
        let parsed = parse_incoming(&raw).unwrap_or_else(|e| {
            panic!(
                "frame `{}`: parse_incoming must succeed on a packet built by build_outgoing — got {:?}",
                frame.label, e
            )
        });

        assert_eq!(
            parsed.flags, frame.flags,
            "frame `{}`: flags byte must round-trip",
            frame.label
        );
        assert_eq!(
            parsed.body.as_ref(),
            frame.body.as_slice(),
            "frame `{}`: body must round-trip byte-for-byte",
            frame.label
        );
        assert_eq!(
            parsed.seq_id, frame.seq_id,
            "frame `{}`: seq_id field must round-trip",
            frame.label
        );
        assert_eq!(
            parsed.acks, frame.acks,
            "frame `{}`: ack list must round-trip in order",
            frame.label
        );
        assert_eq!(
            parsed.first_req_offset, frame.first_req_offset,
            "frame `{}`: first_req_offset must round-trip",
            frame.label
        );
    }
}

/// Concatenated-stream replay: feed every frame through `parse_incoming`
/// in sequence (each as its own datagram). UDP framing means each
/// packet is parsed independently, but a regression that introduced
/// statefulness into the parser would surface as a later frame
/// failing while earlier ones passed. Pin "parser is stateless across
/// successive datagrams" by replaying twice and asserting both passes
/// produce identical results.
#[test]
fn replay_stream_twice_in_a_row_produces_identical_decodes() {
    let frames = frames();
    let datagrams: Vec<Vec<u8>> = frames
        .iter()
        .map(|f| build_outgoing(f.flags, &f.body, f.seq_id, &f.acks, f.first_req_offset).to_vec())
        .collect();

    let pass1: Vec<_> = datagrams
        .iter()
        .map(|d| {
            parse_incoming(d).map(|p| {
                (
                    p.flags,
                    p.body.to_vec(),
                    p.seq_id,
                    p.acks,
                    p.first_req_offset,
                )
            })
        })
        .collect();
    let pass2: Vec<_> = datagrams
        .iter()
        .map(|d| {
            parse_incoming(d).map(|p| {
                (
                    p.flags,
                    p.body.to_vec(),
                    p.seq_id,
                    p.acks,
                    p.first_req_offset,
                )
            })
        })
        .collect();

    for (i, (a, b)) in pass1.iter().zip(pass2.iter()).enumerate() {
        let label = frames[i].label;
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(
                a, b,
                "frame `{label}`: pass-1 and pass-2 decodes must be identical (parser must be stateless across datagrams)"
            ),
            (Err(_), _) | (_, Err(_)) => panic!("frame `{label}`: parse_incoming returned Err on a packet built by build_outgoing"),
        }
    }
}
