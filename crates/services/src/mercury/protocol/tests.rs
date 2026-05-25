//! Tests for the protocol builders.  These verify wire-level invariants:
//! packet sizes, determinism (CBC with zero IV), length-prefix widths, and
//! entity-method encoding boundaries.

use super::super::{
    write_wstring, BASEMSG_LOGGED_OFF, BASEMSG_ON_VERSION_INFO, BASEMSG_RESOURCE_FRAGMENT,
    FRAG_FIRST_AND_LAST,
};
use super::*;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::FLAG_HAS_ACKS;

const TEST_KEY: [u8; 32] = [0x42u8; 32];

#[test]
fn connect_reply_size() {
    let ticket = b"12345678901234567890";
    let out = build_connect_reply(0xDEADBEEF, ticket, &TEST_KEY, 1);
    assert_eq!(
        out.len(),
        64,
        "reply should be 64 bytes: 48 ciphertext + 16 HMAC"
    );
}

#[test]
fn time_sync_size() {
    let out = build_time_sync(&TEST_KEY, 2);
    assert_eq!(
        out.len(),
        48,
        "time sync should be 48 bytes: 32 ciphertext + 16 HMAC"
    );
}

#[test]
fn connect_reply_deterministic() {
    let ticket = b"AABBCCDDEEFF00112233";
    let a = build_connect_reply(1, ticket, &TEST_KEY, 1);
    let b = build_connect_reply(1, ticket, &TEST_KEY, 1);
    assert_eq!(
        a, b,
        "same inputs → same encrypted output (deterministic CBC with zero IV)"
    );
}

#[test]
fn time_sync_deterministic() {
    let a = build_time_sync(&TEST_KEY, 2);
    let b = build_time_sync(&TEST_KEY, 2);
    assert_eq!(a, b);
}

#[test]
fn reply_and_time_sync_differ() {
    let ticket = b"12345678901234567890";
    let reply = build_connect_reply(0, ticket, &TEST_KEY, 1);
    let sync = build_time_sync(&TEST_KEY, 2);
    assert_ne!(reply, sync, "reply and time sync packets must differ");
}

#[test]
fn char_list_empty() {
    // Empty char list → creation screen
    let out = build_char_list(&TEST_KEY, 3, &[], &[], 1);
    assert!(!out.is_empty());
}

#[test]
fn char_list_with_one_character() {
    let chars = vec![CharacterInfo {
        player_id: 1,
        name: "Wanderer".to_string(),
        extra_name: String::new(),
        alignment: 0,
        level: 1,
        gender: 0,
        world_location: "CombatSim".to_string(),
        archetype: 0,
        title: 0,
        player_type: 0,
        playable: 1,
    }];
    let out = build_char_list(&TEST_KEY, 3, &[], &chars, 1);
    assert!(!out.is_empty());
}

#[test]
fn char_list_empty_deterministic() {
    let a = build_char_list(&TEST_KEY, 3, &[], &[], 1);
    let b = build_char_list(&TEST_KEY, 3, &[], &[], 1);
    assert_eq!(a, b);
}

#[test]
fn char_list_with_ack() {
    let out = build_char_list(&TEST_KEY, 3, &[0], &[], 1);
    assert!(!out.is_empty());
}

#[test]
fn ongoing_tick_sync_size() {
    let out = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
    assert_eq!(out.len(), 32, "tick sync should be 32 bytes");
}

#[test]
fn ongoing_tick_sync_with_acks() {
    let out = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[0]);
    assert_eq!(out.len(), 48, "tick sync with 1 ack should be 48 bytes");
}

#[test]
fn ongoing_tick_sync_changes_with_tick() {
    let a = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
    let b = build_ongoing_tick_sync(&TEST_KEY, 4, 1, &[]);
    assert_ne!(
        a, b,
        "different tick values must produce different ciphertexts"
    );
}

#[test]
fn ongoing_tick_sync_changes_with_seq() {
    let a = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
    let b = build_ongoing_tick_sync(&TEST_KEY, 5, 0, &[]);
    assert_ne!(a, b, "different seq_ids must produce different ciphertexts");
}

