//! Helpers for building encrypted Mercury packets from the BaseApp server side.
//!
//! These functions produce byte-identical wire output to the C++ BaseApp:
//! - [`build_connect_reply`]           — `BASEMSG_REPLY_MESSAGE` echoing the ticket back (Phase 3).
//! - [`build_time_sync`]               — three time-sync messages in one packet (Phase 3).
//! - [`build_char_list`]               — game-state + character list (Phase 4, dynamic count).
//! - [`build_ongoing_tick_sync`]       — single tick-sync for the 100 ms heartbeat.
//! - [`build_world_entry`]             — createBasePlayer + viewport + cell + position (Phase 5).
//! - [`build_char_create_failed`]      — `onCharacterCreateFailed` error response.
//! - [`build_resource_fragment`]       — `BASEMSG_RESOURCE_FRAGMENT` for cooked data serving.
//! - [`build_version_info`]            — `onVersionInfo` for client cache version queries.
//!
//! Each function returns a `Vec<u8>` ready to write to the UDP socket.

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{
    build_outgoing, FLAG_HAS_ACKS, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE,
};

/// Server→client reply flags (HAS_SEQUENCE | ON_CHANNEL | RELIABLE = 0x58).
const REPLY_FLAGS: u8 = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_RELIABLE;

// ── Message IDs ───────────────────────────────────────────────────────────────

/// `BASEMSG_REPLY_MESSAGE` — reply to a client request (message 0xFF).
const BASEMSG_REPLY_MESSAGE: u8 = 0xFF;
/// `BASEMSG_UPDATE_FREQUENCY_NOTIFICATION` — tick update frequency (0x02).
const BASEMSG_UPDATE_FREQUENCY_NOTIFICATION: u8 = 0x02;
/// `BASEMSG_TICK_SYNC` — current tick counter and rate (0x0D).
const BASEMSG_TICK_SYNC: u8 = 0x0D;
/// `BASEMSG_SET_GAME_TIME` — set client game clock (0x03).
const BASEMSG_SET_GAME_TIME: u8 = 0x03;
/// `BASEMSG_CREATE_BASE_PLAYER` — create a base entity on the client (0x05).
/// Wire format: `[entityID:u32][classID:u8][propertyCount:u8]`.
const BASEMSG_CREATE_BASE_PLAYER: u8 = 0x05;
/// Base entity method: `onCharacterList` (msg_id = 0x80 + methodId 2 = 0x82).
/// Wire format: `[entityID:u32][ARRAY<CharacterInfo>]`.
const BASEMSG_ON_CHARACTER_LIST: u8 = 0x82;
/// Base entity method: `onCharacterCreateFailed` (msg_id = 0x80 + methodId 3 = 0x83).
/// Wire format: `[entityID:u32][errorCode:i32]`.
const BASEMSG_ON_CHARACTER_CREATE_FAILED: u8 = 0x83;
/// Base entity method: `onCharacterVisuals` (msg_id = 0x80 + methodId 4 = 0x84).
/// Wire format: `[entityID:u32][playerId:i32][bodySet:WSTRING][components:ARRAY<WSTRING>][primaryTint:u32][secondaryTint:u32][skinTint:u32]`.
const BASEMSG_ON_CHARACTER_VISUALS: u8 = 0x84;
/// Base entity method: `onVersionInfo` (msg_id = 0x80 + methodId 0 = 0x80).
/// Wire format: `[entityID:u32][categoryId:u32][version:u32][requiredUpdates:u32][invalidateAll:u8][invalidKeys:ARRAY]`.
const BASEMSG_ON_VERSION_INFO: u8 = 0x80;

