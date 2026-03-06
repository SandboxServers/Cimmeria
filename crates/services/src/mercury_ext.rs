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

/// 16 ARGB skin tint values indexed by SkinTintColorID (0-15).
/// Source: `python/common/Constants.py:4-9` — `SKIN_TINTS` array.
pub const SKIN_TINTS: [u32; 16] = [
    0x2F1308FF, 0x180A08FF, 0x15100DFF, 0x9C4F22FF,
    0x370405FF, 0x2F1219FF, 0x6C1F0DFF, 0x4F1A09FF,
    0xB45B32FF, 0x632319FF, 0x3A2417FF, 0xF8B487FF,
    0xD57D51FF, 0xC36141FF, 0xDF8250FF, 0x8D3F24FF,
];

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

/// Archetype base stat values (from `db/resources/Archetypes/Seed/archetypes.sql`).
#[derive(Debug, Clone)]
pub struct ArchetypeStats {
    pub coordination: i32,
    pub engagement: i32,
    pub fortitude: i32,
    pub morale: i32,
    pub perception: i32,
    pub intelligence: i32,
    pub health: i32,
    pub focus: i32,
    pub health_per_level: i32,
    pub focus_per_level: i32,
}

/// Data loaded from the database for a player entering the world.
/// Used by [`build_map_loaded`] to build the complete `mapLoaded()` packet sequence.
#[derive(Debug, Clone)]
pub struct PlayerLoadData {
    pub level: i32,
    pub player_name: String,
    pub extra_name: String,
    pub alignment: i32,
    pub archetype: i32,
    pub gender: i32,
    pub bodyset: String,
    pub components: Vec<String>,
    pub exp: i32,
    pub naquadah: i32,
    pub known_stargates: Vec<i32>,
    pub abilities: Vec<i32>,
    pub training_points: i32,
    pub applied_science_points: i32,
    pub blueprint_ids: Vec<i32>,
    pub first_login: i32,
    pub access_level: i32,
    pub skin_color_id: i32,
    /// Ability tree data (3 branches). Loaded from `resources.archetype_ability_tree`.
    pub ability_tree: cimmeria_entity::abilities::AbilityTreeData,
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

/// `BASEMSG_CREATE_ENTITY` — create a ghost (non-player) entity on the client (0x09).
/// Sent when an entity enters a player's Area of Interest.
/// Wire: `[msg_id:0x09][wordLen:u16=5][entityId:u32][idAlias:0xFF][classId:u8][0x00][0x00]`
const BASEMSG_CREATE_ENTITY: u8 = 0x09;
/// `BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` — position update
/// for ghost entities (0x10, CONSTANT_LENGTH = 25).
const BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR: u8 = 0x10;
/// `BASEMSG_ENTITY_INVISIBLE` — mark entity invisible before removal (0x0B, CONSTANT_LENGTH = 5).
const BASEMSG_ENTITY_INVISIBLE: u8 = 0x0B;
/// `BASEMSG_LEAVE_AOI` — remove entity from client's AoI (0x0C, WORD_LENGTH).
const BASEMSG_LEAVE_AOI: u8 = 0x0C;

/// Resource fragment flags.
pub const FRAG_FIRST: u8 = 0x41;
pub const FRAG_MIDDLE: u8 = 0x40;
pub const FRAG_LAST: u8 = 0x42;
pub const FRAG_FIRST_AND_LAST: u8 = 0x43;

// ── SGWPlayer flattened ClientMethod indices ─────────────────────────────────
//
// Verified from .def files by traversing the entity hierarchy:
// SGWEntity → SGWSpawnableEntity → SGWBeing (+ interfaces) → SGWPlayer (+ interfaces + own)
//
// Direct encoding (0–127): msg_id = index | 0x80
// Extended encoding (128+): msg_id = 0xBD, payload = entity_id + (index - 61) as u8 + args

/// SGWPlayer flattened ClientMethod indices.
pub mod method_idx {
    // SGWSpawnableEntity own (0–11)
    pub const ON_ENTITY_PROPERTY: u16 = 7;
    pub const ON_KISMET_EVENT_SET_UPDATE: u16 = 9;
    pub const ON_ENTITY_TINT: u16 = 10;

    // SGWBeing interface (12–19)
    pub const ON_LEVEL_UPDATE: u16 = 15;
    pub const ON_TARGET_UPDATE: u16 = 16;
    pub const ON_BEING_NAME_UPDATE: u16 = 17;
    pub const ON_STATE_FIELD_UPDATE: u16 = 19;

