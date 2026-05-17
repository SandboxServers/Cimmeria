//! Unit tests for `parse_incoming` / `build_outgoing` round-trips and the
//! legacy `PacketFlags` ergonomic wrapper.

use cimmeria_common::CimmeriaError;

use super::*;

/// Helper: build then parse a round-trip.
fn round_trip(flags: u8, body: &[u8], seq_id: Option<u32>) -> ParsedPacket {
    let built = build_outgoing(flags, body, seq_id, &[], None);
    parse_incoming(&built).unwrap()
}

#[test]
fn parse_baseapp_login_packet() {
    // Construct a synthetic baseAppLogin packet (flags=0x41, 41 bytes).
    // Packet layout: [0x41][msg_id=0x00][len:u16=25][requestId:u32][nextReqOff:u16=0]
    //                [accountId:u32][ticketLen=20][ticket:20][first_req_off:u16=1][seqId:u32]
    let mut raw = Vec::new();
    raw.push(0x41u8); // flags
    raw.push(0x00u8); // msg_id
    raw.extend_from_slice(&25u16.to_le_bytes()); // WORD_LENGTH
    raw.extend_from_slice(&0xDEADBEEFu32.to_le_bytes()); // requestId
    raw.extend_from_slice(&0u16.to_le_bytes()); // nextReqOffset=0
    raw.extend_from_slice(&42u32.to_le_bytes()); // accountId
    raw.push(20u8); // ticketLen
    raw.extend_from_slice(b"12345678901234567890"); // ticket (20 bytes)
                                                    // footer:
    raw.extend_from_slice(&1u16.to_le_bytes()); // first_req_offset=1
    raw.extend_from_slice(&7u32.to_le_bytes()); // seq_id=7

    assert_eq!(raw.len(), 41);

    let pkt = parse_incoming(&raw).unwrap();
    assert_eq!(pkt.flags, 0x41);
    assert_eq!(pkt.seq_id, Some(7));
    assert_eq!(pkt.first_req_offset, Some(1));
    assert!(pkt.acks.is_empty());

    // Body should be everything between flags byte and footers.
    assert_eq!(pkt.body.len(), 34); // 1+2+4+2+4+1+20 = 34
    assert_eq!(pkt.body[0], 0x00); // msg_id
}

#[test]
fn build_and_parse_reply_packet() {
    // Server reply: flags=0x58, no requests, just seq_id footer.
    let flags = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_RELIABLE;
    let body = b"\xFF\x19\x00\x00\x00\xDE\xAD\xBE\xEF\x14"; // truncated for test

    let pkt = round_trip(flags, body, Some(1));

    assert_eq!(pkt.flags, flags);
    assert_eq!(pkt.seq_id, Some(1));
    assert_eq!(pkt.first_req_offset, None);
    assert_eq!(pkt.body.as_ref(), body);
}

#[test]
fn parse_empty_body_with_seq() {
    let flags = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL;
    let pkt = round_trip(flags, b"", Some(42));
    assert_eq!(pkt.seq_id, Some(42));
    assert!(pkt.body.is_empty());
}

#[test]
fn parse_flags_only_no_footers() {
    // A packet with no flags that add footers.
    let raw = [FLAG_PIGGYBACK]; // no seq, no acks, no requests
    let pkt = parse_incoming(&raw).unwrap();
    assert_eq!(pkt.flags, FLAG_PIGGYBACK);
    assert!(pkt.seq_id.is_none());
    assert!(pkt.body.is_empty());
}

#[test]
fn parse_too_short_fails() {
    let err = parse_incoming(&[]).unwrap_err();
    assert!(matches!(err, CimmeriaError::BufferUnderflow { .. }));
}

#[test]
fn parse_seq_truncated_fails() {
    // flags says HAS_SEQUENCE but only 3 footer bytes available
    let raw = [FLAG_HAS_SEQUENCE, 0x01, 0x02, 0x03]; // only 3 bytes after flags
                                                     // seq_id needs 4 bytes, we only have 3
    assert!(parse_incoming(&raw).is_err());
}

#[test]
fn packet_flags_operations() {
    let f = PacketFlags::default()
        .with(FLAG_RELIABLE)
        .with(FLAG_HAS_SEQUENCE);
    assert!(f.is_reliable());
    assert!(f.has_sequence());
    assert!(!f.has_acks());

    let f2 = f.without(FLAG_RELIABLE);
    assert!(!f2.is_reliable());
    assert!(f2.has_sequence());
}

