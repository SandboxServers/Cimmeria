//! Tests for [`ChannelBundle`]. A child of the `bundle` module so the
//! byte-exact assertions can read the private `body` buffer directly.

use super::*;
use crate::channel_bundle::{IDBASE_NPC_DEFAULT, IDBASE_SGW_PLAYER};
use crate::packet::{
    parse_incoming, FLAG_FRAGMENTED, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE,
};

// ── Per-idbase encoder pins ─────────────────────────────────────────

/// Method index 60 always lands in direct encoding regardless of
/// idbase — well below both SGWPlayer's 61 and any NPC's 62.
#[test]
fn encoder_method_60_direct_for_both_idbases() {
    for &idbase in &[IDBASE_SGW_PLAYER, IDBASE_NPC_DEFAULT] {
        let mut bundle = ChannelBundle::new(true);
        bundle.append_entity_method(60, idbase, 1, &[]);
        let (packets, _) = bundle.finalize(0, 0, passthrough);
        // First byte after the flags is the msg_id.
        assert_eq!(
            packets[0][1],
            0x80 | 60,
            "method 60 must direct-encode (msg_id = 0xBC) for idbase {idbase}",
        );
    }
}

/// Method index 61 for SGWPlayer extended-encodes (marker `0xBD`,
/// sub_index 0) — issue #315 worked example.
#[test]
fn encoder_method_61_extended_for_sgw_player() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(61, IDBASE_SGW_PLAYER, 7, &[]);
    let (packets, _) = bundle.finalize(0, 0, passthrough);
    assert_eq!(
        packets[0][1], EXTENDED_ENCODING_MARKER,
        "method 61 must extended-encode under SGWPlayer's idbase=61"
    );
    // Wire after marker: u16 word_len = 5, u32 entity_id = 7, u8 sub_index = 0.
    assert_eq!(u16::from_le_bytes([packets[0][2], packets[0][3]]), 5);
    assert_eq!(
        u32::from_le_bytes([packets[0][4], packets[0][5], packets[0][6], packets[0][7]]),
        7
    );
    assert_eq!(packets[0][8], 0, "sub_index = 61 - 61 = 0");
}

/// Method index 61 for an NPC (idbase=62) takes the DIRECT path — the
/// wire byte is `0xBD` (same byte as the SGWPlayer extended marker)
/// but as a direct msg_id, NOT a sub-slot trigger. This is the
/// per-entity divergence the threshold parameterisation makes visible.
/// If encoded with the wrong idbase the client would dispatch to a
/// different method.
#[test]
fn encoder_method_61_direct_for_npc_default_idbase() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(61, IDBASE_NPC_DEFAULT, 7, &[]);
    let (packets, _) = bundle.finalize(0, 0, passthrough);
    // Same wire byte (0xBD), DIFFERENT semantics: direct, not marker.
    // Body has NO sub_index byte — payload is just the entity_id.
    assert_eq!(
        packets[0][1],
        0x80 | 61,
        "method 61 must direct-encode under NPC idbase=62 — wire byte is 0xBD as a direct msg_id"
    );
    assert_eq!(
        u16::from_le_bytes([packets[0][2], packets[0][3]]),
        4,
        "direct encoding payload is entity_id only (4 bytes)"
    );
    assert_eq!(
        u32::from_le_bytes([packets[0][4], packets[0][5], packets[0][6], packets[0][7]]),
        7,
        "entity_id immediately follows the length prefix — no sub_index"
    );
}

/// SGWPlayer method 156 (one below the 157-method cap; near the top
/// of the range) extended-encodes with sub_index = 156 - 61 = 95.
#[test]
fn encoder_method_156_extended_sub_index_95_for_sgw_player() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(156, IDBASE_SGW_PLAYER, 1, &[]);
    let (packets, _) = bundle.finalize(0, 0, passthrough);
    assert_eq!(packets[0][1], EXTENDED_ENCODING_MARKER);
    assert_eq!(packets[0][8], 95, "sub_index = 156 - 61 = 95");
}