/// `BASEMSG_SPACE_VIEWPORT_INFO` — CME-custom space/viewport setup (0x08).
/// Wire format: `[entityID:u32][entityID2:u32][spaceID:u32][viewportID:u8]`.
const BASEMSG_SPACE_VIEWPORT_INFO: u8 = 0x08;
/// `BASEMSG_CREATE_CELL_PLAYER` — create cell entity with position (0x06).
/// Wire format: `[spaceID:u32][vehicleID:u32][pos:3×f32][rot:3×f32]`.
const BASEMSG_CREATE_CELL_PLAYER: u8 = 0x06;
/// `BASEMSG_FORCED_POSITION` — authoritative position set (0x31).
/// Wire format: `[entityID:u32][spaceID:u32][vehicleID:u32][pos:3×f32][vel:3×f32][rot:3×f32][flags:u8]`.
const BASEMSG_FORCED_POSITION: u8 = 0x31;
/// `BASEMSG_RESET_ENTITIES` — tear down client entity system (0x04, CONSTANT_LENGTH = 1).
/// Wire format: `[keepBase:u8]` (always 0).
const BASEMSG_RESET_ENTITIES: u8 = 0x04;
/// `BASEMSG_RESOURCE_FRAGMENT` — cooked data fragment (0x36, VARIABLE_LENGTH_MESSAGE).
const BASEMSG_RESOURCE_FRAGMENT: u8 = 0x36;
/// `BASEMSG_LOGGED_OFF` — server tells client the session is terminated (0x37, CONSTANT_LENGTH = 1).
/// Wire format: `[reason:u8]` (0 = normal logoff).
const BASEMSG_LOGGED_OFF: u8 = 0x37;

/// Account entity class ID (EntityTypeID 7 in entity definitions).
const ACCOUNT_CLASS_ID: u8 = 0x07;
/// SGWPlayer entity class ID (EntityTypeID 2 in entity definitions).
const SGWPLAYER_CLASS_ID: u8 = 0x02;
/// Default space ID for CombatSim (matches reference server pcap: 0x10010 = 65552).
pub const DEFAULT_SPACE_ID: u32 = 65552;

// ── Public types ─────────────────────────────────────────────────────────────

/// A character entry for the `onCharacterList` response.
///
/// Fields match the `CharacterInfo` FIXED_DICT from `custom_alias.xml`:
/// `playerId`, `name`, `extraName`, `alignment`, `level`, `gender`,
/// `worldLocation`, `archetype`, `title`, `playerType`, `playable`.
#[derive(Debug, Clone)]
pub struct CharacterInfo {
    pub player_id: i32,
    pub name: String,
    pub extra_name: String,
    pub alignment: u8,
    pub level: u8,
    pub gender: u8,
    pub world_location: String,
    pub archetype: u8,
    pub title: u8,
    pub player_type: i32,
    pub playable: u8,
}

