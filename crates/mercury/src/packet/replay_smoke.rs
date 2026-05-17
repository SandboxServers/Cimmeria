//! Mercury packet replay smoke. The module has two shapes of test:
//!
//! 1. `replay_packet_stream_*` and `fragmented_packet_round_trips_*`
//!    feed `build_outgoing` / `build_outgoing_fragmented` output
//!    back through `parse_incoming`. These are round-trip-only —
//!    they catch a regression in EITHER side that breaks the
//!    symmetry, but a co-drift that changes both sides in the same
//!    direction (e.g., consistent footer-order swap) would round-
//!    trip cleanly.
//!
//! 2. `parser_decodes_hand_coded_byte_fixtures_per_wire_spec` is
//!    the independent oracle. Its fixtures are literal byte
//!    sequences derived from the wire-format spec at the top of
//!    `packet.rs`, NOT from `build_outgoing`. A footer-order swap
//!    in BOTH `build_outgoing` and `parse_incoming` would still
//!    fail at least one byte fixture because the fixtures encode
//!    the spec independently of either function.
//!
//! What the round-trip + oracle pair together catch:
//!
//! - Flag-bit re-allocations: repurposing one of FLAG_HAS_REQUESTS /
//!   FLAG_HAS_ACKS / FLAG_FRAGMENTED / FLAG_HAS_SEQUENCE without
//!   updating the parser; the round-trip flags assertion fires.
//! - Footer-order regressions: the byte fixtures pin the wire
//!   layout independent of the helpers, so a co-drift that swaps
//!   first_req_offset / seq_id / acks order in BOTH helpers is
//!   caught by Fixture 4 (all footers at once).
//! - First-byte / last-byte bounds: bare-flags fixture (1 byte
//!   total) exercises the parser's empty-body path; the all-
//!   footers fixture exercises the maximum offset.
//! - Fragment footer placement: Fixture 5 (FLAG_FRAGMENTED |
//!   FLAG_HAS_SEQUENCE) pins frag_begin / frag_end as innermost
//!   footers, with seq_id outside.