/// setupWorldParameters (SGWPlayer method 122) — audit Appendix C.6
/// wire-capture confirms this encodes as [0xBD][len][entity_id][61].
/// Named landmark method (its method-index 122 == idbase 61 + sub_index
/// 61, the only such alignment in the SGWPlayer schema).
#[test]
fn encoder_setup_world_parameters_122_extended_sub_index_61() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(122, IDBASE_SGW_PLAYER, 1, &[]);
    let (packets, _) = bundle.finalize(0, 0, passthrough);
    assert_eq!(packets[0][1], EXTENDED_ENCODING_MARKER);
    assert_eq!(packets[0][8], 61, "sub_index = 122 - 61 = 61");
}

/// Identity encrypt for tests — packet bytes pass through unmodified
/// so `parse_incoming` can verify the wire layout without needing
/// session-key plumbing.
fn passthrough(plaintext: &[u8]) -> Vec<u8> {
    plaintext.to_vec()
}

/// Pin a representative direct-encoding entity-method append against
/// the wire layout the services-layer `append_entity_method` produces.
/// If this drifts, any migrated call site would emit different bytes
/// than the pre-migration packet builders and the client would
/// silently fail to dispatch.
///
/// Method index 12, entity_id 0xDEAD_BEEF, args [0xAA, 0xBB] → direct
/// encoding: `[0x8C][0x06 0x00][0xEF 0xBE 0xAD 0xDE][0xAA 0xBB]`.
#[test]
fn append_entity_method_direct_encoding_matches_services_layer_byte_for_byte() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(12, IDBASE_SGW_PLAYER, 0xDEAD_BEEF, &[0xAA, 0xBB]);

    // Body should be exactly the bytes the services-layer
    // append_entity_method would emit for the same inputs.
    let expected: &[u8] = &[
        0x8C, // 12 | 0x80
        0x06, 0x00, // word_len = 4 (entity_id) + 2 (args) = 6, u16 LE
        0xEF, 0xBE, 0xAD, 0xDE, // entity_id 0xDEAD_BEEF, u32 LE
        0xAA, 0xBB, // args
    ];
    assert_eq!(
        bundle.body, expected,
        "direct-encoding wire layout must match services-layer append_entity_method"
    );
}

/// Same byte-for-byte pin for the extended encoding path (index ≥ 61).
/// Method index 122 (setupWorldParameters), entity_id 1, no args:
/// `[0xBD][0x05 0x00][0x01 0x00 0x00 0x00][122 - 61 = 61]`.
#[test]
fn append_entity_method_extended_encoding_matches_services_layer_byte_for_byte() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(122, IDBASE_SGW_PLAYER, 1, &[]);

    let expected: &[u8] = &[
        0xBD, // extended marker
        0x05, 0x00, // word_len = 4 (entity_id) + 1 (sub_index) = 5, u16 LE
        0x01, 0x00, 0x00, 0x00, // entity_id = 1, u32 LE
        61,   // sub_index = 122 - 61
    ];
    assert_eq!(
        bundle.body, expected,
        "extended-encoding wire layout must match services-layer append_entity_method"
    );
}

/// Boundary check: index 60 stays direct, index 61 flips to extended.
/// A regression that moved the boundary would silently break either
/// the high direct/extended boundary (e.g. 122 = setupWorldParameters
/// is extended in this implementation) or the low extended ones.
#[test]
fn append_entity_method_boundary_between_direct_and_extended_encoding() {
    let mut direct = ChannelBundle::new(false);
    direct.append_entity_method(60, IDBASE_SGW_PLAYER, 1, &[]);
    assert_eq!(
        direct.body[0],
        60 | 0x80,
        "index 60 must use direct encoding"
    );

    let mut extended = ChannelBundle::new(false);
    extended.append_entity_method(61, IDBASE_SGW_PLAYER, 1, &[]);
    assert_eq!(
        extended.body[0], EXTENDED_ENCODING_MARKER,
        "index 61 must flip to extended encoding"
    );
}