/// Regression guard for the unreliable-stream invariant: the outer flags
/// byte of `build_ongoing_tick_sync` must NOT carry `FLAG_RELIABLE`.
///
/// Putting tickSync on the reliable stream is the failure mode the
/// split-counter design defends against — a reliable tickSync at 10 Hz
/// saturates the 32-slot TX window within seconds of world entry. The
/// invariant lives at the wire-format layer (the receiver decides by
/// looking at this byte), so the regression guard reads the byte after
/// decrypting and verifies the bit is clear.
///
/// Pin both forms: with and without piggybacked ACKs. The ACK form must
/// additionally set `FLAG_HAS_ACKS` without ever toggling `FLAG_RELIABLE`
/// on by accident.
#[test]
fn ongoing_tick_sync_flags_byte_is_unreliable_with_or_without_acks() {
    use cimmeria_mercury::packet::FLAG_RELIABLE;

    // No ACKs: outer flags should be exactly REPLY_FLAGS_UNRELIABLE
    // (FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL), with FLAG_RELIABLE cleared
    // and FLAG_HAS_ACKS not set.
    let no_acks = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[]);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&no_acks).expect("decrypt failed");
    let flags = plaintext[0];
    assert_eq!(
        flags & FLAG_RELIABLE,
        0,
        "tickSync packet flags byte must NOT have FLAG_RELIABLE set; \
         a reliable tickSync at 10 Hz saturates the 32-slot reliable \
         TX window during world-entry burst (got flags = {flags:#04x})"
    );
    assert_eq!(
        flags & FLAG_HAS_ACKS,
        0,
        "tickSync without ACKs must not set FLAG_HAS_ACKS (got flags = {flags:#04x})"
    );
    assert_eq!(
        flags, REPLY_FLAGS_UNRELIABLE,
        "tickSync flags byte should equal REPLY_FLAGS_UNRELIABLE exactly when no ACKs are present"
    );

    // ACK-piggyback form: flags should add FLAG_HAS_ACKS, still no
    // FLAG_RELIABLE.
    let with_acks = build_ongoing_tick_sync(&TEST_KEY, 4, 0, &[0]);
    let plaintext_acks = enc.decrypt(&with_acks).expect("decrypt failed");
    let flags_acks = plaintext_acks[0];
    assert_eq!(
        flags_acks & FLAG_RELIABLE,
        0,
        "tickSync with ACKs must still not set FLAG_RELIABLE (got flags = {flags_acks:#04x})"
    );
    assert_ne!(
        flags_acks & FLAG_HAS_ACKS,
        0,
        "tickSync with ACKs must set FLAG_HAS_ACKS (got flags = {flags_acks:#04x})"
    );
    assert_eq!(
        flags_acks,
        REPLY_FLAGS_UNRELIABLE | FLAG_HAS_ACKS,
        "tickSync flags byte should equal REPLY_FLAGS_UNRELIABLE|FLAG_HAS_ACKS when ACKs are present"
    );
}

#[test]
fn char_create_failed_produces_output() {
    let out = build_char_create_failed(&TEST_KEY, 5, &[], 1, 1);
    assert!(!out.is_empty());
}

#[test]
fn resource_fragment_produces_output() {
    let xml = b"<CharDef>test</CharDef>";
    let out = build_resource_fragment(
        &TEST_KEY,
        5,
        &[],
        1, // data_id
        0, // chunk_id
        FRAG_FIRST_AND_LAST,
        Some(0), // msg_type = MESSAGE_CacheData
        Some(7), // category_id = char_creation
        Some(1), // element_id
        xml,
    );
    assert!(!out.is_empty());
}

#[test]
fn version_info_produces_output() {
    let out = build_version_info(&TEST_KEY, 5, &[], 7, 1, 23, true, &[], 1);
    assert!(!out.is_empty());
}