    // SGWCombatant interface (20–26)
    pub const ON_STAT_UPDATE: u16 = 20;
    pub const ON_STAT_BASE_UPDATE: u16 = 21;
    pub const ON_ARCHETYPE_UPDATE: u16 = 23;
    pub const ON_ALIGNMENT_UPDATE: u16 = 24;
    pub const ON_FACTION_UPDATE: u16 = 25;
    pub const BEING_APPEARANCE: u16 = 26;

    // Communicator interface (27–33)
    pub const ON_PLAYER_COMMUNICATION: u16 = 28;
    pub const ON_CHAT_JOINED: u16 = 31;
    pub const ON_CHAT_LEFT: u16 = 32;

    // GateTravel interface (65–68)
    pub const SETUP_STARGATE_INFO: u16 = 65;

    // SGWInventoryManager interface (69–75)
    pub const ON_BAG_INFO: u16 = 69;
    pub const ON_CASH_CHANGED: u16 = 75;

    // SGWMailManager interface (76–79)
    pub const ON_MAIL_HEADER_INFO: u16 = 76;
    pub const ON_MAIL_HEADER_REMOVE: u16 = 77;
    pub const ON_MAIL_READ: u16 = 78;
    pub const SEND_MAIL_RESULT: u16 = 79;

    // SGWPlayer own methods (base offset 98)
    pub const ON_KNOWN_ABILITIES_UPDATE: u16 = 101;
    pub const ON_TIME_OF_DAY: u16 = 102;
    pub const ON_PLAYER_DATA_LOADED: u16 = 115;
    pub const SETUP_WORLD_PARAMETERS: u16 = 122;
    pub const CLEAR_HINTED_REGIONS: u16 = 124;
    pub const ON_RESET_MAP_INFO: u16 = 126;