/// All-flags-on packet: FLAG_RELIABLE | FLAG_HAS_ACKS | FLAG_HAS_SEQUENCE
/// with a non-empty acks vector. Pins the footer-order invariant
/// (innermost-to-outermost: first_req_offset → seq_id → acks → ack_count)
/// because the existing tests cover each footer in isolation but never
/// the full stack at once.
///
/// Asserts the EXACT raw byte sequence build_outgoing emits before
/// re-parsing, so a serializer/parser pair that both agree on a
/// wrong layout can't slip through.
#[test]
fn round_trip_reliable_with_seq_and_acks() {
    let flags = FLAG_RELIABLE | FLAG_HAS_SEQUENCE | FLAG_HAS_ACKS | FLAG_ON_CHANNEL;
    let body = b"\xAB\xCD\xEF\x12\x34";
    let acks = [101u32, 102, 103];

    let raw = build_outgoing(flags, body, Some(7), &acks, None);
    // Hand-computed wire layout:
    //   [flags:u8] [body:5] [seq_id:u32 LE = 7]
    //   [ack[0]:u32 LE = 101] [ack[1]:u32 LE = 102] [ack[2]:u32 LE = 103]
    //   [ack_count:u8 = 3]
    // No first_req_offset (FLAG_HAS_REQUESTS not set), no fragment ids.
    // Total: 1 + 5 + 4 + 4*3 + 1 = 23 bytes.
    let expected: &[u8] = &[
        flags, // flags byte
        0xAB, 0xCD, 0xEF, 0x12, 0x34, // body
        7, 0, 0, 0, // seq_id = 7
        101, 0, 0, 0, // ack[0]
        102, 0, 0, 0, // ack[1]
        103, 0, 0, 0, // ack[2]
        3, // ack_count
    ];
    assert_eq!(raw.as_ref(), expected, "byte-exact wire layout");

    let pkt = parse_incoming(&raw).unwrap();
    assert_eq!(pkt.flags, flags);
    assert_eq!(pkt.seq_id, Some(7));
    assert_eq!(pkt.acks, vec![101u32, 102, 103]);
    assert_eq!(pkt.body.as_ref(), body);
    assert!(pkt.first_req_offset.is_none());
}

/// Fragmented + reliable + acks together: a real bundle for a large
/// reliable-channel message that piggybacks acks. Pin the layout
/// because the existing fragmentation tests don't exercise the
/// `acks` slice on `build_outgoing_fragmented`. The caller is
/// responsible for setting FLAG_HAS_ACKS in the base flags;
/// build_outgoing_fragmented OR-merges FLAG_FRAGMENTED |
/// FLAG_HAS_SEQUENCE on top.
///
/// Asserts byte-exact output before re-parsing so a co-broken
/// builder/parser pair can't pass.
#[test]
fn round_trip_fragmented_with_reliable_and_acks() {
    let acks = [201u32, 202];
    let raw = build_outgoing_fragmented(
        FLAG_RELIABLE | FLAG_ON_CHANNEL | FLAG_HAS_ACKS,
        b"fragment-payload",
        500,
        500,
        502,
        &acks,
    );
    // Hand-computed wire layout:
    //   [flags:u8] [body:16] [frag_begin:u32 LE = 500]
    //   [frag_end:u32 LE = 502] [seq_id:u32 LE = 500]
    //   [ack[0]:u32 LE = 201] [ack[1]:u32 LE = 202] [ack_count:u8 = 2]
    // Total: 1 + 16 + 4 + 4 + 4 + 4*2 + 1 = 38 bytes.
    let expected_flags =
        FLAG_RELIABLE | FLAG_ON_CHANNEL | FLAG_HAS_ACKS | FLAG_FRAGMENTED | FLAG_HAS_SEQUENCE;
    let mut expected = vec![expected_flags];
    expected.extend_from_slice(b"fragment-payload");
    expected.extend_from_slice(&500u32.to_le_bytes()); // frag_begin
    expected.extend_from_slice(&502u32.to_le_bytes()); // frag_end
    expected.extend_from_slice(&500u32.to_le_bytes()); // seq_id
    expected.extend_from_slice(&201u32.to_le_bytes()); // ack[0]
    expected.extend_from_slice(&202u32.to_le_bytes()); // ack[1]
    expected.push(2); // ack_count
    assert_eq!(raw.as_ref(), expected.as_slice(), "byte-exact wire layout");

    let pkt = parse_incoming(&raw).unwrap();
    assert!(pkt.flags & FLAG_FRAGMENTED != 0);
    assert!(pkt.flags & FLAG_HAS_SEQUENCE != 0);
    assert!(pkt.flags & FLAG_RELIABLE != 0);
    assert!(pkt.flags & FLAG_HAS_ACKS != 0);
    assert_eq!(pkt.seq_id, Some(500));
    assert_eq!(pkt.frag_begin, Some(500));
    assert_eq!(pkt.frag_end, Some(502));
    assert_eq!(pkt.acks, vec![201u32, 202]);
    assert_eq!(pkt.body.as_ref(), b"fragment-payload");
}

