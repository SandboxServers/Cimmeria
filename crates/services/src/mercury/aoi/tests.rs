//! Wire-layout tests for the AoI builder family.

use super::*;
use cimmeria_mercury::encryption::MercuryEncryption;

const TEST_KEY: [u8; 32] = [0x42u8; 32];

/// `build_forced_position` must produce the exact byte layout the BigWorld
/// engine expects on the client side. The layout matches `build_enter_world_body`
/// in world_data/phases.rs — same message id, same field order, same flags
/// byte. Mismatch here is the difference between the avatar snapping and
/// staying put.
#[test]
fn forced_position_wire_layout() {
    // Caller passes a non-zero prev_pos distinct from `position` so the
    // byte slot (offsets 24-35 of the 49-byte payload, decrypted offsets
    // 26-38) is observable — emitting zeros there causes the camera to
    // slide from world origin (0,0,0) through the floor to the spawn
    // point. See `spec.protocol.mercury` §1.10.6 + §1.16 Q3 closure.
    let pkt = build_forced_position(
        &TEST_KEY,
        1,
        &[],
        0x12345678,
        0x0001_0010,
        [10.0, 20.0, 30.0],
        [9.0, 19.0, 29.0],
    );
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();

    // body starts at offset 1 (offset 0 is flags).
    assert_eq!(pt[1], crate::mercury::BASEMSG_FORCED_POSITION, "msg id");
    // entity_id LE
    assert_eq!(&pt[2..6], &0x12345678u32.to_le_bytes());
    // space_id LE
    assert_eq!(&pt[6..10], &0x00010010u32.to_le_bytes());
    // vehicleID = 0
    assert_eq!(&pt[10..14], &0u32.to_le_bytes());
    // pos x/y/z
    assert_eq!(&pt[14..18], &10.0f32.to_le_bytes());
    assert_eq!(&pt[18..22], &20.0f32.to_le_bytes());
    assert_eq!(&pt[22..26], &30.0f32.to_le_bytes());
    // prev_pos x/y/z (offsets 24-35 of the 49-byte payload — prev-position
    // reference, NOT velocity).
    assert_eq!(&pt[26..30], &9.0f32.to_le_bytes(), "prev_pos.x");
    assert_eq!(&pt[30..34], &19.0f32.to_le_bytes(), "prev_pos.y");
    assert_eq!(&pt[34..38], &29.0f32.to_le_bytes(), "prev_pos.z");
    // rotation = 0,0,0 (12 zero bytes)
    assert_eq!(&pt[38..50], &[0u8; 12]);
    // flags = 0x01
    assert_eq!(pt[50], 0x01, "flags");
}

