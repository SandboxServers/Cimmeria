//! Helpers for building encrypted Mercury packets from the BaseApp server side.
//!
//! These functions produce byte-identical wire output to the C++ BaseApp:
//! - [`build_connect_reply`]           — `BASEMSG_REPLY_MESSAGE` echoing the ticket back (Phase 3).
//! - [`build_time_sync`]               — three time-sync messages in one packet (Phase 3).
//! - [`build_char_list`]               — game-state + character list (Phase 4, dynamic count).
//! - [`build_ongoing_tick_sync`]       — single tick-sync for the 100 ms heartbeat.
//! - [`build_world_entry_phase_a`]     — createBasePlayer + onClientMapLoad (Phase 5b-A).
//! - [`build_world_entry_phase_b`]     — viewport + cell + position (Phase 5b-B, after client loads terrain).
//! - [`build_char_create_failed`]      — `onCharacterCreateFailed` error response.
//! - [`build_resource_fragment`]       — `BASEMSG_RESOURCE_FRAGMENT` for cooked data serving.
//! - [`build_version_info`]            — `onVersionInfo` for client cache version queries.
//!
//! Most functions return a `Vec<u8>` ready to write to the UDP socket.
//! [`build_map_loaded`] returns `(Vec<Vec<u8>>, u32)` — Mercury-fragmented
//! packets that the client reassembles into a single bundle before processing.

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};

// ── Submodules ───────────────────────────────────────────────────────────────

pub mod types;
pub mod protocol;
pub mod aoi;
pub mod world_data;

// ── Re-exports ───────────────────────────────────────────────────────────────
// All items that were previously `pub` in mercury_ext.rs are re-exported here
// so that `use crate::mercury::*` provides the same names.

pub use types::{CharacterInfo, WorldEntryInfo, ArchetypeStats, PlayerLoadData};

pub use protocol::{
    build_connect_reply, build_time_sync, build_char_list, build_on_character_list,
    build_ongoing_tick_sync, build_reset_entities, build_logged_off,
    build_char_create_failed, build_character_visuals, build_resource_fragment,
    build_version_info,
};

pub use aoi::{
    build_create_entity, build_entity_leave, build_avatar_update,
    build_entity_method_packet,
};