/// Empty body with acks-only footer. The body slice must be empty
/// even though the packet has substantive footer content; previous
/// versions of the parser bled the ack-count byte into `body` when
/// the body length was 0.
#[test]
fn parse_empty_body_with_acks_only() {
    let flags = FLAG_HAS_ACKS;
    let raw = build_outgoing(flags, b"", None, &[42u32, 43], None);
    let pkt = parse_incoming(&raw).unwrap();

    assert_eq!(pkt.flags, flags);
    assert_eq!(pkt.acks, vec![42u32, 43]);
    assert!(
        pkt.body.is_empty(),
        "body must be empty when only acks are present"
    );
}

/// Full matrix: FLAG_PIGGYBACK | FLAG_HAS_ACKS | FLAG_FRAGMENTED |
/// FLAG_RELIABLE | FLAG_HAS_SEQUENCE all set together. The #123
/// "flag combination matrix" item names piggyback explicitly;
/// this round-trips the bit through build/parse so a regression
/// that strips or relocates the piggyback bit gets caught.
/// Cimmeria doesn't actually parse piggyback sub-packets (see
/// the FLAG_PIGGYBACK doc-comment in this file), so the body is
/// just the same opaque payload the other tests use; the only
/// thing this test pins beyond the existing matrix is the flag
/// bit's preservation.
#[test]
fn round_trip_with_piggyback_in_full_matrix() {
    let acks = [301u32];
    let raw = build_outgoing_fragmented(
        FLAG_RELIABLE | FLAG_ON_CHANNEL | FLAG_HAS_ACKS | FLAG_PIGGYBACK,
        b"piggy-payload",
        700,
        700,
        701,
        &acks,
    );
    let pkt = parse_incoming(&raw).unwrap();
    assert!(
        pkt.flags & FLAG_PIGGYBACK != 0,
        "piggyback bit must round-trip through build/parse"
    );
    assert!(pkt.flags & FLAG_FRAGMENTED != 0);
    assert!(pkt.flags & FLAG_HAS_SEQUENCE != 0);
    assert!(pkt.flags & FLAG_RELIABLE != 0);
    assert!(pkt.flags & FLAG_HAS_ACKS != 0);
    assert_eq!(pkt.seq_id, Some(700));
    assert_eq!(pkt.acks, vec![301u32]);
    assert_eq!(pkt.body.as_ref(), b"piggy-payload");
}

// ── Issue #292 finding #7: 28-bit sequence space + sentinel rejection ──

/// `NULL_SEQUENCE` (`0x10000000`) is the spec'd "unset" sentinel and
/// must NEVER appear on the wire as a real packet sequence. A packet
/// arriving with `seq_id = NULL_SEQUENCE` is dropped at parse with an
/// R4-class error. Per `mercury-wire-format` spec §2.4 R4.
#[test]
fn parse_incoming_rejects_null_sequence_sentinel() {
    let raw = build_outgoing(
        FLAG_HAS_SEQUENCE | FLAG_RELIABLE,
        b"hello",
        Some(NULL_SEQUENCE),
        &[],
        None,
    );
    let err = parse_incoming(&raw).expect_err("NULL_SEQUENCE must be rejected");
    let msg = format!("{err}");
    assert!(
        msg.contains("outside valid range"),
        "error must mention out-of-range sequence: {msg}"
    );
}

/// Any sequence beyond the 28-bit valid space (`> 0x0FFFFFFF`) is also
/// rejected. Pin `0xFFFFFFFE` as the issue body specifies — a hostile
/// or buggy peer could emit a u32-max-adjacent seq, and the parser
/// must drop it before it pollutes channel state.
#[test]
fn parse_incoming_rejects_out_of_range_sequence_high_bits() {
    let raw = build_outgoing(
        FLAG_HAS_SEQUENCE | FLAG_RELIABLE,
        b"hi",
        Some(0xFFFF_FFFE),
        &[],
        None,
    );
    let err = parse_incoming(&raw).expect_err("out-of-range seq must be rejected");
    let msg = format!("{err}");
    assert!(msg.contains("outside valid range"), "got: {msg}");
}

/// The maximum legal sequence (`0x0FFFFFFF`) parses successfully — the
/// boundary case, just under the sentinel. Pin so an off-by-one in the
/// rejection condition (`> NULL_SEQUENCE` vs `>= NULL_SEQUENCE`) is
/// caught.
#[test]
fn parse_incoming_accepts_max_28_bit_sequence() {
    let raw = build_outgoing(
        FLAG_HAS_SEQUENCE | FLAG_RELIABLE,
        b"max",
        Some(SEQUENCE_MASK),
        &[],
        None,
    );
    let pkt = parse_incoming(&raw).expect("0x0FFFFFFF is the highest legal seq");
    assert_eq!(pkt.seq_id, Some(SEQUENCE_MASK));
}