/// Pack 5 different cross-entity messages into one bundle. After
/// finalize, all 5 must land in a single packet (sub-fragment-size
/// total) and the packet body must contain all 5 message records in
/// append order. This is the core "N call sites collapse into 1 UDP
/// datagram" promise of the bundle abstraction.
#[test]
fn bundle_packs_multiple_cross_entity_messages_into_one_packet() {
    let mut bundle = ChannelBundle::new(true);
    for (i, entity_id) in [10u32, 20, 30, 40, 50].iter().enumerate() {
        bundle.append_entity_method(12, IDBASE_SGW_PLAYER, *entity_id, &[i as u8]);
    }
    assert_eq!(bundle.num_messages(), 5);
    assert!(
        bundle.body_len() < FRAGMENT_BODY_SIZE,
        "5 tiny messages must fit in one fragment for this test to be meaningful"
    );

    let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 100, passthrough);
    assert_eq!(packets.len(), 1, "5 small messages collapse to 1 packet");
    assert_eq!(seqs_consumed, 1, "1 packet consumes 1 sequence id");

    // Parse the packet to confirm the body carries all 5 records.
    // Each direct-encoded record is 1 (msg_id) + 2 (len) + 4 (entity_id)
    // + 1 (arg) = 8 bytes; 5 records → 40 body bytes.
    let parsed = parse_incoming(&packets[0]).expect("packet parses");
    assert_eq!(
        parsed.body.len(),
        5 * 8,
        "body carries all 5 message records"
    );
    assert_eq!(parsed.seq_id, Some(100), "packet seq matches base_seq");
    assert_eq!(
        parsed.flags & FLAG_HAS_SEQUENCE,
        FLAG_HAS_SEQUENCE,
        "single-packet path sets FLAG_HAS_SEQUENCE"
    );
    assert_eq!(
        parsed.flags & FLAG_FRAGMENTED,
        0,
        "single-packet path must NOT set FLAG_FRAGMENTED"
    );
}

/// When the bundle body exceeds [`FRAGMENT_BODY_SIZE`] (1300 bytes),
/// finalize must emit multiple fragmented packets. Build a body of
/// ~3KB (3 fragments worth), finalize, and assert 3 fragments — each
/// flagged FLAG_FRAGMENTED, with consecutive sequence numbers and
/// matching frag_begin/frag_end footers.
#[test]
fn bundle_fragments_when_body_exceeds_packet_capacity() {
    let mut bundle = ChannelBundle::new(true);
    // Each append: 1 (msg_id) + 2 (len) + 4 (entity_id) + 100 (args) = 107 bytes.
    // 30 appends ≈ 3210 bytes → 3 fragments (1300 + 1300 + 610).
    let args = [0xAB; 100];
    for entity_id in 0u32..30 {
        bundle.append_entity_method(12, IDBASE_SGW_PLAYER, entity_id, &args);
    }
    assert!(
        bundle.body_len() > 2 * FRAGMENT_BODY_SIZE,
        "test setup must exceed 2 fragments"
    );
    let expected_frags = bundle.estimated_packet_count();

    let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 200, passthrough);
    assert_eq!(
        packets.len(),
        expected_frags,
        "fragment count matches estimate"
    );
    assert_eq!(
        seqs_consumed, expected_frags as u32,
        "each fragment consumes one sequence id"
    );

    for (i, raw) in packets.iter().enumerate() {
        let parsed = parse_incoming(raw).expect("fragment parses");
        assert_eq!(
            parsed.flags & FLAG_FRAGMENTED,
            FLAG_FRAGMENTED,
            "fragment {i} must set FLAG_FRAGMENTED"
        );
        assert_eq!(parsed.seq_id, Some(200 + i as u32), "fragment {i} seq");
        assert_eq!(
            parsed.frag_begin,
            Some(200),
            "all fragments share frag_begin"
        );
        assert_eq!(
            parsed.frag_end,
            Some(200 + expected_frags as u32 - 1),
            "all fragments share frag_end"
        );
    }
}

/// ACK piggyback rides ONLY the first fragment of a multi-packet
/// bundle — matches the C++ Bundle and the wire-format spec. A
/// regression that put acks on every fragment would inflate the
/// per-fragment footer size AND duplicate ACK delivery to the peer's
/// ack-coverage tracker.
#[test]
fn finalized_acks_ride_only_the_first_fragment() {
    let mut bundle = ChannelBundle::new(true);
    bundle.add_acks(&[10, 20, 30]);
    // Force fragmentation so we have multiple packets to inspect.
    let args = [0xCD; 100];
    for entity_id in 0u32..30 {
        bundle.append_entity_method(12, IDBASE_SGW_PLAYER, entity_id, &args);
    }

    let (packets, _) = bundle.finalize(FLAG_RELIABLE, 500, passthrough);
    assert!(packets.len() > 1, "test must exercise multi-fragment path");

    let first = parse_incoming(&packets[0]).expect("first fragment parses");
    assert_eq!(
        first.flags & FLAG_HAS_ACKS,
        FLAG_HAS_ACKS,
        "first fragment must carry FLAG_HAS_ACKS"
    );
    assert_eq!(
        first.acks,
        vec![10, 20, 30],
        "first fragment carries all acks"
    );

    for (i, raw) in packets.iter().enumerate().skip(1) {
        let parsed = parse_incoming(raw).expect("fragment parses");
        assert_eq!(
            parsed.flags & FLAG_HAS_ACKS,
            0,
            "fragment {i} must NOT carry FLAG_HAS_ACKS"
        );
        assert!(parsed.acks.is_empty(), "fragment {i} has no acks");
    }
}