    // Extended encoding (≥ 128)
    pub const ON_EXTRA_NAME_UPDATE: u16 = 130;
    pub const ON_EXP_UPDATE: u16 = 131;
    pub const ON_MAX_EXP_UPDATE: u16 = 132;
    pub const ON_UPDATE_KNOWN_CRAFTS: u16 = 139;
    pub const ON_ABILITY_TREE_INFO: u16 = 141;
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

// ── AoI packet builders ─────────────────────────────────────────────────────

/// Build and encrypt `CREATE_ENTITY (0x09)` + `UPDATE_AVATAR (0x10)` for when
/// an entity enters a witness's Area of Interest.
///
/// Matches C++ `ClientHandler::createEntity()` + `moveEntity()` from
/// `client_handler.cpp:492-504`.
pub fn build_create_entity(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    class_id: u8,
    position: [f32; 3],
    direction: [f32; 3],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(64);

    // CREATE_ENTITY (0x09, WORD_LENGTH)
    body.push(BASEMSG_CREATE_ENTITY);
    body.extend_from_slice(&5u16.to_le_bytes()); // wordLength = 5
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias
    body.push(class_id);
    body.push(0x00); // unknown1
    body.push(0x00); // unknown2

    // UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10, CONSTANT_LENGTH = 25)
    body.push(BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
    body.extend_from_slice(&entity_id.to_le_bytes());
    for &c in &position {
        body.extend_from_slice(&c.to_le_bytes());
    }
    // Velocity XYZ = zero → 5 zero bytes (packXYZ encodes (0,0,0) as all zeros)
    body.extend_from_slice(&[0u8; 5]);
    // Physics mode flags = 0x01 (standard walking)
    body.push(0x01);
    // Direction: yaw, pitch, roll — each u8, quantized to 256 steps per circle
    // rotation.y = yaw, rotation.x = pitch, rotation.z = roll (C++ convention)
    body.push(pack_angle(direction[1])); // yaw = rotation.y
    body.push(pack_angle(direction[0])); // pitch = rotation.x
    body.push(pack_angle(direction[2])); // roll = rotation.z

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `ENTITY_INVISIBLE (0x0B)` + `LEAVE_AOI (0x0C)` for when
/// an entity leaves a witness's Area of Interest.
///
/// Matches C++ `ClientHandler::leaveAoI()` from `client_handler.cpp:516-539`.
pub fn build_entity_leave(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(24);

    // ENTITY_INVISIBLE (0x0B, CONSTANT_LENGTH = 5)
    body.push(BASEMSG_ENTITY_INVISIBLE);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.push(0xFF); // idAlias = no alias

    // LEAVE_AOI (0x0C, WORD_LENGTH)
    body.push(BASEMSG_LEAVE_AOI);
    body.extend_from_slice(&8u16.to_le_bytes()); // wordLength = 8
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // cacheStamp = 0

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build and encrypt `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10)` for
/// relaying position updates to AoI witnesses.
///
/// Matches C++ `ClientHandler::moveEntity()` from `client_handler.cpp:542-556`.
pub fn build_avatar_update(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    position: [f32; 3],
    velocity: [f32; 3],
    direction: [f32; 3],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32);

    body.push(BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR);
    body.extend_from_slice(&entity_id.to_le_bytes());
    for &c in &position {
        body.extend_from_slice(&c.to_le_bytes());
    }
    // Pack velocity using the C++ packXYZ algorithm
    let packed = pack_velocity_xyz(velocity);
    body.extend_from_slice(&packed);
    // Physics mode flags = 0x01
    body.push(0x01);
    // Direction: yaw, pitch, roll
    body.push(pack_angle(direction[1])); // yaw
    body.push(pack_angle(direction[0])); // pitch
    body.push(pack_angle(direction[2])); // roll

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Pack a float angle (radians) into a single byte (256 steps per circle).
///
/// Matches C++ `(uint8_t)(angle / 0.024543693f)`.
fn pack_angle(radians: f32) -> u8 {
    const SCALE: f32 = 0.024543693;
    (radians / SCALE) as u8
}

/// Pack a velocity Vec3 into 5 bytes using the C++ `packXYZ` format.
///
/// Exact port of `ClientHandler::packXYZ()` from `client_handler.cpp:647-687`.
fn pack_velocity_xyz(v: [f32; 3]) -> [u8; 5] {
    let mut packed1: u32 = 0;
    let mut packed2: u8 = 0;

    // X component
    let x = if v[0] < 0.0 {
        packed1 |= 0x00800000;
        -v[0]
    } else {
        v[0]
    };
    let x_biased = x + 2.0;
    let x_bits = x_biased.to_bits();
    packed1 |= (x_bits >> 3) & 0x007FF000;

    // Z component
    let z = if v[2] < 0.0 {
        packed1 |= 0x00000800;
        -v[2]
    } else {
        v[2]
    };
    let z_biased = z + 2.0;
    let z_bits = z_biased.to_bits();
    packed1 |= (z_bits >> 15) & 0x000007FF;

    // Y component
    let y = if v[1] < 0.0 {
        packed2 |= 0x80;
        -v[1]
    } else {
        v[1]
    };
    let y_biased = y + 2.0;
    let y_bits = y_biased.to_bits();
    let y_delta = (y_bits >> 12) & 0x00007FFF;
    packed1 |= (y_delta & 0xFF) << 24;
    packed2 |= ((y_delta & 0x7F00) >> 8) as u8;

    let p1 = packed1.to_le_bytes();
    [p1[0], p1[1], p1[2], p1[3], packed2]
}

// ── Entity method encoding ───────────────────────────────────────────────────

/// Append a server→client entity method call to a Mercury message body.
///
/// Handles two encoding schemes per C++ `Bundle::beginEntityMessage()`:
/// - **Direct** (method_index 0–127): `[(index | 0x80): u8][word_len: u16][entity_id: u32][args…]`
/// - **Extended** (method_index ≥ 128): `[0xBD: u8][word_len: u16][entity_id: u32][(index − 61): u8][args…]`
///
/// The C++ server conservatively uses extended at ≥ 61, but the client accepts
/// direct encoding through index 127 (verified with setupWorldParameters=122
/// and onPlayerDataLoaded=115). We use the simpler boundary at 128.
pub fn append_entity_method(body: &mut Vec<u8>, method_index: u16, entity_id: u32, args: &[u8]) {
    if method_index >= 128 {
        // Extended encoding: marker 0xBD
        body.push(0xBD);
        let payload_len = (4 + 1 + args.len()) as u16; // entity_id + sub_index + args
        body.extend_from_slice(&payload_len.to_le_bytes());
        body.extend_from_slice(&entity_id.to_le_bytes());
        body.push((method_index - 61) as u8);
    } else {
        // Direct encoding: msg_id = index | 0x80
        body.push((method_index as u8) | 0x80);
        let payload_len = (4 + args.len()) as u16; // entity_id + args
        body.extend_from_slice(&payload_len.to_le_bytes());
        body.extend_from_slice(&entity_id.to_le_bytes());
    }
    body.extend_from_slice(args);
}

/// Look up archetype base stats by archetype ID.
///
/// Hardcoded from `db/resources/Archetypes/Seed/archetypes.sql`. All archetypes
/// except Commando share the same stat spread in the seed data.
pub fn archetype_stats(archetype_id: i32) -> ArchetypeStats {
    match archetype_id {
        2 => ArchetypeStats { // Commando (only one with different stats)
            coordination: 4, engagement: 4, fortitude: 2, morale: 3,
            perception: 5, intelligence: 3, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        },
        _ => ArchetypeStats { // Soldier, Scientist, Archeologist, Asgard, Goa'uld, Shol'va, Jaffa
            coordination: 5, engagement: 4, fortitude: 3, morale: 4,
            perception: 3, intelligence: 2, health: 760, focus: 1570,
            health_per_level: 10, focus_per_level: 70,
        },
    }
}

/// Ability tree data per archetype (from `db/resources/Archetypes/Seed/archetype_ability_tree.sql`).
///
/// Only Soldier (1) and Commando (2) have tree data in seed; others get empty trees.
pub fn archetype_ability_tree(archetype_id: i32) -> cimmeria_entity::abilities::AbilityTreeData {
    use cimmeria_entity::abilities::AbilityTreeData;
    match archetype_id {
        1 => AbilityTreeData { // Soldier
            trees: [
                // Tree 0 (29 abilities)
                vec![597,603,604,610,611,616,617,621,622,623,625,627,641,
                     643,645,648,650,652,661,662,663,666,667,668,672,675,
                     677,679,680],
                // Tree 1 (28 abilities)
                vec![598,599,605,606,612,613,618,619,624,626,628,629,642,
                     644,646,647,649,651,653,654,664,665,669,670,673,674,
                     676,678],
                // Tree 2 (27 abilities)
                vec![600,601,602,607,608,609,614,615,620,630,631,655,656,
                     657,658,659,660,671,681,682,683,684,685,686,687,688,689],
            ],
        },
        2 => AbilityTreeData { // Commando
            trees: [
                // Tree 0 (28 abilities)
                vec![700,706,707,713,714,720,721,726,727,731,732,733,735,
                     736,750,751,753,755,757,765,766,770,771,774,775,778,
                     780,781],
                // Tree 1 (30 abilities)
                vec![701,702,708,709,715,716,722,723,728,729,734,737,738,
                     739,752,754,756,758,759,767,768,769,772,773,776,777,
                     779,782,783,784],
                // Tree 2 (27 abilities)
                vec![703,704,705,710,711,712,717,718,719,724,725,730,740,
                     741,760,761,762,763,764,785,786,787,788,789,790,791,792],
            ],
        },
        _ => AbilityTreeData::default(), // Other archetypes: empty trees
    }
}

/// Total XP required to reach each level (from `python/common/Constants.py`).
const LEVEL_EXP: [i32; 21] = [
    0,
    // Level 1–10
    100, 200, 300, 600, 1000, 1600, 2500, 4000, 6000, 9000,
    // Level 11–20
    14000, 18000, 25000, 40000, 60000, 90000, 120000, 180000, 250000, 400000,
];

/// Get the XP required for a given level (clamped to table bounds).
fn level_exp(level: i32) -> i32 {
    let idx = (level as usize).min(LEVEL_EXP.len() - 1);
    LEVEL_EXP[idx]
}

// ── Cell→Client entity method calls (mapLoaded sequence) ─────────────────────

/// Cell→client entity method: `onPlayerDataLoaded` (no args).
///
/// Flattened ClientMethods index 115 → msg_id = 0xF3.
/// This is the signal that tells the client "all player data has been sent,
/// transition from loading to gameplay mode."
///
/// Wire format: `[msg_id:0xF3][word_len:u16=4][entity_id:u32]`
pub fn build_on_player_data_loaded(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    append_entity_method(&mut body, method_idx::ON_PLAYER_DATA_LOADED, entity_id, &[]);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Cell→client entity method: `setupWorldParameters` (22 args: 5×i32 + 17×f32).
///
/// Flattened ClientMethods index 122 → msg_id = 0xFA.
/// Sets world physics constants (gravity, movement speeds, etc.).
pub fn build_setup_world_parameters(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    let args = build_world_params_args();
    append_entity_method(&mut body, method_idx::SETUP_WORLD_PARAMETERS, entity_id, &args);

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Serialize setupWorldParameters argument payload (22 args from World.py defaults).
fn build_world_params_args() -> Vec<u8> {
    let mut args = Vec::with_capacity(88);
    args.extend_from_slice(&1i32.to_le_bytes());       // worldId
    args.extend_from_slice(&0i32.to_le_bytes());       // weatherSetId
    args.extend_from_slice(&1i32.to_le_bytes());       // minToRealMinutes
    args.extend_from_slice(&1440i32.to_le_bytes());    // minutesPerDay
    args.extend_from_slice(&100000i32.to_le_bytes());  // currentTimeInSeconds
    args.extend_from_slice(&(-9.8f32).to_le_bytes());  // gravity
    args.extend_from_slice(&6.0f32.to_le_bytes());     // runSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes());     // sidewaysRunSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes());     // backwardsRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // walkSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // sidewaysWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes());     // backwardsWalkSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes());     // crouchRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // sidewaysCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // backwardsCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes());     // crouchWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes());     // sidewaysCrouchWalkSpeed
    args.extend_from_slice(&0.75f32.to_le_bytes());    // backwardsCrouchWalkSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes());     // swimSpeed
    args.extend_from_slice(&2.5f32.to_le_bytes());     // sidewaysSwimSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes());     // backwardsSwimSpeed
    args.extend_from_slice(&8.0f32.to_le_bytes());     // jumpSpeed
    args
}

/// Build and encrypt the complete `mapLoaded()` packet sequence.
///
/// Sent after Phase 5b world entry. Contains all entity method calls needed to
/// initialize the client's game state: world parameters, stats, appearance,
/// Build and encrypt a single server→client entity method call.
///
/// Used by CellService to send entity method calls (e.g., `onTimerUpdate`,
/// `onEffectResults`) to a specific client.
pub fn build_entity_method_packet(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
    method_index: u16,
    args: &[u8],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(32 + args.len());
    append_entity_method(&mut body, method_index, entity_id, args);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}

/// abilities, inventory stubs, experience, and the final `onPlayerDataLoaded`
/// signal that transitions the client from loading screen to gameplay.
///
/// Mirrors `python/cell/SGWPlayer.py:464-546`.
pub fn build_map_loaded(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
    data: &PlayerLoadData,
) -> Vec<u8> {
    let stats = archetype_stats(data.archetype);
    let mut body = Vec::with_capacity(2048);

    // 1. setupWorldParameters (22 args: 5×i32 + 17×f32)
    append_entity_method(&mut body, method_idx::SETUP_WORLD_PARAMETERS, entity_id,
        &build_world_params_args());

    // 2. setupStargateInfo (3×ARRAY<INT32>: world, known, hidden)
    {
        let mut args = Vec::new();
        args.extend_from_slice(&0u32.to_le_bytes()); // worldStargateIds: empty
        args.extend_from_slice(&(data.known_stargates.len() as u32).to_le_bytes());
        for &sg in &data.known_stargates {
            args.extend_from_slice(&sg.to_le_bytes());
        }
        args.extend_from_slice(&0u32.to_le_bytes()); // hiddenStargates: empty
        append_entity_method(&mut body, method_idx::SETUP_STARGATE_INFO, entity_id, &args);
    }

    // 3. clearClientHintedGenericRegions (no args)
    append_entity_method(&mut body, method_idx::CLEAR_HINTED_REGIONS, entity_id, &[]);

    // 4. onTimeofDay(FLOAT32, FLOAT32, UINT8) — hardcoded sun position
    {
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&0.129059f32.to_le_bytes());
        args.extend_from_slice(&1.0f32.to_le_bytes());
        args.push(1u8);
        append_entity_method(&mut body, method_idx::ON_TIME_OF_DAY, entity_id, &args);
    }

    // 5. onLevelUpdate(INT32)
    append_entity_method(&mut body, method_idx::ON_LEVEL_UPDATE, entity_id,
        &data.level.to_le_bytes());

    // 6. onBeingNameUpdate(UNICODE_STRING)
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.player_name);
        append_entity_method(&mut body, method_idx::ON_BEING_NAME_UPDATE, entity_id, &args);
    }

    // 7. onStateFieldUpdate(UINT32) — default 0
    append_entity_method(&mut body, method_idx::ON_STATE_FIELD_UPDATE, entity_id,
        &0u32.to_le_bytes());

    // 8. onKismetEventSetUpdate(INT32) — default 1025
    append_entity_method(&mut body, method_idx::ON_KISMET_EVENT_SET_UPDATE, entity_id,
        &1025i32.to_le_bytes());

    // 9. sendStats: onStatUpdate + onStatBaseUpdate
    //    Uses StatList from entity crate — sends ALL stats, matching Python's
    //    `self.sendStats(self.client, False, False)` which sends everything.
    {
        use cimmeria_entity::stats::{StatList, ArchetypeStatValues};
        let mut stat_list = StatList::new();
        stat_list.apply_archetype(&ArchetypeStatValues {
            coordination: stats.coordination,
            engagement: stats.engagement,
            fortitude: stats.fortitude,
            morale: stats.morale,
            perception: stats.perception,
            intelligence: stats.intelligence,
            health: stats.health,
            focus: stats.focus,
        });
        // onStatUpdate: dynamic values (min, current, max)
        let stat_args = stat_list.serialize_all();
        append_entity_method(&mut body, method_idx::ON_STAT_UPDATE, entity_id, &stat_args);
        // onStatBaseUpdate: base values (same for fresh characters)
        let base_args = stat_list.serialize_all_base();
        append_entity_method(&mut body, method_idx::ON_STAT_BASE_UPDATE, entity_id, &base_args);
    }

    // 10. onArchetypeUpdate(INT32)
    append_entity_method(&mut body, method_idx::ON_ARCHETYPE_UPDATE, entity_id,
        &data.archetype.to_le_bytes());

    // 11. onAlignmentUpdate(INT8)
    append_entity_method(&mut body, method_idx::ON_ALIGNMENT_UPDATE, entity_id,
        &[data.alignment as u8]);

    // 12. onFactionUpdate(INT8) — hardcoded 3 (from setupPlayer)
    append_entity_method(&mut body, method_idx::ON_FACTION_UPDATE, entity_id, &[3u8]);

    // 13. onAbilityTreeInfo(ARRAY<ARRAY<INT32>>) — 3 ability tree branches
    //     Extended encoding (method_index 141 ≥ 128)
    {
        let tree_args = data.ability_tree.serialize();
        append_entity_method(&mut body, method_idx::ON_ABILITY_TREE_INFO, entity_id, &tree_args);
    }

    // 14. onKnownAbilitiesUpdate(ARRAY<INT32>)
    {
        let mut args = Vec::with_capacity(4 + data.abilities.len() * 4);
        args.extend_from_slice(&(data.abilities.len() as u32).to_le_bytes());
        for &id in &data.abilities {
            args.extend_from_slice(&id.to_le_bytes());
        }
        append_entity_method(&mut body, method_idx::ON_KNOWN_ABILITIES_UPDATE, entity_id, &args);
    }

    // 15. onResetMapInfo (no args)
    append_entity_method(&mut body, method_idx::ON_RESET_MAP_INFO, entity_id, &[]);

    // 16. BeingAppearance(UNICODE_STRING bodySet, ARRAY<UNICODE_STRING> components)
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.bodyset);
        args.extend_from_slice(&(data.components.len() as u32).to_le_bytes());
        for comp in &data.components {
            write_wstring(&mut args, comp);
        }
        append_entity_method(&mut body, method_idx::BEING_APPEARANCE, entity_id, &args);
    }

    // 17. onEntityTint(UINT32, UINT32, UINT32) — primary, secondary, skin
    {
        let skin_tint = if (data.skin_color_id as usize) < SKIN_TINTS.len() {
            SKIN_TINTS[data.skin_color_id as usize]
        } else {
            SKIN_TINTS[0]
        };
        let mut args = Vec::with_capacity(12);
        args.extend_from_slice(&0xFFu32.to_le_bytes());    // primaryColorId default
        args.extend_from_slice(&0xFFu32.to_le_bytes());    // secondaryColorId default
        args.extend_from_slice(&skin_tint.to_le_bytes());
        append_entity_method(&mut body, method_idx::ON_ENTITY_TINT, entity_id, &args);
    }

    // 18. onExtraNameUpdate(UNICODE_STRING) — extended encoding
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.extra_name);
        append_entity_method(&mut body, method_idx::ON_EXTRA_NAME_UPDATE, entity_id, &args);
    }