/// Decrypt + parse the onVersionInfo body and assert each field's bytes.
/// Catches an encoder change that flips field order, drops the
/// `InvalidKeys` count prefix, reorders/zeroes keys, or rewrites the
/// `invalidate_all` byte — none of which a packet-size-only test sees.
#[test]
fn version_info_invalid_keys_payload_layout_is_byte_exact() {
    let invalid_keys: [u32; 3] = [622, 641, 80622];
    let category_id = 3u32;
    let version = 33304u32;
    let required_updates = invalid_keys.len() as u32;
    let account_eid = 0xDEADBEEFu32;

    let out = build_version_info(
        &TEST_KEY,
        5,
        &[],
        category_id,
        version,
        required_updates,
        false,
        &invalid_keys,
        account_eid,
    );

    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("decrypt failed");

    // Plaintext shape: [flags:u8][msg_id:u8][len:u16][payload...][footers...]
    assert_eq!(plaintext[0], REPLY_FLAGS, "outer reply flags");
    assert_eq!(
        plaintext[1], BASEMSG_ON_VERSION_INFO,
        "msg_id must be BASEMSG_ON_VERSION_INFO (0x80)",
    );
    let word_len = u16::from_le_bytes([plaintext[2], plaintext[3]]) as usize;

    // Payload layout (per build_version_info):
    //   account_entity_id u32 | category_id u32 | version u32 |
    //   required_updates u32 | invalidate_all u8 |
    //   invalid_keys ARRAY<u32> { count u32, entries... }
    let p = &plaintext[4..4 + word_len];
    let expected_payload_len = 4 + 4 + 4 + 4 + 1 + 4 + invalid_keys.len() * 4;
    assert_eq!(
        word_len, expected_payload_len,
        "u16 length prefix must match payload size",
    );

    assert_eq!(
        u32::from_le_bytes(p[0..4].try_into().unwrap()),
        account_eid,
        "account_entity_id LE u32",
    );
    assert_eq!(
        u32::from_le_bytes(p[4..8].try_into().unwrap()),
        category_id,
        "category_id LE u32",
    );
    assert_eq!(
        u32::from_le_bytes(p[8..12].try_into().unwrap()),
        version,
        "version LE u32",
    );
    assert_eq!(
        u32::from_le_bytes(p[12..16].try_into().unwrap()),
        required_updates,
        "required_updates LE u32",
    );
    assert_eq!(p[16], 0, "invalidate_all byte must be 0 (false)");
    assert_eq!(
        u32::from_le_bytes(p[17..21].try_into().unwrap()),
        invalid_keys.len() as u32,
        "InvalidKeys ARRAY count must equal slice length",
    );
    // Keys appear in slice order (deterministic; pinned so a future
    // .sort() or .iter().rev() doesn't slip in unnoticed).
    for (i, &expected) in invalid_keys.iter().enumerate() {
        let off = 21 + i * 4;
        assert_eq!(
            u32::from_le_bytes(p[off..off + 4].try_into().unwrap()),
            expected,
            "InvalidKeys[{i}] must equal {expected}",
        );
    }
}

/// `invalidate_all = true` + empty `invalid_keys` writes a 0-count
/// trailing array — the legacy whole-cache nuke path. Pin that the byte
/// after the flag is the count word, not stray data.
#[test]
fn version_info_invalidate_all_writes_zero_count_array() {
    let out = build_version_info(&TEST_KEY, 5, &[], 7, 1, 23, true, &[], 1);
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("decrypt failed");

    assert_eq!(plaintext[1], BASEMSG_ON_VERSION_INFO);
    let word_len = u16::from_le_bytes([plaintext[2], plaintext[3]]) as usize;
    let p = &plaintext[4..4 + word_len];

    assert_eq!(p[16], 1, "invalidate_all byte must be 1 (true)");
    assert_eq!(
        u32::from_le_bytes(p[17..21].try_into().unwrap()),
        0,
        "InvalidKeys array count must be 0 when slice is empty",
    );
    assert_eq!(word_len, 21, "no per-key bytes after the count");
}