pub use world_data::{
    build_world_entry_phase_a, build_world_entry_phase_b,
    build_on_player_data_loaded, build_setup_world_parameters,
    build_map_loaded, archetype_stats, archetype_ability_tree,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// Server→client reply flags (HAS_SEQUENCE | ON_CHANNEL | RELIABLE = 0x58).
pub(crate) const REPLY_FLAGS: u8 = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_RELIABLE;

// ── Message IDs ───────────────────────────────────────────────────────────────

/// `BASEMSG_REPLY_MESSAGE` — reply to a client request (message 0xFF).
pub(crate) const BASEMSG_REPLY_MESSAGE: u8 = 0xFF;
/// `BASEMSG_UPDATE_FREQUENCY_NOTIFICATION` — tick update frequency (0x02).
pub(crate) const BASEMSG_UPDATE_FREQUENCY_NOTIFICATION: u8 = 0x02;
/// `BASEMSG_TICK_SYNC` — current tick counter and rate (0x0D).
pub(crate) const BASEMSG_TICK_SYNC: u8 = 0x0D;
/// `BASEMSG_SET_GAME_TIME` — set client game clock (0x03).
pub(crate) const BASEMSG_SET_GAME_TIME: u8 = 0x03;
/// `BASEMSG_CREATE_BASE_PLAYER` — create a base entity on the client (0x05).
/// Wire format: `[entityID:u32][classID:u8][propertyCount:u8]`.
pub(crate) const BASEMSG_CREATE_BASE_PLAYER: u8 = 0x05;
/// Base entity method: `onCharacterList` (msg_id = 0x80 + methodId 2 = 0x82).
/// Wire format: `[entityID:u32][ARRAY<CharacterInfo>]`.
pub(crate) const BASEMSG_ON_CHARACTER_LIST: u8 = 0x82;
/// Base entity method: `onCharacterCreateFailed` (msg_id = 0x80 + methodId 3 = 0x83).
/// Wire format: `[entityID:u32][errorCode:i32]`.
pub(crate) const BASEMSG_ON_CHARACTER_CREATE_FAILED: u8 = 0x83;
/// Base entity method: `onCharacterVisuals` (msg_id = 0x80 + methodId 4 = 0x84).
/// Wire format: `[entityID:u32][playerId:i32][bodySet:WSTRING][components:ARRAY<WSTRING>][primaryTint:u32][secondaryTint:u32][skinTint:u32]`.
pub(crate) const BASEMSG_ON_CHARACTER_VISUALS: u8 = 0x84;
/// Base entity method: `onVersionInfo` (msg_id = 0x80 + methodId 0 = 0x80).
/// Wire format: `[entityID:u32][categoryId:u32][version:u32][requiredUpdates:u32][invalidateAll:u8][invalidKeys:ARRAY]`.
pub(crate) const BASEMSG_ON_VERSION_INFO: u8 = 0x80;

/// `BASEMSG_SPACE_VIEWPORT_INFO` — CME-custom space/viewport setup (0x08).
/// Wire format: `[entityID:u32][entityID2:u32][spaceID:u32][viewportID:u8]`.
pub(crate) const BASEMSG_SPACE_VIEWPORT_INFO: u8 = 0x08;
/// `BASEMSG_CREATE_CELL_PLAYER` — create cell entity with position (0x06).
/// Wire format: `[spaceID:u32][vehicleID:u32][pos:3×f32][rot:3×f32]`.
pub(crate) const BASEMSG_CREATE_CELL_PLAYER: u8 = 0x06;
/// `BASEMSG_FORCED_POSITION` — authoritative position set (0x31).
/// Wire format: `[entityID:u32][spaceID:u32][vehicleID:u32][pos:3×f32][vel:3×f32][rot:3×f32][flags:u8]`.
pub(crate) const BASEMSG_FORCED_POSITION: u8 = 0x31;
/// `BASEMSG_RESET_ENTITIES` — tear down client entity system (0x04, CONSTANT_LENGTH = 1).
/// Wire format: `[keepBase:u8]` (always 0).
pub(crate) const BASEMSG_RESET_ENTITIES: u8 = 0x04;
/// `BASEMSG_RESOURCE_FRAGMENT` — cooked data fragment (0x36, VARIABLE_LENGTH_MESSAGE).
pub(crate) const BASEMSG_RESOURCE_FRAGMENT: u8 = 0x36;
/// `BASEMSG_LOGGED_OFF` — server tells client the session is terminated (0x37, CONSTANT_LENGTH = 1).
/// Wire format: `[reason:u8]` (0 = normal logoff).
pub(crate) const BASEMSG_LOGGED_OFF: u8 = 0x37;

/// Account entity class ID (EntityTypeID 7 in entity definitions).
pub(crate) const ACCOUNT_CLASS_ID: u8 = 0x07;
/// SGWPlayer entity class ID (EntityTypeID 2 in entity definitions).
pub(crate) const SGWPLAYER_CLASS_ID: u8 = 0x02;
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
    pub const ON_UPDATE_ITEM: u16 = 72;
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
    pub const ON_CLIENT_MAP_LOAD: u16 = 117;
    pub const SETUP_WORLD_PARAMETERS: u16 = 122;
    pub const CLEAR_HINTED_REGIONS: u16 = 124;
    pub const ON_RESET_MAP_INFO: u16 = 126;

    // Extended encoding (>= 128)
    pub const ON_EXTRA_NAME_UPDATE: u16 = 130;
    pub const ON_EXP_UPDATE: u16 = 131;
    pub const ON_MAX_EXP_UPDATE: u16 = 132;
    pub const ON_UPDATE_KNOWN_CRAFTS: u16 = 139;
    pub const ON_ABILITY_TREE_INFO: u16 = 141;
    pub const ON_PLAY_MOVIE: u16 = 155;
}

// ── Entity method encoding ───────────────────────────────────────────────────

/// Append a server→client entity method call to a Mercury message body.
///
/// Handles two encoding schemes per C++ `Bundle::beginEntityMessage()`:
/// - **Direct** (method_index 0–127): `[(index | 0x80): u8][word_len: u16][entity_id: u32][args...]`
/// - **Extended** (method_index >= 128): `[0xBD: u8][word_len: u16][entity_id: u32][(index - 61): u8][args...]`
///
/// The C++ server conservatively uses extended at >= 61, but the client accepts
/// direct encoding through index 127 (verified with setupWorldParameters=122
/// and onPlayerDataLoaded=115). We use the simpler boundary at 128.
pub fn append_entity_method(body: &mut Vec<u8>, method_index: u16, entity_id: u32, args: &[u8]) {
    // BigWorld uses direct encoding for indices 0-60 (msg_id 0x80-0xBC) and
    // extended encoding for indices 61+ (msg_id 0xBD with sub_index byte).
    // 0xBD is the marker — it cannot be used as a direct msg_id.
    if method_index >= 61 {
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

// ── Serialization helpers ────────────────────────────────────────────────────

/// Write a BigWorld `WSTRING` to a buffer.
///
/// Wire format: `[char_count: u32 LE][UTF-16LE data: char_count × 2 bytes]`.
pub(crate) fn write_wstring(buf: &mut Vec<u8>, s: &str) {
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
pub(crate) fn encrypt_packet(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let enc = MercuryEncryption::from_session_key(*key);
    enc.encrypt(plaintext)
        .expect("Mercury packet encryption failed")
}
