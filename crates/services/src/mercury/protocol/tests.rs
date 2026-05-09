//! Tests for the protocol builders.  These verify wire-level invariants:
//! packet sizes, determinism (CBC with zero IV), length-prefix widths, and
//! entity-method encoding boundaries.

use super::super::{
    write_wstring, BASEMSG_LOGGED_OFF, BASEMSG_RESOURCE_FRAGMENT, FRAG_FIRST_AND_LAST,
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

#[test]
fn version_info_per_key_invalidation_round_trips_through_encoder() {
    // Pin the call shape: invalid_keys is an `&[u32]` argument and the
    // empty-vs-populated calls produce different output sizes. Catches
    // a future signature change that drops the slice or makes it optional
    // without surfacing as a compile error at all call sites.
    let no_keys = build_version_info(&TEST_KEY, 5, &[], 3, 99, 0, false, &[], 1);
    let two_keys = build_version_info(&TEST_KEY, 5, &[], 3, 99, 0, false, &[622u32, 641u32], 1);
    assert!(
        two_keys.len() > no_keys.len(),
        "per-key payload must be wider"
    );
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

    let mut body = Vec::new();
    append_entity_method(&mut body, 0, 42, &[]);
    // msg_id = 0 | 0x80 = 0x80, word_len = 4, entity_id = 42
    assert_eq!(body[0], 0x80);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 4);
    assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 42);
    assert_eq!(body.len(), 7);
}

#[test]
fn append_entity_method_direct_index_60() {
    use super::super::append_entity_method;

    // Index 60 is the last direct-encoded index (msg_id 0x80-0xBC)
    let mut body = Vec::new();
    append_entity_method(&mut body, 60, 1, &[0xAA, 0xBB]);
    // msg_id = 60 | 0x80 = 0xBC, word_len = 6, entity_id = 1, args
    assert_eq!(body[0], 0xBC);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 6);
    assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 1);
    assert_eq!(&body[7..9], &[0xAA, 0xBB]);
}

#[test]
fn append_entity_method_extended_index_61() {
    use super::super::append_entity_method;

    // Index 61 is the first extended-encoded index (0xBD marker)
    let mut body = Vec::new();
    append_entity_method(&mut body, 61, 99, &[]);
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

    let mut body = Vec::new();
    append_entity_method(&mut body, 128, 99, &[]);
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

    // onAbilityTreeInfo = 141 → sub_index = 141 - 61 = 80 (0x50)
    let mut body = Vec::new();
    let args = [0x01, 0x02, 0x03];
    append_entity_method(&mut body, 141, 5, &args);
    assert_eq!(body[0], 0xBD);
    assert_eq!(u16::from_le_bytes([body[1], body[2]]), 8); // 4 + 1 + 3
    assert_eq!(body[7], 80); // 0x50
    assert_eq!(&body[8..11], &[0x01, 0x02, 0x03]);
}

#[test]
fn append_entity_method_preserves_existing_body() {
    use super::super::append_entity_method;

    let mut body = vec![0xDE, 0xAD];
    append_entity_method(&mut body, 0, 1, &[]);
    assert_eq!(&body[..2], &[0xDE, 0xAD]); // prefix preserved
    assert_eq!(body[2], 0x80); // method appended after
}