#[test]
fn read_wstring_roundtrip() {
    use super::super::read_wstring;

    let mut buf = Vec::new();
    write_wstring(&mut buf, "Hello");
    let (s, consumed) = read_wstring(&buf, 0).unwrap();
    assert_eq!(s, "Hello");
    assert_eq!(consumed, 4 + 5 * 2); // 4 byte count + 5 UTF-16LE chars
}

#[test]
fn read_wstring_empty() {
    use super::super::read_wstring;

    let mut buf = Vec::new();
    write_wstring(&mut buf, "");
    let (s, consumed) = read_wstring(&buf, 0).unwrap();
    assert_eq!(s, "");
    assert_eq!(consumed, 4);
}

#[test]
fn resource_fragment_uses_u16_length_prefix() {
    // C++ ServerMessageList says RESOURCE_FRAGMENT (0x36) uses WORD_LENGTH (u16).
    // A previous bug wrote u32 here, which corrupts all cooked data serving.
    let xml = b"<CharDef>test</CharDef>";
    let out = build_resource_fragment(
        &TEST_KEY,
        5,
        &[],
        1, // data_id
        0, // chunk_id
        FRAG_FIRST_AND_LAST,
        Some(0), // msg_type
        Some(7), // category_id
        Some(1), // element_id
        xml,
    );

    // Decrypt to get the plaintext Mercury packet
    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("decrypt failed");

    // Plaintext layout: [flags:u8][body...][footers]
    // build_outgoing puts footers AFTER body (seq_id is a footer, not a header).
    // Body starts immediately after the flags byte at offset 1.
    assert_eq!(plaintext[0], REPLY_FLAGS);
    let body_start = 1;

    // Body starts with msg_id = 0x36
    assert_eq!(
        plaintext[body_start], BASEMSG_RESOURCE_FRAGMENT,
        "first body byte should be RESOURCE_FRAGMENT (0x36)"
    );

    // Next 2 bytes: u16 LE length prefix
    let len_lo = plaintext[body_start + 1];
    let len_hi = plaintext[body_start + 2];
    let word_len = u16::from_le_bytes([len_lo, len_hi]);

    // Expected payload: dataId(2) + chunkId(1) + flags(1) + msgType(1) +
    //                   categoryId(4) + elementId(4) + xml(23) = 36
    let expected_payload_len: u16 = 2 + 1 + 1 + 1 + 4 + 4 + xml.len() as u16;
    assert_eq!(
        word_len, expected_payload_len,
        "u16 length prefix should match payload size"
    );

    // The byte right after the u16 length prefix is the start of the payload
    // (dataId low byte). If the length were u32, byte at body_start+3 would
    // be 0x00 (upper bytes of a zero-extended small length). Instead it must
    // be part of the actual payload data (dataId = 1 low byte = 0x01).
    let byte_after_u16 = plaintext[body_start + 3];
    assert_ne!(
        byte_after_u16, 0x00,
        "byte after u16 length should be payload data (dataId=1), not 0x00 from a u32 prefix"
    );
}

/// Pin the chunk-size budget for `BASEMSG_RESOURCE_FRAGMENT`: a
/// **first** fragment carries 16 bytes of in-body header overhead
/// (`BASEMSG + WORD_LEN(2) + data_id(2) + chunk_id + frag_flags +
/// msg_type + category_id(4) + element_id(4)`), so the largest XML
/// chunk that still fits in Mercury's 1411-byte `MAX_BODY_LENGTH`
/// plaintext body is 1395 bytes. The cooked-data senders chunk at
/// 1390 (5-byte safety margin); this guard makes sure the resulting
/// first-fragment body decrypts cleanly and lands within that cap.
///
/// Without this, a bump of `MAX_CHUNK` (e.g. the 1000 → 1390 raise that
/// cut the cold-cache login burst) has no regression net — a future
/// overhead change in `build_resource_fragment` would silently push us
/// over the wire limit on every patched-mission push. See
/// `crates/services/src/base/cooked_data.rs::MAX_CHUNK`.
/// Mercury `MAX_BODY_LENGTH` (plaintext body length, post-decrypt).
const MERCURY_MAX_BODY_LENGTH: usize = 1411;