/// Empty bundle finalize must emit zero packets (and consume zero
/// sequence ids). A regression that emitted an empty seq-only packet
/// would waste a TX-window slot on a no-op flush.
#[test]
fn empty_bundle_finalize_emits_zero_packets() {
    let bundle = ChannelBundle::new(true);
    assert!(bundle.is_empty(), "new bundle is empty");
    assert_eq!(bundle.estimated_packet_count(), 0);

    let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 999, passthrough);
    assert!(packets.is_empty(), "empty bundle emits no packets");
    assert_eq!(seqs_consumed, 0, "empty bundle consumes no seqs");
}

/// Ack-only bundle (no message body, but pending acks) must emit
/// exactly one packet so the acks reach the peer. Without this path
/// the channel could accumulate pending acks indefinitely if no
/// outgoing application message coincided with the next flush.
#[test]
fn ack_only_bundle_finalize_emits_one_packet() {
    let mut bundle = ChannelBundle::new(true);
    bundle.add_acks(&[7, 8, 9]);
    assert!(!bundle.is_empty(), "ack-only bundle is not empty");
    assert_eq!(bundle.estimated_packet_count(), 1);

    let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 42, passthrough);
    assert_eq!(packets.len(), 1, "ack-only bundle emits one packet");
    assert_eq!(seqs_consumed, 1, "ack-only packet consumes one seq");

    let parsed = parse_incoming(&packets[0]).expect("packet parses");
    assert_eq!(parsed.acks, vec![7, 8, 9], "acks reach the peer");
    assert_eq!(
        parsed.seq_id,
        Some(42),
        "ack-only packet carries the allocated seq"
    );
}

/// Pin the exact bytes ChannelBundle produces for a representative
/// 2-message bundle against an equivalent manual construction via
/// the services-layer `append_entity_method` + `build_outgoing`.
/// Byte-exact equality means migrating a single call site never
/// changes what the client sees.
///
/// This is the load-bearing regression guard for the migration: if
/// it ever fails, a Layer B migration cannot be byte-equivalent
/// without a corresponding wire-format change elsewhere.
#[test]
fn bundle_body_byte_exact_against_manual_construction() {
    // Bundle path
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(12, IDBASE_SGW_PLAYER, 0x0000_0042, &[0x11, 0x22, 0x33]);
    bundle.append_entity_method(141, IDBASE_SGW_PLAYER, 0x0000_0099, &[0xFE, 0xED]);
    let bundle_body = bundle.body.clone();

    // Manual path — same wire format the services-layer builders use.
    let mut manual_body: Vec<u8> = Vec::new();
    // Message 1: direct encoding, index 12.
    manual_body.push(0x8C);
    manual_body.extend_from_slice(&(4u16 + 3).to_le_bytes());
    manual_body.extend_from_slice(&0x0000_0042u32.to_le_bytes());
    manual_body.extend_from_slice(&[0x11, 0x22, 0x33]);
    // Message 2: extended encoding, index 141.
    manual_body.push(EXTENDED_ENCODING_MARKER);
    manual_body.extend_from_slice(&(4u16 + 1 + 2).to_le_bytes());
    manual_body.extend_from_slice(&0x0000_0099u32.to_le_bytes());
    manual_body.push((141 - 61) as u8);
    manual_body.extend_from_slice(&[0xFE, 0xED]);

    assert_eq!(
        bundle_body, manual_body,
        "ChannelBundle body must be byte-identical to manual append_entity_method composition"
    );
}