use super::{
    build_outgoing, build_outgoing_fragmented, parse_incoming, FLAG_FRAGMENTED, FLAG_HAS_ACKS,
    FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE,
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
            // High-but-valid 28-bit seq — exercises upper sequence bits
            // without crossing `NULL_SEQUENCE`. A value above the 28-bit
            // range (e.g. `0xFFFF_0000`) would be dropped at parse as an
            // R4 violation.
            seq_id: Some(0x0FFF_0000),
            acks: vec![0x0F00_0001, 0x0F00_0002],
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

/// Fragmented packets travel through `build_outgoing_fragmented` rather
/// than `build_outgoing` and carry an extra `frag_begin` / `frag_end`
/// footer pair (innermost). The main matrix test above intentionally
/// omits this shape so its frame struct can stay flat; pin
/// fragmentation here.
///
/// A regression that reorders or drops the frag footers (or moves
/// them outside the `seq_id` footer) would surface as a failed
/// `frag_begin` / `frag_end` round-trip.
///
/// Production fragments DO carry piggybacked acks on the first
/// fragment of a bundle — `build_fragmented_bundle` (`packet.rs`)
/// passes the full ack slice to fragment 0 and `&[]` to the rest.
/// That ack-bearing first-fragment shape is pinned by
/// `round_trip_fragmented_with_reliable_and_acks` in `packet::tests`;
/// this smoke covers the bare-fragment shape (later fragments,
/// no acks) so the two together span both halves of the bundle.
#[test]
fn fragmented_packet_round_trips_frag_footers() {
    let body = b"fragment-payload-bytes";
    let raw = build_outgoing_fragmented(
        FLAG_RELIABLE,
        body,
        0x0BCD_1234, // seq_id (28-bit; was 0xABCD_1234 before #292 #7 rejection)
        100,         // frag_begin
        103,         // frag_end (4-fragment range)
        &[],         // acks: bare-fragment shape (matches fragments 1+ of a bundle)
    );

    let parsed = parse_incoming(&raw).expect("fragmented packet must parse");
    assert!(
        parsed.flags & FLAG_FRAGMENTED != 0,
        "FLAG_FRAGMENTED bit must round-trip in flags byte"
    );
    assert!(
        parsed.flags & FLAG_HAS_SEQUENCE != 0,
        "build_outgoing_fragmented forces FLAG_HAS_SEQUENCE; bit must round-trip"
    );
    assert_eq!(parsed.body.as_ref(), body, "body must round-trip");
    assert_eq!(parsed.seq_id, Some(0x0BCD_1234), "seq_id must round-trip");
    assert_eq!(parsed.frag_begin, Some(100), "frag_begin must round-trip");
    assert_eq!(parsed.frag_end, Some(103), "frag_end must round-trip");
    assert!(
        parsed.acks.is_empty(),
        "no piggybacked acks on this fragment"
    );
}

/// Independent oracle for the parser. The `replay_packet_stream`
/// test above feeds the output of `build_outgoing` back through
/// `parse_incoming` — if both functions co-drift in the same
/// direction (e.g., swap two footers consistently on both sides),
/// the round-trip would still pass.
///
/// This test feeds **hand-coded byte sequences** into `parse_incoming`
/// directly. The expected fields are derived from the wire-format
/// spec at the top of `packet.rs`, NOT from the build helper. Any
/// drift between the spec and the parser surfaces here without
/// being papered over by a sympathetic builder regression.
///
/// Each fixture is a literal byte sequence with the layout in the
/// comment above it; the assertions check the parser's output
/// against the spec, not against another helper.
#[test]
fn parser_decodes_hand_coded_byte_fixtures_per_wire_spec() {
    // Fixture 1: bare flags=0, no body, no footers. Single byte.
    {
        let raw: &[u8] = &[0x00];
        let p = parse_incoming(raw).expect("bare flags must parse");
        assert_eq!(p.flags, 0);
        assert!(p.body.is_empty());
        assert_eq!(p.seq_id, None);
        assert_eq!(p.frag_begin, None);
        assert_eq!(p.frag_end, None);
        assert!(p.acks.is_empty());
        assert_eq!(p.first_req_offset, None);
    }

    // Fixture 2: FLAG_HAS_SEQUENCE only, body = "AB", seq_id = 0x01020304.
    // Layout: [flags=0x40][body=0x41 0x42][seq_id u32 LE = 04 03 02 01]
    {
        let raw: &[u8] = &[0x40, 0x41, 0x42, 0x04, 0x03, 0x02, 0x01];
        let p = parse_incoming(raw).expect("seq-only fixture must parse");
        assert_eq!(p.flags, FLAG_HAS_SEQUENCE);
        assert_eq!(p.body.as_ref(), b"AB");
        assert_eq!(p.seq_id, Some(0x01020304));
        assert_eq!(p.first_req_offset, None);
        assert!(p.acks.is_empty());
    }

    // Fixture 3: FLAG_HAS_ACKS only, body empty, two acks (5 then 9).
    // Layout (acks-with-count is OUTERMOST footer):
    //   [flags=0x04][acks: 05 00 00 00, 09 00 00 00][ack_count=2]
    {
        let raw: &[u8] = &[
            0x04, // flags = FLAG_HAS_ACKS
            0x05, 0x00, 0x00, 0x00, // ack[0] = 5
            0x09, 0x00, 0x00, 0x00, // ack[1] = 9
            0x02, // ack_count
        ];
        let p = parse_incoming(raw).expect("acks-only fixture must parse");
        assert_eq!(p.flags, FLAG_HAS_ACKS);
        assert_eq!(p.acks, vec![5, 9], "ack list order matches wire-byte order");
        assert!(p.body.is_empty());
        assert_eq!(p.seq_id, None);
    }

    // Fixture 4: All footers at once. flags = FLAG_HAS_REQUESTS |
    // FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS = 0x45.
    // Body = "x" (1 byte). first_req_offset = 1, seq_id = 7,
    // acks = [42, 99], ack_count = 2.
    //
    // Footer order (innermost → outermost):
    //   first_req_offset u16 LE: 01 00
    //   seq_id           u32 LE: 07 00 00 00
    //   acks             u32 LE × 2: 2A 00 00 00, 63 00 00 00
    //   ack_count        u8:    02
    {
        let raw: &[u8] = &[
            0x45, // flags
            0x78, // body byte 'x'
            0x01, 0x00, // first_req_offset = 1
            0x07, 0x00, 0x00, 0x00, // seq_id = 7
            0x2A, 0x00, 0x00, 0x00, // ack[0] = 42
            0x63, 0x00, 0x00, 0x00, // ack[1] = 99
            0x02, // ack_count
        ];
        let p = parse_incoming(raw).expect("all-footers fixture must parse");
        assert_eq!(
            p.flags,
            FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS
        );
        assert_eq!(p.body.as_ref(), b"x");
        assert_eq!(p.first_req_offset, Some(1));
        assert_eq!(p.seq_id, Some(7));
        assert_eq!(p.acks, vec![42, 99]);
    }

    // Fixture 5: Fragmented packet. flags = FLAG_FRAGMENTED |
    // FLAG_HAS_SEQUENCE = 0x60. Body = "Q" (1 byte).
    // frag_begin = 5, frag_end = 8, seq_id = 0x10.
    //
    // Footer order (innermost → outermost):
    //   frag_begin u32 LE: 05 00 00 00
    //   frag_end   u32 LE: 08 00 00 00
    //   seq_id     u32 LE: 10 00 00 00
    {
        let raw: &[u8] = &[
            0x60, // flags = FLAG_FRAGMENTED | FLAG_HAS_SEQUENCE
            0x51, // body 'Q'
            0x05, 0x00, 0x00, 0x00, // frag_begin = 5
            0x08, 0x00, 0x00, 0x00, // frag_end   = 8
            0x10, 0x00, 0x00, 0x00, // seq_id     = 16
        ];
        let p = parse_incoming(raw).expect("fragmented fixture must parse");
        assert_eq!(p.flags, FLAG_FRAGMENTED | FLAG_HAS_SEQUENCE);
        assert_eq!(p.body.as_ref(), b"Q");
        assert_eq!(p.frag_begin, Some(5));
        assert_eq!(p.frag_end, Some(8));
        assert_eq!(p.seq_id, Some(0x10));
    }
}