    // 19. onExpUpdate(INT32) — extended encoding
    append_entity_method(&mut body, method_idx::ON_EXP_UPDATE, entity_id,
        &data.exp.to_le_bytes());

    // 20. onMaxExpUpdate(INT32) — extended encoding
    append_entity_method(&mut body, method_idx::ON_MAX_EXP_UPDATE, entity_id,
        &level_exp(data.level).to_le_bytes());

    // 21. onEntityProperty ×6 (INT32 propId, INT32 value)
    //     GENERICPROPERTY IDs from Atrea.enums
    for &(prop_id, value) in &[
        (2i32, data.applied_science_points), // AppliedSciencePoints
        (1,    data.training_points),        // TrainingPoints
        (7,    data.access_level),           // AccessLevel
        (8,    data.gender),                 // Gender
        (4,    0),                           // PvPFlag
        (3,    0),                           // AmmoTypeId
    ] {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&prop_id.to_le_bytes());
        args.extend_from_slice(&value.to_le_bytes());
        append_entity_method(&mut body, method_idx::ON_ENTITY_PROPERTY, entity_id, &args);
    }

    // 22. Inventory: onBagInfo(ARRAY<BagInfo>) — single call with all bags
    {
        use cimmeria_entity::inventory::Inventory;
        let inv = Inventory::new(data.naquadah);
        let bag_info = inv.serialize_bag_info();
        append_entity_method(&mut body, method_idx::ON_BAG_INFO, entity_id, &bag_info);
    }

    // 23. onCashChanged(INT32) — naquadah
    append_entity_method(&mut body, method_idx::ON_CASH_CHANGED, entity_id,
        &data.naquadah.to_le_bytes());

    // 24. onUpdateKnownCrafts(ARRAY<INT32>) — extended encoding
    {
        let mut args = Vec::with_capacity(4 + data.blueprint_ids.len() * 4);
        args.extend_from_slice(&(data.blueprint_ids.len() as u32).to_le_bytes());
        for &bp in &data.blueprint_ids {
            args.extend_from_slice(&bp.to_le_bytes());
        }
        append_entity_method(&mut body, method_idx::ON_UPDATE_KNOWN_CRAFTS, entity_id, &args);
    }

    // 25. onChatJoined — notify client about default channels
    //     Reference: python/base/SGWPlayer.py onClientReady → ChannelManager.playerLoggedIn
    for &(channel_name, channel_id) in &[
        ("say", 0u8), ("emote", 1), ("yell", 2), ("team", 3),
        ("squad", 4), ("command", 5), ("server", 7), ("tell", 9),
    ] {
        let mut args = Vec::new();
        write_wstring(&mut args, channel_name);
        args.push(channel_id);
        append_entity_method(&mut body, method_idx::ON_CHAT_JOINED, entity_id, &args);
    }

    // 26. onPlayerDataLoaded (no args) — client transitions to gameplay
    append_entity_method(&mut body, method_idx::ON_PLAYER_DATA_LOADED, entity_id, &[]);

    // 26. onTargetUpdate(INT32) — default 0 (no target)
    append_entity_method(&mut body, method_idx::ON_TARGET_UPDATE, entity_id,
        &0i32.to_le_bytes());

    // 28. onPlayerCommunication(WSTRING speaker, UINT8 flags, UINT8 channel, WSTRING text)
    //     Welcome message on the feedback channel.
    {
        let mut args = Vec::new();
        write_wstring(&mut args, &data.player_name); // Speaker
        args.push(0u8); // SpeakerFlags
        args.push(8u8); // Channel = CHAN_FEEDBACK
        let welcome = format!(
            "Welcome to Stargate Worlds. Your player id is: {}.",
            entity_id
        );
        write_wstring(&mut args, &welcome); // Text
        append_entity_method(&mut body, method_idx::ON_PLAYER_COMMUNICATION, entity_id, &args);
    }

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
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

    // ── Entity method encoding tests ────────────────────────────────────────

    #[test]
    fn append_entity_method_direct_index_0() {
        let mut body = Vec::new();
        append_entity_method(&mut body, 0, 42, &[]);
        // msg_id = 0 | 0x80 = 0x80, word_len = 4, entity_id = 42
        assert_eq!(body[0], 0x80);
        assert_eq!(u16::from_le_bytes([body[1], body[2]]), 4);
        assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 42);
        assert_eq!(body.len(), 7);
    }

    #[test]
    fn append_entity_method_direct_index_127() {
        let mut body = Vec::new();
        append_entity_method(&mut body, 127, 1, &[0xAA, 0xBB]);
        // msg_id = 127 | 0x80 = 0xFF, word_len = 6, entity_id = 1, args
        assert_eq!(body[0], 0xFF);
        assert_eq!(u16::from_le_bytes([body[1], body[2]]), 6);
        assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 1);
        assert_eq!(&body[7..9], &[0xAA, 0xBB]);
    }

    #[test]
    fn append_entity_method_extended_index_128() {
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
        let mut body = vec![0xDE, 0xAD];
        append_entity_method(&mut body, 0, 1, &[]);
        assert_eq!(&body[..2], &[0xDE, 0xAD]); // prefix preserved
        assert_eq!(body[2], 0x80); // method appended after
    }

    #[test]
    fn build_on_player_data_loaded_uses_correct_msg_id() {
        let out = build_on_player_data_loaded(&TEST_KEY, 1, &[], 42);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&out).unwrap();
        // body starts at offset 1, msg_id should be 115 | 0x80 = 0xF3
        assert_eq!(pt[1], 0xF3);
    }

    #[test]
    fn build_setup_world_parameters_uses_correct_msg_id() {
        let out = build_setup_world_parameters(&TEST_KEY, 1, &[], 42);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&out).unwrap();
        // msg_id should be 122 | 0x80 = 0xFA
        assert_eq!(pt[1], 0xFA);
    }

    #[test]
    fn build_map_loaded_produces_output() {
        let data = PlayerLoadData {
            level: 1,
            player_name: "Test".into(),
            extra_name: String::new(),
            alignment: 1,
            archetype: 1,
            gender: 1,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: vec!["head_test".into()],
            exp: 0,
            naquadah: 0,
            known_stargates: vec![],
            abilities: vec![],
            training_points: 0,
            applied_science_points: 0,
            blueprint_ids: vec![],
            first_login: 1,
            access_level: 0,
            skin_color_id: 0,
            ability_tree: Default::default(),
        };
        let out = build_map_loaded(&TEST_KEY, 5, &[], 42, &data);
        assert!(!out.is_empty());
        // Should be significantly larger than the old 2-packet approach
        assert!(out.len() > 200, "mapLoaded packet should be substantial: {}", out.len());
    }

    #[test]
    fn build_map_loaded_contains_setup_world_params_and_player_data_loaded() {
        let data = PlayerLoadData {
            level: 5,
            player_name: "Warrior".into(),
            extra_name: "The Brave".into(),
            alignment: 2,
            archetype: 2,
            gender: 1,
            bodyset: "BS_HumanMale.BS_HumanMale".into(),
            components: vec![],
            exp: 500,
            naquadah: 100,
            known_stargates: vec![1, 2],
            abilities: vec![10, 20],
            training_points: 3,
            applied_science_points: 1,
            blueprint_ids: vec![],
            first_login: 0,
            access_level: 0,
            skin_color_id: 5,
            ability_tree: archetype_ability_tree(2),
        };
        let out = build_map_loaded(&TEST_KEY, 5, &[], 100, &data);
        let enc = MercuryEncryption::from_session_key(TEST_KEY);
        let pt = enc.decrypt(&out).unwrap();

        // Scan body for known msg_ids
        let body = &pt[1..]; // skip flags byte, body before footers
        // setupWorldParameters = 0xFA should be present
        assert!(body.contains(&0xFA), "should contain setupWorldParameters (0xFA)");
        // onPlayerDataLoaded = 0xF3 should be present
        assert!(body.contains(&0xF3), "should contain onPlayerDataLoaded (0xF3)");
        // onAbilityTreeInfo uses extended 0xBD
        assert!(body.contains(&0xBD), "should contain extended encoding marker (0xBD)");
    }

    #[test]
    fn level_exp_boundaries() {
        assert_eq!(level_exp(0), 0);
        assert_eq!(level_exp(1), 100);
        assert_eq!(level_exp(10), 9000);
        assert_eq!(level_exp(20), 400000);
        assert_eq!(level_exp(99), 400000); // clamped
    }

    #[test]
    fn archetype_stats_commando_differs() {
        let soldier = archetype_stats(1);
        let commando = archetype_stats(2);
        assert_eq!(soldier.coordination, 5);
        assert_eq!(commando.coordination, 4);
        assert_eq!(commando.perception, 5);
    }
}