/// Read the actual `BASEMSG_RESOURCE_FRAGMENT` body length from a decrypted
/// Mercury plaintext. Returns `(word_len_payload, total_body_len)` where
/// `total_body_len = 1 (BASEMSG) + 2 (WORD_LEN) + word_len_payload` — i.e.
/// the full slice the wire-format cap applies to. Pulling these from the
/// actual decoded bytes (instead of a hand-computed sum) makes the
/// downstream guards survive a future change to the in-body header layout
/// in `build_resource_fragment`: the math sanity-check would shift, but
/// the cap-fits assertion still works against ground truth.
fn measure_resource_fragment_body(plaintext: &[u8]) -> (usize, usize) {
    assert_eq!(
        plaintext[1], BASEMSG_RESOURCE_FRAGMENT,
        "decrypted plaintext does not start with BASEMSG_RESOURCE_FRAGMENT"
    );
    let word_len = u16::from_le_bytes([plaintext[2], plaintext[3]]) as usize;
    let total_body_len = 1 + 2 + word_len; // BASEMSG + WORD_LEN(u16) + payload
    (word_len, total_body_len)
}

#[test]
fn resource_fragment_first_frag_at_max_chunk_fits_within_mercury_body_limit() {
    // First-fragment in-body payload (after BASEMSG + WORD_LEN(u16)) is
    // data_id(2) + chunk_id(1) + frag_flags(1) + msg_type(1) +
    // category_id(4) + element_id(4) + xml = 13 + xml bytes. So the
    // largest XML that still fits MAX_BODY_LENGTH = 1411 is
    // 1411 - 3 (BASEMSG + WORD_LEN) - 13 = 1395 bytes.
    const FIRST_FRAG_MAX_CHUNK: usize = MERCURY_MAX_BODY_LENGTH - 16;

    let xml = vec![b'X'; FIRST_FRAG_MAX_CHUNK];
    let out = build_resource_fragment(
        &TEST_KEY,
        5,
        &[],
        1, // data_id
        0, // chunk_id
        FRAG_FIRST_AND_LAST,
        Some(0),  // msg_type
        Some(7),  // category_id
        Some(42), // element_id
        &xml,
    );

    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("first-frag decrypt failed");

    let (word_len, total_body_len) = measure_resource_fragment_body(&plaintext);
    // The first-fragment payload (BASEMSG + WORD_LEN consumed) is:
    // data_id(2) + chunk_id(1) + frag_flags(1) + msg_type(1) +
    // category_id(4) + element_id(4) + xml.len() = 13 + xml.len()
    assert_eq!(
        word_len,
        13 + xml.len(),
        "WORD_LEN prefix should match expected first-fragment payload size",
    );
    assert!(
        total_body_len <= MERCURY_MAX_BODY_LENGTH,
        "first-fragment body ({total_body_len}) exceeded MAX_BODY_LENGTH ({MERCURY_MAX_BODY_LENGTH})"
    );
    assert_eq!(
        total_body_len, MERCURY_MAX_BODY_LENGTH,
        "first-fragment at FIRST_FRAG_MAX_CHUNK should pack to exactly MAX_BODY_LENGTH",
    );
}