/// Verify the AoI-burst shape that the conservative migration in
/// Layer B targets: 28 NPC phase-1 packets (CREATE_ENTITY +
/// UPDATE_AVATAR per NPC, ~50 bytes each via raw append) collapse
/// into a small number of fragments.
///
/// 28 × 50 = 1400 bytes → 2 fragments (1300 + 100). This pins the
/// expected fragment count so the Layer B migration test can assert
/// the same shape.
#[test]
fn aoi_burst_shape_28_npcs_collapse_to_two_fragments() {
    let mut bundle = ChannelBundle::new(true);
    // Mock per-NPC phase 1: CREATE_ENTITY (12 bytes) + UPDATE_AVATAR
    // (33 bytes) ≈ 45 bytes (close to the real shape in
    // crates/services/src/mercury/aoi/create.rs).
    let phase1_per_npc = vec![0xAB; 50];
    for _ in 0..28 {
        bundle.append_raw_message(&phase1_per_npc);
    }
    assert_eq!(
        bundle.body_len(),
        28 * 50,
        "28 NPCs × 50 bytes/phase-1 = 1400 bytes of body"
    );
    let estimated = bundle.estimated_packet_count();
    assert_eq!(
        estimated, 2,
        "28-NPC AoI burst should collapse to 2 fragments (was 28 packets pre-bundle)"
    );

    let (packets, seqs) = bundle.finalize(FLAG_RELIABLE, 1000, passthrough);
    assert_eq!(packets.len(), 2);
    assert_eq!(seqs, 2);
}

/// `num_acks` counts only ACKs, independent of appended messages, and
/// tracks both single (`add_ack`) and batched (`add_acks`) additions.
/// A fresh bundle reports zero. This accessor backs caller-side
/// "do we even have acks to flush?" checks.
#[test]
fn num_acks_counts_acks_independently_of_messages() {
    let mut bundle = ChannelBundle::new(true);
    assert_eq!(bundle.num_acks(), 0, "fresh bundle has no acks");

    // Appending a message must not change the ack count.
    bundle.append_entity_method(12, IDBASE_SGW_PLAYER, 1, &[]);
    assert_eq!(bundle.num_acks(), 0, "message append does not add acks");

    bundle.add_ack(5);
    assert_eq!(bundle.num_acks(), 1, "single add_ack bumps the count");

    bundle.add_acks(&[6, 7, 8]);
    assert_eq!(
        bundle.num_acks(),
        4,
        "add_acks appends each ack to the running count"
    );

    // num_messages is tracked separately and is unaffected by acks.
    assert_eq!(bundle.num_messages(), 1);
}

/// `is_reliable` reports back the constructor argument unmodified —
/// it's metadata the caller consults to drive its own
/// FLAG_RELIABLE-in-base_flags decision.
#[test]
fn is_reliable_returns_constructor_argument() {
    assert!(ChannelBundle::new(true).is_reliable());
    assert!(!ChannelBundle::new(false).is_reliable());
    assert!(!ChannelBundle::default().is_reliable());
}

/// Pin the `estimated_packet_count == finalize().packets.len()`
/// contract at the fragment boundary (`body_len == FRAGMENT_BODY_SIZE`)
/// with ACKs present — historically the first place an estimate vs
/// actual-emission drift would surface. The `send_bundle_to_witness_reliable`
/// helper reserves seqs based on the estimate before calling finalize;
/// if they ever disagree it would over-/under-reserve and leave gaps
/// in the reliable stream.
#[test]
fn estimated_packet_count_matches_finalize_at_fragment_boundary_with_acks() {
    let mut bundle = ChannelBundle::new(true);
    let body = vec![0xAB; FRAGMENT_BODY_SIZE];
    bundle.append_raw_message(&body);
    bundle.add_ack(7);

    let estimated = bundle.estimated_packet_count();
    let (packets, seqs_consumed) = bundle.finalize(FLAG_RELIABLE, 77, passthrough);

    assert_eq!(
        packets.len(),
        estimated,
        "exactly-at-boundary body must collapse to one packet \
         (FRAGMENT_BODY_SIZE fits in a single fragment)"
    );
    assert_eq!(seqs_consumed as usize, estimated);
    assert_eq!(estimated, 1, "body == FRAGMENT_BODY_SIZE → 1 packet");
}