/// `build_create_entity_base` emits CREATE_ENTITY (0x09) immediately
/// followed by UPDATE_AVATAR (0x10) in a single packet — the client
/// expects the create + initial pose to arrive together so the entity
/// is fully positioned before the cascade configures it.
#[test]
fn create_entity_base_wire_layout() {
    let pkt = build_create_entity_base(
        &TEST_KEY,
        1,
        &[],
        0xDEADBEEF,
        0x42,
        [10.0, 20.0, 30.0],
        [0.0, 0.0, 0.0],
    );
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();

    // Body starts at offset 1 (offset 0 is flags).
    // CREATE_ENTITY: [0x09][word_len=8 LE][entity_id u32][0xFF][class_id][0x00][0x00]
    assert_eq!(pt[1], BASEMSG_CREATE_ENTITY);
    assert_eq!(
        u16::from_le_bytes([pt[2], pt[3]]),
        8,
        "CREATE_ENTITY wordLength"
    );
    assert_eq!(&pt[4..8], &0xDEADBEEFu32.to_le_bytes());
    assert_eq!(pt[8], 0xFF, "idAlias = no alias");
    assert_eq!(pt[9], 0x42, "class_id");
    assert_eq!(&pt[10..12], &[0x00, 0x00], "two trailing zero bytes");

    // UPDATE_AVATAR begins at offset 12: [0x10][entity_id u32][pos 3×f32]
    // [vel 5 zero bytes][0x01 physics mode][yaw][pitch][roll]
    assert_eq!(pt[12], BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
    assert_eq!(&pt[13..17], &0xDEADBEEFu32.to_le_bytes());
    assert_eq!(&pt[17..21], &10.0f32.to_le_bytes());
    assert_eq!(&pt[21..25], &20.0f32.to_le_bytes());
    assert_eq!(&pt[25..29], &30.0f32.to_le_bytes());
    assert_eq!(
        &pt[29..34],
        &[0u8; 5],
        "velocity bytes (zero on initial pose)"
    );
    assert_eq!(pt[34], 0x01, "physics mode");
    // yaw/pitch/roll are direction[1]/[0]/[2] — all zero direction packs to 0
    assert_eq!(
        &pt[35..38],
        &[0u8; 3],
        "yaw/pitch/roll = 0 for zero direction"
    );

    // Body length = 11 (CREATE_ENTITY) + 26 (UPDATE_AVATAR) = 37 bytes,
    // occupying pt[1..38]. With FLAG_HAS_SEQUENCE the seq_id (we passed
    // 1) is appended as a u32 footer immediately after the body, so
    // pt[38..42] must equal the seq_id LE bytes. Asserting that pins
    // "the body is exactly 37 bytes" — a layout drift would shift the
    // footer and fail this check.
    assert_eq!(
        u32::from_le_bytes([pt[38], pt[39], pt[40], pt[41]]),
        1,
        "seq_id footer must start at pt[38] (proves body length = 37)",
    );
}

/// `build_entity_invisible` is the smallest AoI builder — 6-byte body.
/// Used for ring-transport teleport-out fades where the entity should
/// vanish but not be deleted from the client's entity table.
#[test]
fn entity_invisible_wire_layout() {
    let pkt = build_entity_invisible(&TEST_KEY, 1, &[], 0x12345678);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();

    // [flags][0x0B][entity_id u32][0xFF]
    assert_eq!(pt[1], BASEMSG_ENTITY_INVISIBLE);
    assert_eq!(&pt[2..6], &0x12345678u32.to_le_bytes());
    assert_eq!(pt[6], 0xFF, "idAlias = no alias");
}

/// `build_entity_leave` emits ENTITY_INVISIBLE (0x0B) then LEAVE_AOI (0x0C)
/// in a single packet — invisible-first prevents a one-frame "ghost"
/// flicker as the client tears down the entity. Both records must reach
/// the wire in this order.
#[test]
fn entity_leave_emits_invisible_then_leave_aoi() {
    let pkt = build_entity_leave(&TEST_KEY, 1, &[], 0xCAFEF00D);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();

    // ENTITY_INVISIBLE first: [0x0B][entity_id u32][0xFF]
    assert_eq!(pt[1], BASEMSG_ENTITY_INVISIBLE);
    assert_eq!(&pt[2..6], &0xCAFEF00Du32.to_le_bytes());
    assert_eq!(pt[6], 0xFF);

    // LEAVE_AOI next at offset 7:
    //   [0x0C][word_len=8 LE][entity_id u32][cacheStamp=0 u32]
    assert_eq!(pt[7], BASEMSG_LEAVE_AOI);
    assert_eq!(
        u16::from_le_bytes([pt[8], pt[9]]),
        8,
        "LEAVE_AOI wordLength"
    );
    assert_eq!(&pt[10..14], &0xCAFEF00Du32.to_le_bytes());
    assert_eq!(&pt[14..18], &0u32.to_le_bytes(), "cacheStamp = 0");
}

/// `build_avatar_update` is the per-tick AoI position relay for ghost
/// entities. Wire layout is fixed-size 26 bytes after the msg id, so any
/// drift in field order desyncs all subsequent updates in the same packet.
#[test]
fn avatar_update_wire_layout() {
    let pkt = build_avatar_update(
        &TEST_KEY,
        1,
        &[],
        0x00ABCDEF,
        [100.0, 200.0, 300.0],
        [0.0, 0.0, 0.0], // zero velocity — packed to 5 zero bytes
        [0.0, 0.0, 0.0], // zero direction — yaw/pitch/roll all 0
    );
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();

    // [flags][0x10][entity_id u32][pos 3×f32][vel 5 bytes][0x01][yaw][pitch][roll]
    assert_eq!(pt[1], BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
    assert_eq!(&pt[2..6], &0x00ABCDEFu32.to_le_bytes());
    assert_eq!(&pt[6..10], &100.0f32.to_le_bytes());
    assert_eq!(&pt[10..14], &200.0f32.to_le_bytes());
    assert_eq!(&pt[14..18], &300.0f32.to_le_bytes());
    // Zero velocity packed via pack_velocity_xyz — exact bytes are an
    // implementation detail of the packer, but a zero input MUST round-
    // trip to all zeros so a missing-update doesn't drift the ghost.
    assert_eq!(
        &pt[18..23],
        &[0u8; 5],
        "zero velocity must pack to 5 zero bytes"
    );
    assert_eq!(pt[23], 0x01, "physics mode");
    assert_eq!(&pt[24..27], &[0u8; 3], "yaw/pitch/roll for zero direction");
}

/// Ghost-entity lifecycle as observed by a single witness: the four
/// packets that the client receives across the lifecycle. Pins:
///
///   - Phase 1a: CREATE_ENTITY + UPDATE_AVATAR in the same packet
///     (`cached_entity.cpp:199` — entity is allocated and positioned
///     before any property cascade).
///   - Phase 1b: property cascade in a SEPARATE packet, AFTER 1a.
///     The cascade body must NOT begin with CREATE_ENTITY (0x09) —
///     it carries entity-method records (onEntityProperty, onVisible,
///     etc.) configured against an entity the client has already
///     allocated. A regression that fired the cascade before
///     CREATE_ENTITY would surface as a corrupted/dropped entity on
///     the client.
///   - Phase 2: UPDATE_AVATAR ONLY in the move packet — the body's
///     length matches the bare UPDATE_AVATAR record size, so a stray
///     CREATE_ENTITY (or any other extra record) would show up as a
///     longer body. Structural length pin replaces a brittle
///     "no 0x09 byte anywhere in the body" scan, since 0x09 can
///     legitimately appear as a payload byte (entity_id, packed
///     velocity, float bytes).
///   - Phase 3: ENTITY_INVISIBLE then LEAVE_AOI in the same packet
///     (`client_handler.cpp:516-556` — invisible-first prevents a
///     one-frame ghost flicker).
///   - All four packets reference the same entity_id.
///
/// Note on what this CAN'T pin: at the unit level we drive the
/// builders directly, so the seq_id values come from us — that means
/// we can pin "this builder writes back the seq_id we passed" but
/// NOT "the AoI dispatch loop hands these out in order". The
/// production ordering would need a `send_to_witness` /
/// cell_dispatch integration test, which is a separate harness shape.
#[test]
fn ghost_lifecycle_emits_create_cascade_update_invisible_leave_in_order() {
    const ENTITY_ID: u32 = 0xDEAD_BEEF;
    let enc = MercuryEncryption::from_session_key(TEST_KEY);

    // ── phase 1a: CREATE_ENTITY + UPDATE_AVATAR (entry, immediate) ─
    let p1a = build_create_entity_base(
        &TEST_KEY,
        1,
        &[],
        ENTITY_ID,
        0x04,
        [10.0, 20.0, 30.0],
        [0.0, 0.0, 0.0],
    );
    let pt1a = enc.decrypt(&p1a).unwrap();
    assert_eq!(
        pt1a[1], BASEMSG_CREATE_ENTITY,
        "phase 1a must START with CREATE_ENTITY (0x09)"
    );
    assert_eq!(&pt1a[4..8], &ENTITY_ID.to_le_bytes());
    assert_eq!(
        pt1a[12], BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR,
        "CREATE_ENTITY must be immediately followed by UPDATE_AVATAR in the entry packet"
    );
    assert_eq!(&pt1a[13..17], &ENTITY_ID.to_le_bytes());

    // ── phase 1b: property cascade (separate packet, AFTER 1a) ─────
    // With class_id=0x00 and npc_data=None the cascade emits exactly
    // two entity-method records (the SGWSpawnableEntity tail):
    //   1. onEntityFlags(u64=0)   — direct encoding, msg_id = 4|0x80 = 0x84
    //   2. onVisible(u8=1)        — direct encoding, msg_id = 8|0x80 = 0x88
    // The structural pin asserts:
    //   - the first record is onEntityFlags addressing OUR entity_id,
    //     not CREATE_ENTITY (0x09) and not some other entity
    //   - the second record is onVisible, also addressing OUR entity_id
    //   - body length matches (15-byte onEntityFlags + 8-byte onVisible
    //     record = 23 bytes total body)
    // A regression that fired the cascade against the wrong entity_id,
    // dropped the body entirely, or reordered the records would surface
    // here.
    let p1b = build_create_entity_cascade(&TEST_KEY, 2, &[], ENTITY_ID, 0x00, 0, None);
    let pt1b = enc.decrypt(&p1b).unwrap();
    // Record 1: onEntityFlags @ pt1b[1..16]
    //   [msg_id=0x84][word_len=12 LE][entity_id u32][entity_flags u64]
    assert_eq!(
        pt1b[1], 0x84,
        "cascade record 1 must be onEntityFlags direct (4 | 0x80 = 0x84), not CREATE_ENTITY"
    );
    assert_eq!(
        u16::from_le_bytes([pt1b[2], pt1b[3]]),
        12,
        "onEntityFlags word_len = 4 entity_id + 8 entity_flags"
    );
    assert_eq!(
        &pt1b[4..8],
        &ENTITY_ID.to_le_bytes(),
        "onEntityFlags must address OUR entity_id, not a stale id"
    );
    assert_eq!(
        &pt1b[8..16],
        &0u64.to_le_bytes(),
        "default entity_flags = 0 when npc_data is None"
    );
    // Record 2: onVisible @ pt1b[16..24]
    //   [msg_id=0xA1][word_len=5 LE][entity_id u32][visible u8=1]
    assert_eq!(
        pt1b[16], 0x88,
        "cascade record 2 must be onVisible direct (8 | 0x80 = 0x88)"
    );
    assert_eq!(
        u16::from_le_bytes([pt1b[17], pt1b[18]]),
        5,
        "onVisible word_len = 4 entity_id + 1 visible"
    );
    assert_eq!(
        &pt1b[19..23],
        &ENTITY_ID.to_le_bytes(),
        "onVisible must address OUR entity_id"
    );
    assert_eq!(pt1b[23], 0x01, "onVisible flag = 1 (registers in viewport)");
    // Total decrypted = 1 (flags) + 23 (body) + 4 (seq_id footer) = 28
    assert_eq!(
        pt1b.len(),
        1 + 23 + 4,
        "cascade body is exactly the two SGWSpawnableEntity records — extra bytes mean a stray record snuck in"
    );
    // Phase 1b seq_id footer @ pt1b[24..28]
    let seq1b = u32::from_le_bytes([pt1b[24], pt1b[25], pt1b[26], pt1b[27]]);
    assert_eq!(seq1b, 2, "phase 1b carries the seq_id we passed");

    // ── phase 2: UPDATE_AVATAR only in the move packet ────────────
    let p2 = build_avatar_update(
        &TEST_KEY,
        3,
        &[],
        ENTITY_ID,
        [11.0, 20.0, 31.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    );
    let pt2 = enc.decrypt(&p2).unwrap();
    assert_eq!(
        pt2[1], BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR,
        "phase 2 must be UPDATE_AVATAR (0x10)",
    );
    assert_eq!(&pt2[2..6], &ENTITY_ID.to_le_bytes());

    // Structural pin: the move packet body is exactly one
    // UPDATE_AVATAR record (26 bytes — fixed CONSTANT_LENGTH for
    // 0x10). Total decrypted = 1 (flags) + 26 (body) + 4 (seq_id
    // footer) = 31. A stray CREATE_ENTITY or any other extra record
    // would inflate the body and shift the seq_id footer, surfacing
    // here. This replaces a brittle byte-scan check that would have
    // matched 0x09 inside legitimate payload (entity_id, packed
    // velocity, float bytes).
    const FLAGS_LEN: usize = 1;
    const UPDATE_AVATAR_BODY_LEN: usize = 26;
    const SEQ_FOOTER_LEN: usize = 4;
    assert_eq!(
        pt2.len(),
        FLAGS_LEN + UPDATE_AVATAR_BODY_LEN + SEQ_FOOTER_LEN,
        "phase 2 body must be exactly one UPDATE_AVATAR record — extra bytes mean a stray record snuck in"
    );

    // ── phase 3: ENTITY_INVISIBLE then LEAVE_AOI ──────────────────
    let p3 = build_entity_leave(&TEST_KEY, 4, &[], ENTITY_ID);
    let pt3 = enc.decrypt(&p3).unwrap();
    assert_eq!(
        pt3[1], BASEMSG_ENTITY_INVISIBLE,
        "phase 3 must START with ENTITY_INVISIBLE (0x0B) — not LEAVE_AOI",
    );
    assert_eq!(&pt3[2..6], &ENTITY_ID.to_le_bytes());
    assert_eq!(
        pt3[7], BASEMSG_LEAVE_AOI,
        "ENTITY_INVISIBLE must be followed by LEAVE_AOI (0x0C) in the same packet",
    );
    assert_eq!(&pt3[10..14], &ENTITY_ID.to_le_bytes());

    // ── seq_id pin (per-builder write-back contract) ──────────────
    // This pins each builder writes the seq_id we passed in. It's
    // NOT enough on its own to prove dispatch-level ordering — that
    // would need a send_to_witness driver — but it does catch a
    // regression that drops or swaps the seq_id field inside a
    // builder, which would corrupt every downstream ack. Phase 1b
    // is asserted inline in its block above; phases 1a, 2, 3 are
    // checked here using the documented body offsets.
    // Phase 1a: CREATE_ENTITY (11) + UPDATE_AVATAR (26) = 37 → seq @ pt1a[38..42].
    // Phase 2:  UPDATE_AVATAR (26)                          → seq @ pt2[27..31].
    // Phase 3:  ENTITY_INVISIBLE (6) + LEAVE_AOI (11) = 17  → seq @ pt3[18..22].
    let seq1a = u32::from_le_bytes([pt1a[38], pt1a[39], pt1a[40], pt1a[41]]);
    let seq2 = u32::from_le_bytes([pt2[27], pt2[28], pt2[29], pt2[30]]);
    let seq3 = u32::from_le_bytes([pt3[18], pt3[19], pt3[20], pt3[21]]);
    assert_eq!(seq1a, 1);
    assert_eq!(seq2, 3);
    assert_eq!(seq3, 4);
}

/// `compose_create_entity_base_body` must produce the same
/// body bytes that `build_create_entity_base` puts inside its
/// (eventually-encrypted) Mercury packet.
///
/// This is the load-bearing regression guard for the bundle migration:
/// the bundle path appends `compose_*` output verbatim into a multi-
/// message bundle, and the resulting wire format must be byte-equivalent
/// to a sequence of standalone-packet `build_*` emits. Any drift would
/// mean the migrated AoI flush path is sending different bytes to the
/// client than the un-migrated paths still are — a wire-format
/// divergence the existing flush-test would not catch.
///
/// Verifies: decrypted standalone packet body == compose() output.
#[test]
fn compose_create_entity_base_body_matches_build_create_entity_base_body() {
    use crate::mercury::compose_create_entity_base_body;

    let composed =
        compose_create_entity_base_body(0xDEADBEEF, 0x42, [10.0, 20.0, 30.0], [0.0, 0.0, 0.0]);

    // Decrypt the standalone-packet variant and strip [flags] prefix +
    // [seq_id] footer to recover the body byte slice.
    let pkt = build_create_entity_base(
        &TEST_KEY,
        1,
        &[],
        0xDEADBEEF,
        0x42,
        [10.0, 20.0, 30.0],
        [0.0, 0.0, 0.0],
    );
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();
    // pt[0] = flags, body follows, then 4 bytes of seq_id footer.
    let standalone_body = &pt[1..pt.len() - 4];

    assert_eq!(
        composed.as_slice(),
        standalone_body,
        "compose_create_entity_base_body must produce byte-identical output to \
         build_create_entity_base's pre-framing body — divergence would split the \
         wire format between bundle-migrated and un-migrated call sites"
    );
}

/// **Load-bearing byte-equivalence guard for the entity-method bundle path.**
///
/// `ChannelBundle::append_entity_method` (in `cimmeria-mercury`) and
/// `build_player_entity_method_packet`'s body (via `services::mercury::append_entity_method`)
/// must emit byte-identical wire bytes. They live in two different crates
/// and the encoder is duplicated for performance — the only thing keeping
/// them in lock-step is the shared `EXTENDED_ENCODING_THRESHOLD` /
/// `EXTENDED_ENCODING_MARKER` constants and this test.
///
/// Coverage:
/// - **Direct encoding** (`method_index < 61`): pin `BEING_APPEARANCE` (26),
///   `ON_ENTITY_TINT` (10), `ON_CHAT_JOINED` (31), `ON_PLAYER_COMMUNICATION`
///   (28). Together these are every method in the
///   `handle_on_client_ready` post-bundle burst — drift in any of them
///   silently desyncs the post-onClientReady client state.
/// - **Extended encoding** (`method_index >= 61`): pin `ON_PLAY_MOVIE` (155),
///   the only extended-encoding emit in `world_entry_appearance.rs` (via
///   `send_cinematic`). The encoder's `(method_index - 61) as u8` truncation
///   path needs at least one extended-encoding witness or a future regression
///   could land at index ≥ 317 (`u8::MAX + 61 + 1`) and silently truncate.
///
/// **Regression shape**: if a future refactor changes the wire layout in
/// only one of the two encoders (a length-prefix endian flip, dropping the
/// 0x80 direct-encoding marker, etc.), the bundle-migrated `handle_on_client_ready`
/// path would emit different bytes than the still-per-message `send_cinematic`
/// path, breaking the BeingAppearance/onChatJoined replay semantics. This
/// test fires before the wire divergence reaches a real client.
#[test]
fn channel_bundle_append_entity_method_matches_build_entity_method_packet_body() {
    use crate::mercury::{build_player_entity_method_packet, method_idx};
    use cimmeria_mercury::channel_bundle::{ChannelBundle, IDBASE_SGW_PLAYER};

    const ENTITY_ID: u32 = 0xCAFE_BABE;

    // Direct-encoding witnesses: every method index in the
    // handle_on_client_ready burst.
    let direct_cases: &[(u16, &[u8])] = &[
        (method_idx::BEING_APPEARANCE, b"appearance-body-bytes"),
        (method_idx::ON_ENTITY_TINT, &[0u8; 12]),
        (method_idx::ON_CHAT_JOINED, b"chat-joined-args"),
        (method_idx::ON_PLAYER_COMMUNICATION, b"welcome-msg-args"),
    ];

    for &(method_index, args) in direct_cases {
        // Bundle path: append one method and grab the accumulated body.
        let mut bundle = ChannelBundle::new(true);
        bundle.append_entity_method(method_index, IDBASE_SGW_PLAYER, ENTITY_ID, args);
        // body is private — finalize without acks/flags to recover the
        // bytes. With body_len() < FRAGMENT_BODY_SIZE this emits 1 packet,
        // and the plaintext closure is identity so we get the body back.
        let (packets, _seqs) = bundle.finalize(0, 0, |plaintext| plaintext.to_vec());
        assert_eq!(
            packets.len(),
            1,
            "small body must be one un-fragmented packet"
        );
        // Bundle's finalize prepends the flags byte AND appends the seq
        // footer (since FLAG_HAS_SEQUENCE is implied by base_seq=0 +
        // build_fragmented_bundle's framing). Recover body == pkt[1..len-4].
        let bundle_body = &packets[0][1..packets[0].len() - 4];

        // Standalone-packet path: decrypt and strip flags+seq footer.
        let pkt =
            build_player_entity_method_packet(&TEST_KEY, 1, &[], ENTITY_ID, method_index, args);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();
        let standalone_body = &pt[1..pt.len() - 4];

        assert_eq!(
            bundle_body, standalone_body,
            "direct encoding mismatch for method_index={method_index} — \
             ChannelBundle::append_entity_method drifted from \
             services::mercury::append_entity_method"
        );
    }

    // Extended-encoding witness: ON_PLAY_MOVIE (155 ≥ 61). Exercises the
    // marker-byte + sub_index code path that direct cases don't.
    {
        let method_index = method_idx::ON_PLAY_MOVIE;
        let args = b"\x10\x00\x00\x00Cine-Test.Cine\x00\x01".as_slice();
        let mut bundle = ChannelBundle::new(true);
        bundle.append_entity_method(method_index, IDBASE_SGW_PLAYER, ENTITY_ID, args);
        let (packets, _seqs) = bundle.finalize(0, 0, |plaintext| plaintext.to_vec());
        assert_eq!(packets.len(), 1);
        let bundle_body = &packets[0][1..packets[0].len() - 4];

        let pkt =
            build_player_entity_method_packet(&TEST_KEY, 1, &[], ENTITY_ID, method_index, args);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&pkt).unwrap();
        let standalone_body = &pt[1..pt.len() - 4];

        assert_eq!(
            bundle_body, standalone_body,
            "extended encoding mismatch for method_index={method_index} — \
             the 0xBD marker + sub_index path drifted between bundle and \
             standalone-packet encoders"
        );
        // Also pin the extended-marker first byte so a regression that
        // accidentally encoded ON_PLAY_MOVIE as direct (255 & 0x80 → wrong
        // method id) still fails clearly.
        assert_eq!(
            bundle_body[0], 0xBD,
            "method_index >= 61 must use extended encoding marker 0xBD"
        );
    }
}

/// Same regression guard for the phase-2 cascade composer. Tests the
/// `npc_data: None` shape (player phase-2, smallest cascade — class_id !=
/// 0 → includes the SGWBeing block but skips all the npc-specific
/// optional fields).
#[test]
fn compose_create_entity_cascade_body_matches_build_create_entity_cascade_body() {
    use crate::mercury::compose_create_entity_cascade_body;

    let composed = compose_create_entity_cascade_body(0x12345678, 0x42, 7, None);

    let pkt = build_create_entity_cascade(&TEST_KEY, 1, &[], 0x12345678, 0x42, 7, None);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();
    let standalone_body = &pt[1..pt.len() - 4];

    assert_eq!(
        composed.as_slice(),
        standalone_body,
        "compose_create_entity_cascade_body must produce byte-identical output to \
         build_create_entity_cascade's pre-framing body — divergence would split \
         the wire format between bundle-migrated and un-migrated call sites"
    );
}

/// **Byte-equivalence guard for the FORCED_POSITION bundle migration
/// (issue #360 teleport family).**
///
/// `compose_forced_position_body` is appended via
/// [`cimmeria_mercury::channel_bundle::ChannelBundle::append_raw_message`]
/// inside [`crate::base::world_entry::teleport::build_teleport_bundle`].
/// The bundled wire bytes for that raw message MUST equal the body
/// portion of the standalone [`build_forced_position`] packet. Divergence
/// would split the wire format between the bundle-migrated teleport path
/// and the standalone-packet `build_forced_position` callers (none today
/// other than tests, but the cross-path guard prevents drift if a future
/// caller resurrects the standalone path).
///
/// Verifies: decrypted standalone packet body == compose() output.
#[test]
fn compose_forced_position_body_matches_build_forced_position_body() {
    use crate::mercury::compose_forced_position_body;

    const ENTITY_ID: u32 = 0xDEAD_BEEF;
    const SPACE_ID: u32 = 0x0001_0010;
    const POS: [f32; 3] = [10.0, 20.0, 30.0];
    const PREV_POS: [f32; 3] = [9.0, 19.0, 29.0];

    let composed = compose_forced_position_body(ENTITY_ID, SPACE_ID, POS, PREV_POS);

    let pkt = build_forced_position(&TEST_KEY, 1, &[], ENTITY_ID, SPACE_ID, POS, PREV_POS);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let pt = enc.decrypt(&pkt).unwrap();
    let standalone_body = &pt[1..pt.len() - 4];

    assert_eq!(
        composed.as_slice(),
        standalone_body,
        "compose_forced_position_body must produce byte-identical output to \
         build_forced_position's pre-framing body — divergence would split \
         the wire format between bundle-migrated and standalone-packet paths"
    );
    // Length pin: catches a regression that adds/drops a field. The
    // offset-based forced_position_wire_layout test pins the layout, but
    // a length-drift hits this assertion more directly.
    assert_eq!(
        composed.len(),
        50,
        "FORCED_POSITION body is exactly 50 bytes: msg_id(1) + entity_id(4) + \
         space_id(4) + vehicleID(4) + pos(12) + prev_pos(12) + rot(12) + flags(1)"
    );
}