/// Non-first fragments drop msg_type + category_id + element_id (-9
/// bytes), so the in-body header collapses to 7 bytes and the chunk
/// cap rises to 1404. Pin that separately — the 5-byte safety margin
/// in `MAX_CHUNK = 1390` exists for the tighter first-fragment cap,
/// and a refactor that accidentally treats non-first like first would
/// not be caught by the first-fragment guard above.
#[test]
fn resource_fragment_non_first_frag_at_max_chunk_fits_within_mercury_body_limit() {
    // Non-first in-body payload: data_id(2) + chunk_id(1) + frag_flags(1)
    // + xml = 4 + xml. Max xml = 1411 - 3 (BASEMSG + WORD_LEN) - 4 = 1404.
    const NON_FIRST_FRAG_MAX_CHUNK: usize = MERCURY_MAX_BODY_LENGTH - 7;

    let xml = vec![b'Y'; NON_FIRST_FRAG_MAX_CHUNK];
    let out = build_resource_fragment(
        &TEST_KEY,
        5,
        &[],
        1, // data_id
        1, // chunk_id (non-first)
        super::super::FRAG_MIDDLE,
        None, // msg_type omitted on non-first
        None, // category_id omitted
        None, // element_id omitted
        &xml,
    );

    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("non-first-frag decrypt failed");

    let (word_len, total_body_len) = measure_resource_fragment_body(&plaintext);
    assert_eq!(
        word_len,
        4 + xml.len(),
        "WORD_LEN prefix should match expected non-first payload size",
    );
    assert!(
        total_body_len <= MERCURY_MAX_BODY_LENGTH,
        "non-first body ({total_body_len}) exceeded MAX_BODY_LENGTH ({MERCURY_MAX_BODY_LENGTH})"
    );
    assert_eq!(
        total_body_len, MERCURY_MAX_BODY_LENGTH,
        "non-first at NON_FIRST_FRAG_MAX_CHUNK should pack to exactly MAX_BODY_LENGTH",
    );
}

/// The cooked-data layer chunks at `MAX_CHUNK = 1390`. Pin that the
/// resulting first-fragment body decrypts cleanly and stays under
/// MAX_BODY_LENGTH with the actual 5-byte safety margin — the two
/// tests above prove the theoretical absolute cap; this one proves
/// the production chunker value still has headroom against the
/// decoded WORD_LEN, not just a hand-computed body sum.
#[test]
fn resource_fragment_first_frag_at_cooked_data_max_chunk_has_safety_margin() {
    const COOKED_DATA_MAX_CHUNK: usize = 1390; // crates/services/src/base/cooked_data.rs

    let xml = vec![b'Z'; COOKED_DATA_MAX_CHUNK];
    let out = build_resource_fragment(
        &TEST_KEY,
        5,
        &[],
        1,
        0,
        FRAG_FIRST_AND_LAST,
        Some(0),
        Some(7),
        Some(42),
        &xml,
    );

    let enc = MercuryEncryption::from_session_key(TEST_KEY);
    let plaintext = enc.decrypt(&out).expect("decrypt failed");

    let (_word_len, total_body_len) = measure_resource_fragment_body(&plaintext);
    assert!(
        total_body_len <= MERCURY_MAX_BODY_LENGTH,
        "cooked-data first-fragment body ({total_body_len}) must fit \
         MAX_BODY_LENGTH ({MERCURY_MAX_BODY_LENGTH})",
    );
    // 5-byte safety margin: 1395 (first-frag cap) - 1390 (chunk size).
    let safety_margin = MERCURY_MAX_BODY_LENGTH - total_body_len;
    assert!(
        safety_margin >= 5,
        "safety margin under first-fragment cap dropped to {safety_margin} bytes \
         — if this fires, either lower MAX_CHUNK or audit the in-body header"
    );
}

#[test]
fn logged_off_produces_output() {
    let out = build_logged_off(&TEST_KEY, 10, &[]);
    assert!(!out.is_empty(), "logged_off packet should not be empty");
}

#[test]
fn logged_off_body_contains_msg_id_and_reason() {
    let out = build_logged_off(&TEST_KEY, 10, &[]);
    // Decrypt to verify body contents.
    let enc = MercuryEncryption::new(TEST_KEY, [0u8; 16], TEST_KEY);
    let plaintext = enc.decrypt(&out).unwrap();
    // Body starts after flags byte at offset 1 (seq is in the footer).
    let body_start = 1;
    assert!(plaintext.len() > body_start + 1);
    assert_eq!(
        plaintext[body_start], BASEMSG_LOGGED_OFF,
        "msg_id should be 0x37"
    );
    assert_eq!(plaintext[body_start + 1], 0x00, "reason should be 0");
}