/// Pin the same contract one byte over the boundary — the first
/// case that MUST fragment. `body_len == FRAGMENT_BODY_SIZE + 1`
/// rounds up to 2 packets.
#[test]
fn estimated_packet_count_matches_finalize_one_byte_over_boundary() {
    let mut bundle = ChannelBundle::new(true);
    let body = vec![0xCD; FRAGMENT_BODY_SIZE + 1];
    bundle.append_raw_message(&body);

    let estimated = bundle.estimated_packet_count();
    let (packets, _) = bundle.finalize(FLAG_RELIABLE, 0, passthrough);
    assert_eq!(packets.len(), estimated);
    assert_eq!(estimated, 2, "1 byte over boundary forces 2 fragments");
}

/// `append_raw_message` is the catch-all path the AoI burst migration
/// uses to bundle multi-message packet bodies (CREATE_ENTITY +
/// UPDATE_AVATAR pairs, cascade per NPC). It must preserve arbitrary
/// bytes exactly — no length prefix, no msg-id wrapping, no padding.
///
/// A regression that added a length prefix or framing byte to
/// `append_raw_message` would silently corrupt every migrated
/// caller's wire bytes.
#[test]
fn append_raw_message_preserves_arbitrary_bytes_in_order() {
    let mut bundle = ChannelBundle::new(true);

    // Append three differently-shaped raw messages — a tight little
    // CREATE_ENTITY shape, a UPDATE_AVATAR-shaped block, and a
    // synthetic extended-encoding shape — and assert the bundle body
    // is exactly their concatenation in append order.
    let create_entity_shape: &[u8] = &[
        0x09, // BASEMSG_CREATE_ENTITY
        0x08, 0x00, // wordLen = 8
        0xEF, 0xBE, 0xAD, 0xDE, // entity_id
        0xFF, 0x42, 0x00, 0x00, // idAlias + class_id + zeros
    ];
    let update_avatar_shape: &[u8] = &[
        0x10, // BASEMSG_UPDATE_AVATAR
        0xEF, 0xBE, 0xAD, 0xDE, // entity_id
        0x00, 0x00, 0x20,
        0x41, // pos.x = 10.0 f32 LE
              // … rest of UPDATE_AVATAR is byte-after-byte fine to elide
    ];
    let extended_shape: &[u8] = &[
        EXTENDED_ENCODING_MARKER,
        0x05,
        0x00, // payload_len = 5
        0x01,
        0x00,
        0x00,
        0x00, // entity_id = 1
        61,   // sub_index
    ];

    bundle.append_raw_message(create_entity_shape);
    bundle.append_raw_message(update_avatar_shape);
    bundle.append_raw_message(extended_shape);

    let mut expected = Vec::new();
    expected.extend_from_slice(create_entity_shape);
    expected.extend_from_slice(update_avatar_shape);
    expected.extend_from_slice(extended_shape);
    assert_eq!(
        bundle.body, expected,
        "append_raw_message must preserve every byte in append order, \
         with no framing or padding inserted"
    );
    assert_eq!(bundle.num_messages(), 3);
}

/// Field-width contract: extended-encoding sub_index overflow MUST
/// panic (not silently truncate to garbage). A `method_index = 317`
/// (== 61 + 256) exceeds the single-byte sub_index range; the panic
/// message should name the violated invariant clearly.
#[test]
#[should_panic(expected = "extended-encoding range")]
fn append_entity_method_panics_on_extended_sub_index_overflow() {
    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(
        u16::from(IDBASE_SGW_PLAYER) + 256,
        IDBASE_SGW_PLAYER,
        1,
        &[],
    );
}

/// Field-width contract: payload_len overflow MUST panic. A 65_536-byte
/// args buffer exceeds the u16 length field by 1 byte (after adding
/// the 4-byte entity_id, it's well over u16::MAX). The panic message
/// should name the violated invariant clearly.
#[test]
#[should_panic(expected = "u16 length field")]
fn append_entity_method_panics_on_payload_length_overflow() {
    let mut bundle = ChannelBundle::new(true);
    let huge_args = vec![0u8; u16::MAX as usize - 3]; // 4 + (u16::MAX - 3) > u16::MAX
    bundle.append_entity_method(12, IDBASE_SGW_PLAYER, 1, &huge_args);
}
