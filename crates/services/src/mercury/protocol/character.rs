//! Character-list and character-lifecycle builders: char list (with and without
//! CREATE_BASE_PLAYER), char create failure, and character visuals.
//!
//! All of these target the Account entity on the client, sent during the
//! character-select phase before world entry.

use cimmeria_mercury::packet::FLAG_HAS_ACKS;

use super::{
    encrypt_packet, write_wstring, REPLY_FLAGS,
    BASEMSG_CREATE_BASE_PLAYER, BASEMSG_ON_CHARACTER_LIST,
    BASEMSG_ON_CHARACTER_CREATE_FAILED, BASEMSG_ON_CHARACTER_VISUALS,
    ACCOUNT_CLASS_ID, CharacterInfo,
};

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