#[test]
fn logged_off_with_acks_includes_ack_flag() {
    let out = build_logged_off(&TEST_KEY, 10, &[3, 4]);
    let enc = MercuryEncryption::new(TEST_KEY, [0u8; 16], TEST_KEY);
    let plaintext = enc.decrypt(&out).unwrap();
    // flags byte should include FLAG_HAS_ACKS (0x04)
    assert_ne!(plaintext[0] & FLAG_HAS_ACKS, 0, "should have ACK flag set");
}

// ── Entity method encoding tests ────────────────────────────────────────

#[test]
fn append_entity_method_direct_index_0() {
    use super::super::append_entity_method;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    let mut body = Vec::new();
    append_entity_method(&mut body, 0, IDBASE_SGW_PLAYER, 42, &[]);
    // msg_id = 0 | 0x80 = 0x80, word_len = 4, entity_id = 42
    assert_eq!(body[0], 0x80);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 4);
    assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 42);
    assert_eq!(body.len(), 7);
}

#[test]
fn append_entity_method_direct_index_60() {
    use super::super::append_entity_method;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    // Index 60 is the last direct-encoded index (msg_id 0x80-0xBC)
    let mut body = Vec::new();
    append_entity_method(&mut body, 60, IDBASE_SGW_PLAYER, 1, &[0xAA, 0xBB]);
    // msg_id = 60 | 0x80 = 0xBC, word_len = 6, entity_id = 1, args
    assert_eq!(body[0], 0xBC);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 6);
    assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 1);
    assert_eq!(&body[7..9], &[0xAA, 0xBB]);
}

#[test]
fn append_entity_method_extended_index_61() {
    use super::super::append_entity_method;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    // Index 61 is the first extended-encoded index (0xBD marker)
    let mut body = Vec::new();
    append_entity_method(&mut body, 61, IDBASE_SGW_PLAYER, 99, &[]);
    // marker = 0xBD, word_len = 5 (entity_id + sub_index), entity_id = 99,
    // sub_index = 61 - 61 = 0
    assert_eq!(body[0], 0xBD);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 5);
    assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 99);
    assert_eq!(body[7], 0);
    assert_eq!(body.len(), 8);
}

#[test]
fn append_entity_method_extended_index_128() {
    use super::super::append_entity_method;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    let mut body = Vec::new();
    append_entity_method(&mut body, 128, IDBASE_SGW_PLAYER, 99, &[]);
    // marker = 0xBD, word_len = 5 (entity_id + sub_index), entity_id = 99,
    // sub_index = 128 - 61 = 67 (0x43)
    assert_eq!(body[0], 0xBD);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 5);
    assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 99);
    assert_eq!(body[7], 67); // 0x43
    assert_eq!(body.len(), 8);
}

#[test]
fn append_entity_method_extended_index_141() {
    use super::super::append_entity_method;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    // onAbilityTreeInfo = 141 → sub_index = 141 - 61 = 80 (0x50)
    let mut body = Vec::new();
    let args = [0x01, 0x02, 0x03];
    append_entity_method(&mut body, 141, IDBASE_SGW_PLAYER, 5, &args);
    assert_eq!(body[0], 0xBD);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 8); // 4 + 1 + 3
    assert_eq!(body[7], 80); // 0x50
    assert_eq!(&body[8..11], &[0x01, 0x02, 0x03]);
}

#[test]
fn append_entity_method_preserves_existing_body() {
    use super::super::append_entity_method;
    use cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER;

    let mut body = vec![0xDE, 0xAD];
    append_entity_method(&mut body, 0, IDBASE_SGW_PLAYER, 1, &[]);
    assert_eq!(&body[..2], &[0xDE, 0xAD]); // prefix preserved
    assert_eq!(body[2], 0x80); // method appended after
}
