//! Session/phase builders: connect, time sync, char list, tick sync, reset,
//! logged off, char create failed, character visuals, resource fragment, version info.

use cimmeria_mercury::packet::FLAG_HAS_ACKS;

use super::{
    encrypt_packet, write_wstring, REPLY_FLAGS,
    BASEMSG_REPLY_MESSAGE, BASEMSG_UPDATE_FREQUENCY_NOTIFICATION,
    BASEMSG_TICK_SYNC, BASEMSG_SET_GAME_TIME, BASEMSG_CREATE_BASE_PLAYER,
    BASEMSG_ON_CHARACTER_LIST, BASEMSG_ON_CHARACTER_CREATE_FAILED,
    BASEMSG_ON_CHARACTER_VISUALS, BASEMSG_ON_VERSION_INFO,
    BASEMSG_RESET_ENTITIES, BASEMSG_RESOURCE_FRAGMENT, BASEMSG_LOGGED_OFF,
    ACCOUNT_CLASS_ID,
};
use super::types::CharacterInfo;

// ── Public builders ───────────────────────────────────────────────────────────

/// Build and encrypt the `BASEMSG_REPLY_MESSAGE` packet.
///
/// This is the server's response to the `baseAppLogin` connect request.
/// The reply echoes `request_id` and the 20-byte `ticket` back to the client
/// so it can verify the server is legitimate.
pub fn build_connect_reply(
    request_id: u32,
    ticket: &[u8],
    key: &[u8; 32],
    seq_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    assert_eq!(ticket.len(), 20, "ticket must be exactly 20 bytes");

    let mut body = Vec::with_capacity(1 + 4 + 4 + 1 + 20);
    body.push(BASEMSG_REPLY_MESSAGE);
    body.extend_from_slice(&25u32.to_le_bytes()); // DWORD_LENGTH = 25
    body.extend_from_slice(&request_id.to_le_bytes());
    body.push(ticket.len() as u8); // ticketLen = 20
    body.extend_from_slice(ticket);

    let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(seq_id), &[], None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the time-sync bundle packet.
///
/// Packs three constant-length messages into one packet, matching the C++
/// `ClientHandler::onConnected()` sequence.
pub fn build_time_sync(key: &[u8; 32], seq_id: u32) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    const UPDATE_FREQ: u8 = 10;
    const TICK_RATE: u32 = 100;
    const TICKS: u32 = 0;

    let mut body = Vec::with_capacity(2 + 9 + 5);

    body.push(BASEMSG_UPDATE_FREQUENCY_NOTIFICATION);
    body.push(UPDATE_FREQ);

    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&TICKS.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    body.push(BASEMSG_SET_GAME_TIME);
    body.extend_from_slice(&TICKS.to_le_bytes());

    let plaintext = build_outgoing(REPLY_FLAGS, &body, Some(seq_id), &[], None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt the Phase 4 character list response packet.
///
/// Sent immediately after the client confirms login with msg_id=0x01.
/// Contains two messages:
///
/// 1. `BASEMSG_CREATE_BASE_PLAYER` (0x05) — creates the Account entity.
/// 2. `BASEMSG_ON_CHARACTER_LIST` (0x82) — character list.
///
/// If `characters` is empty, the client shows the character creation screen.
/// If non-empty, the client shows the character select screen.
pub fn build_char_list(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    characters: &[CharacterInfo],
    account_entity_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(128);

    // BASEMSG_CREATE_BASE_PLAYER (WORD_LENGTH = 6)
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&account_entity_id.to_le_bytes());
    body.push(ACCOUNT_CLASS_ID);
    body.push(0x00); // propertyCount = 0

    // BASEMSG_ON_CHARACTER_LIST
    body.push(BASEMSG_ON_CHARACTER_LIST);

    // Build payload: [entityID][ARRAY<CharacterInfo>]
    let mut payload = Vec::with_capacity(80 * characters.len().max(1));
    payload.extend_from_slice(&account_entity_id.to_le_bytes());

    // Array count
    payload.extend_from_slice(&(characters.len() as u32).to_le_bytes());

    // Serialize each CharacterInfo FIXED_DICT
    for ch in characters {
        payload.extend_from_slice(&ch.player_id.to_le_bytes());
        write_wstring(&mut payload, &ch.name);
        write_wstring(&mut payload, &ch.extra_name);
        payload.push(ch.alignment);
        payload.push(ch.level);
        payload.push(ch.gender);
        write_wstring(&mut payload, &ch.world_location);
        payload.push(ch.archetype);
        payload.push(ch.title);
        payload.extend_from_slice(&ch.player_type.to_le_bytes());
        payload.push(ch.playable);
    }

    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt an `onCharacterList` entity method call (without CREATE_BASE_PLAYER).
///
/// Used after character creation/deletion to refresh the list on an already-existing
/// Account entity.  The initial login uses [`build_char_list`] which includes
/// CREATE_BASE_PLAYER.
pub fn build_on_character_list(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    characters: &[CharacterInfo],
    account_entity_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(128);

    // BASEMSG_ON_CHARACTER_LIST (entity method call, no CREATE_BASE_PLAYER)
    body.push(BASEMSG_ON_CHARACTER_LIST);

    // Build payload: [entityID][ARRAY<CharacterInfo>]
    let mut payload = Vec::with_capacity(80 * characters.len().max(1));
    payload.extend_from_slice(&account_entity_id.to_le_bytes());

    // Array count
    payload.extend_from_slice(&(characters.len() as u32).to_le_bytes());

    // Serialize each CharacterInfo FIXED_DICT
    for ch in characters {
        payload.extend_from_slice(&ch.player_id.to_le_bytes());
        write_wstring(&mut payload, &ch.name);
        write_wstring(&mut payload, &ch.extra_name);
        payload.push(ch.alignment);
        payload.push(ch.level);
        payload.push(ch.gender);
        write_wstring(&mut payload, &ch.world_location);
        payload.push(ch.archetype);
        payload.push(ch.title);
        payload.extend_from_slice(&ch.player_type.to_le_bytes());
        payload.push(ch.playable);
    }

    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a single `BASEMSG_TICK_SYNC` heartbeat packet.
pub fn build_ongoing_tick_sync(key: &[u8; 32], seq_id: u32, tick: u32, acks: &[u32]) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    const TICK_RATE: u32 = 100;

    let mut body = Vec::with_capacity(9);
    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&tick.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt Phase 5a: RESET_ENTITIES only.
///
/// The C++ server sends RESET_ENTITIES in its own flushed bundle, separate from
/// the cell/viewport data.  The client tears down all entities, then sends
/// ENABLE_ENTITIES, at which point Phase 5b fires.
pub fn build_reset_entities(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(4);
    body.push(BASEMSG_RESET_ENTITIES);
    body.push(0x00); // keepBase = false

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a `LOGGED_OFF` message (0x37, CONSTANT_LENGTH = 1).
///
/// Sent when the client calls `logOff` (0xC2) — tells the client the server
/// has terminated the session.  The client calls `EntityManager::loggedOff()`
/// which does a partial cleanup (clears game entities, keeps login-screen
/// entities) and then tears down the Mercury channel.
///
/// C++ reference: `client_handler.cpp:461` — `BASEMSG_LOGGED_OFF` with
/// `reason = 0` followed by `flushBundle` + `channel->condemn()`.
pub fn build_logged_off(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(2);
    body.push(BASEMSG_LOGGED_OFF);
    body.push(0x00); // reason = 0 (normal logoff)

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `onCharacterCreateFailed` (0x83).
///
/// Sent when character creation fails (duplicate name, invalid data, etc.).
/// Error codes: 1 = name taken, 2 = invalid data, 3 = DB error.
pub fn build_char_create_failed(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    error_code: i32,
    account_entity_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(16);

    body.push(BASEMSG_ON_CHARACTER_CREATE_FAILED);
    // WORD_LENGTH = 8: entityID(4) + errorCode(4)
    body.extend_from_slice(&8u16.to_le_bytes());
    body.extend_from_slice(&account_entity_id.to_le_bytes());
    body.extend_from_slice(&error_code.to_le_bytes());

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `onCharacterVisuals` (0x84).
///
/// Sent in response to `requestCharacterVisuals` so the client can render the
/// character model on the select screen.
pub fn build_character_visuals(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    player_id: i32,
    bodyset: &str,
    components: &[String],
    primary_tint: u32,
    secondary_tint: u32,
    skin_tint: u32,
    account_entity_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut body = Vec::with_capacity(128);

    body.push(BASEMSG_ON_CHARACTER_VISUALS);

    // Reserve 2 bytes for WORD_LENGTH (fill in at the end).
    let wl_pos = body.len();
    body.extend_from_slice(&0u16.to_le_bytes());

    let wl_start = body.len();

    // entityID
    body.extend_from_slice(&account_entity_id.to_le_bytes());

    // playerId: INT32
    body.extend_from_slice(&player_id.to_le_bytes());

    // bodySet: WSTRING
    write_wstring(&mut body, bodyset);

    // components: ARRAY<WSTRING> — u32 count, then each WSTRING
    body.extend_from_slice(&(components.len() as u32).to_le_bytes());
    for comp in components {
        write_wstring(&mut body, comp);
    }

    // primaryTint: UINT32
    body.extend_from_slice(&primary_tint.to_le_bytes());
    // secondaryTint: UINT32
    body.extend_from_slice(&secondary_tint.to_le_bytes());
    // skinTint: UINT32
    body.extend_from_slice(&skin_tint.to_le_bytes());

    // Patch WORD_LENGTH.
    let word_len = (body.len() - wl_start) as u16;
    body[wl_pos..wl_pos + 2].copy_from_slice(&word_len.to_le_bytes());

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt a `BASEMSG_RESOURCE_FRAGMENT` (0x36).
///
/// Wire format (VARIABLE_LENGTH_MESSAGE):
/// ```text
/// [dataId: u16]     — increments per resource transfer
/// [chunkId: u8]     — 0, 1, 2, ... fragment sequence
/// [flags: u8]       — 0x41=first, 0x40=middle, 0x42=last, 0x43=first+last
/// [msgType: u8]     — 0 (MESSAGE_CacheData), only in FIRST fragment
/// [categoryId: u32] — e.g. 7 (char_creation), only in FIRST fragment
/// [elementId: u32]  — e.g. CharDefId (1-23), only in FIRST fragment
/// [xmlBody: bytes]  — raw UTF-8 XML chunk
/// ```
pub fn build_resource_fragment(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    data_id: u16,
    chunk_id: u8,
    frag_flags: u8,
    msg_type: Option<u8>,
    category_id: Option<u32>,
    element_id: Option<u32>,
    xml_chunk: &[u8],
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut payload = Vec::with_capacity(64 + xml_chunk.len());

    // Fixed header
    payload.extend_from_slice(&data_id.to_le_bytes());
    payload.push(chunk_id);
    payload.push(frag_flags);

    // First-fragment-only fields
    if let Some(mt) = msg_type {
        payload.push(mt);
    }
    if let Some(cat) = category_id {
        payload.extend_from_slice(&cat.to_le_bytes());
    }
    if let Some(elem) = element_id {
        payload.extend_from_slice(&elem.to_le_bytes());
    }

    // XML data
    payload.extend_from_slice(xml_chunk);

    let mut body = Vec::with_capacity(3 + payload.len());
    body.push(BASEMSG_RESOURCE_FRAGMENT);
    // WORD_LENGTH message: u16 length prefix (matches messages.cpp:373)
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `onVersionInfo` (0x80) for a ClientCache method call.
///
/// Tells the client about the current version of a resource category.
/// If `invalidate_all` is true, the client will re-request all elements.
pub fn build_version_info(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    category_id: u32,
    version: u32,
    required_updates: u32,
    invalidate_all: bool,
    account_entity_id: u32,
) -> Vec<u8> {
    use cimmeria_mercury::packet::build_outgoing;

    let mut payload = Vec::with_capacity(32);
    payload.extend_from_slice(&account_entity_id.to_le_bytes());
    payload.extend_from_slice(&category_id.to_le_bytes());
    payload.extend_from_slice(&version.to_le_bytes());
    payload.extend_from_slice(&required_updates.to_le_bytes());
    payload.push(if invalidate_all { 1 } else { 0 });
    // invalidKeys = empty ARRAY<u32>
    payload.extend_from_slice(&0u32.to_le_bytes()); // count = 0

    let mut body = Vec::with_capacity(4 + payload.len());
    body.push(BASEMSG_ON_VERSION_INFO);
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{FRAG_FIRST_AND_LAST, BASEMSG_RESOURCE_FRAGMENT, BASEMSG_LOGGED_OFF};
    use cimmeria_mercury::encryption::MercuryEncryption;

    const TEST_KEY: [u8; 32] = [0x42u8; 32];

    #[test]
    fn connect_reply_size() {
        let ticket = b"12345678901234567890";
        let out = build_connect_reply(0xDEADBEEF, ticket, &TEST_KEY, 1);
        assert_eq!(out.len(), 64, "reply should be 64 bytes: 48 ciphertext + 16 HMAC");
    }

    #[test]
    fn time_sync_size() {
        let out = build_time_sync(&TEST_KEY, 2);
        assert_eq!(out.len(), 48, "time sync should be 48 bytes: 32 ciphertext + 16 HMAC");
    }

    #[test]
    fn connect_reply_deterministic() {
        let ticket = b"AABBCCDDEEFF00112233";
        let a = build_connect_reply(1, ticket, &TEST_KEY, 1);
        let b = build_connect_reply(1, ticket, &TEST_KEY, 1);
        assert_eq!(a, b, "same inputs → same encrypted output (deterministic CBC with zero IV)");
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
        assert_ne!(a, b, "different tick values must produce different ciphertexts");
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
            1,    // data_id
            0,    // chunk_id
            FRAG_FIRST_AND_LAST,
            Some(0),    // msg_type = MESSAGE_CacheData
            Some(7),    // category_id = char_creation
            Some(1),    // element_id
            xml,
        );
        assert!(!out.is_empty());
    }

    #[test]
    fn version_info_produces_output() {
        let out = build_version_info(&TEST_KEY, 5, &[], 7, 1, 23, true, 1);
        assert!(!out.is_empty());
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
            1,    // data_id
            0,    // chunk_id
            FRAG_FIRST_AND_LAST,
            Some(0),  // msg_type
            Some(7),  // category_id
            Some(1),  // element_id
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
        //                   categoryId(4) + elementId(4) + xml(22) = 35
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
        assert_eq!(plaintext[body_start], BASEMSG_LOGGED_OFF, "msg_id should be 0x37");
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
}