/// Data for world entry (Phase 5) pulled from the database.
#[derive(Debug, Clone)]
pub struct WorldEntryInfo {
    pub player_entity_id: u32,
    pub space_id: u32,
    pub pos: [f32; 3],
    pub rot: [f32; 3],
}

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
    const TICK_RATE: u32 = 100;

    let mut body = Vec::with_capacity(9);
    body.push(BASEMSG_TICK_SYNC);
    body.extend_from_slice(&tick.to_le_bytes());
    body.extend_from_slice(&TICK_RATE.to_le_bytes());

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt Phase 5a: spatial setup + entity system reset.
///
/// Matches the C++ `createCellPlayer()` + `switchEntity()` flow:
/// 1. `SPACE_VIEWPORT_INFO` — set up viewport for the player entity
/// 2. `CREATE_CELL_PLAYER` — create cell entity with position
/// 3. `FORCED_POSITION` — authoritative position lock
/// 4. `RESET_ENTITIES` — tear down old entity system (Account)
///
/// After the client processes `RESET_ENTITIES`, it sends `ENABLE_ENTITIES`
/// (0x08), and the server responds with `build_create_base_player`.
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
    let mut body = Vec::with_capacity(2);
    body.push(BASEMSG_LOGGED_OFF);
    body.push(0x00); // reason = 0 (normal logoff)

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt Phase 5b: CREATE_BASE_PLAYER + viewport + cell + forced position.
///
/// Sent in response to `ENABLE_ENTITIES` (0x08) from the client after it
/// processes the `RESET_ENTITIES` from Phase 5a.  The C++ server triggers
/// the cell data via `sendConnectEntity` inside `enableEntities()` — we
/// combine all messages into one packet since we're single-process.
pub fn build_world_entry(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);

    // 1. CREATE_BASE_PLAYER for SGWPlayer (WORD_LENGTH = 6)
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.push(SGWPLAYER_CLASS_ID);
    body.push(0x00); // propertyCount = 0

    // 2. spaceViewportInfo (0x08, CONSTANT_LENGTH = 13)
    body.push(BASEMSG_SPACE_VIEWPORT_INFO);
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.push(0x00); // viewportID = 0

    // 3. createCellPlayer (0x06, WORD_LENGTH = 32)
    body.push(BASEMSG_CREATE_CELL_PLAYER);
    body.extend_from_slice(&32u16.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &info.pos {
        body.extend_from_slice(&c.to_le_bytes());
    }
    // C++ sends rotX, rotZ, rotY (Y and Z swapped in wire format)
    body.extend_from_slice(&info.rot[0].to_le_bytes()); // rotX
    body.extend_from_slice(&info.rot[2].to_le_bytes()); // rotZ (swapped)
    body.extend_from_slice(&info.rot[1].to_le_bytes()); // rotY (swapped)

    // 4. forcedPosition (0x31, CONSTANT_LENGTH = 49)
    body.push(BASEMSG_FORCED_POSITION);
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &info.pos {
        body.extend_from_slice(&c.to_le_bytes());
    }
    for &c in &[0.0f32, 0.0, 0.0] {
        body.extend_from_slice(&c.to_le_bytes());
    } // velocity = zero
    // C++ sends rotX, rotZ, rotY (swapped)
    body.extend_from_slice(&info.rot[0].to_le_bytes()); // rotX
    body.extend_from_slice(&info.rot[2].to_le_bytes()); // rotZ (swapped)
    body.extend_from_slice(&info.rot[1].to_le_bytes()); // rotY (swapped)
    body.push(0x01); // flags

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

/// Resource fragment flags.
pub const FRAG_FIRST: u8 = 0x41;
pub const FRAG_MIDDLE: u8 = 0x40;
pub const FRAG_LAST: u8 = 0x42;
pub const FRAG_FIRST_AND_LAST: u8 = 0x43;

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

// ── Serialization helpers ────────────────────────────────────────────────────

/// Write a BigWorld `WSTRING` to a buffer.
///
/// Wire format: `[char_count: u32 LE][UTF-16LE data: char_count × 2 bytes]`.
fn write_wstring(buf: &mut Vec<u8>, s: &str) {
    let chars: Vec<u16> = s.encode_utf16().collect();
    buf.extend_from_slice(&(chars.len() as u32).to_le_bytes());
    for &ch in &chars {
        buf.extend_from_slice(&ch.to_le_bytes());
    }
}

/// Read a BigWorld `WSTRING` from a buffer at a given offset.
///
/// Returns `(decoded_string, bytes_consumed)`.
pub fn read_wstring(buf: &[u8], offset: usize) -> Result<(String, usize), String> {
    if offset + 4 > buf.len() {
        return Err("WSTRING: not enough bytes for char_count".into());
    }
    let char_count = u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]) as usize;
    let data_start = offset + 4;
    let data_len = char_count * 2;
    if data_start + data_len > buf.len() {
        return Err(format!(
            "WSTRING: need {} bytes for {} chars, have {}",
            data_len,
            char_count,
            buf.len() - data_start
        ));
    }
    let mut chars = Vec::with_capacity(char_count);
    for i in 0..char_count {
        let lo = buf[data_start + i * 2];
        let hi = buf[data_start + i * 2 + 1];
        chars.push(u16::from_le_bytes([lo, hi]));
    }
    let s = String::from_utf16(&chars).map_err(|e| format!("WSTRING: invalid UTF-16: {e}"))?;
    Ok((s, 4 + data_len))
}

// ── Encryption helper ─────────────────────────────────────────────────────────

/// Encrypt a plaintext Mercury packet (flags + body + footers).
fn encrypt_packet(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let enc = MercuryEncryption::from_session_key(*key);
    enc.encrypt(plaintext)
        .expect("Mercury packet encryption failed")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut buf = Vec::new();
        write_wstring(&mut buf, "Hello");
        let (s, consumed) = read_wstring(&buf, 0).unwrap();
        assert_eq!(s, "Hello");
        assert_eq!(consumed, 4 + 5 * 2); // 4 byte count + 5 UTF-16LE chars
    }

    #[test]
    fn read_wstring_empty() {
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
}
