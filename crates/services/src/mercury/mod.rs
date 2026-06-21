//! Helpers for building encrypted Mercury packets from the BaseApp server side.
//!
//! These functions produce byte-identical wire output to the C++ BaseApp:
//! - [`build_connect_reply`]           — `BASEMSG_REPLY_MESSAGE` echoing the ticket back (Phase 3).
//! - [`build_time_sync`]               — three time-sync messages in one packet (Phase 3).
//! - [`build_char_list`]               — game-state + character list (Phase 4, dynamic count).
//! - [`build_ongoing_tick_sync`]       — single tick-sync for the 100 ms heartbeat.
//! - [`build_create_player`]           — createBasePlayer + onClientMapLoad (player creation + map load).
//! - [`build_enter_world`]             — viewport + cell + position (world entry, after client loads terrain).
//! - [`build_char_create_failed`]      — `onCharacterCreateFailed` error response.
//! - [`build_resource_fragment`]       — `BASEMSG_RESOURCE_FRAGMENT` for cooked data serving.
//! - [`build_version_info`]            — `onVersionInfo` for client cache version queries.
//!
//! Most functions return a `Vec<u8>` ready to write to the UDP socket.
//! [`build_map_loaded`] returns `(Vec<Vec<u8>>, u32)` — Mercury-fragmented
//! packets that the client reassembles into a single bundle before processing.

use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};
use cimmeria_mercury::packet::{FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};

// ── Submodules ───────────────────────────────────────────────────────────────

pub mod aoi;
pub mod protocol;
pub mod types;
pub mod world_data;

// ── Re-exports ───────────────────────────────────────────────────────────────
// All items that were previously `pub` in mercury_ext.rs are re-exported here
// so that `use crate::mercury::*` provides the same names.

pub use types::{ArchetypeStats, CharacterInfo, PlayerLoadData, WorldEntryInfo};

pub use protocol::{
    build_char_create_failed, build_char_list, build_character_visuals, build_connect_reply,
    build_logged_off, build_on_character_list, build_ongoing_tick_sync, build_reset_entities,
    build_resource_fragment, build_time_sync, build_version_info,
};

pub use aoi::{
    build_avatar_update, build_create_entity_base, build_create_entity_cascade,
    build_entity_invisible, build_entity_leave, build_entity_method_packet, build_forced_position,
    build_player_entity_method_packet,
};
pub(crate) use aoi::{
    compose_create_entity_base_body, compose_create_entity_cascade_body,
    compose_forced_position_body,
};

pub use world_data::{
    archetype_ability_tree, archetype_stats, build_create_player, build_enter_world,
    build_enter_world_body, build_map_loaded, build_map_loaded_body, build_on_player_data_loaded,
    build_setup_world_parameters, fragment_count, fragment_map_loaded,
};

// ── Constants ────────────────────────────────────────────────────────────────

/// Server→client reply flags for **reliable** packets (HAS_SEQUENCE |
/// ON_CHANNEL | RELIABLE = 0x58). Use for state-change messages where
/// loss is permanent damage: entity create/destroy, property updates,
/// mission state, inventory, interaction triggers, almost all
/// server-initiated entity method calls. Loss is recovered by the
/// per-session `Channel`'s retransmit driver.
pub(crate) const REPLY_FLAGS_RELIABLE: u8 = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL | FLAG_RELIABLE;

/// Server→client reply flags for **unreliable** packets (HAS_SEQUENCE |
/// ON_CHANNEL = 0x48 — `FLAG_RELIABLE` cleared). Use for self-correcting
/// / ephemeral messages where loss recovers naturally on the next emit:
/// avatar position updates (`UPDATE_AVATAR` family — the next position
/// frame supersedes any lost one), tick sync (continuous 100ms
/// heartbeat). Per-packet flag, not per-channel; this lets a single
/// channel carry both reliability classes.
pub(crate) const REPLY_FLAGS_UNRELIABLE: u8 = FLAG_HAS_SEQUENCE | FLAG_ON_CHANNEL;

/// **Deprecated alias** for `REPLY_FLAGS_RELIABLE` — keeps existing call
/// sites compiling while the per-site reliability audit migrates them
/// to the explicit `_RELIABLE` / `_UNRELIABLE` constants. New code MUST
/// pick one of the explicit variants.
pub(crate) const REPLY_FLAGS: u8 = REPLY_FLAGS_RELIABLE;

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

/// Account entity class ID (EntityTypeID 7 — client skips ServerOnly entities).
/// entities.xml has SGWBlackMarket(ServerOnly) at index 7, but client numbering
/// excludes it: 0=SGWSpawnableEntity..6=SGWDuelMarker, 7=Account.
/// Confirmed by C++ server pcap: `Base Player Create: type=7`.
pub(crate) const ACCOUNT_CLASS_ID: u8 = 0x07;
/// SGWPlayer entity class ID (EntityTypeID 2 in entity definitions).
pub(crate) const SGWPLAYER_CLASS_ID: u8 = 0x02;
/// SGWGmPlayer entity class ID (EntityTypeID 3 in entity definitions).
///
/// `SGWGmPlayer.def` declares `<Parent>SGWPlayer</Parent>` with an empty
/// `<Implements>`, so its own methods APPEND at the end of the flattened
/// tables (cell 109+, client 157+) and the inherited 0-108 / 0-156 indices
/// do NOT renumber. The wire `idbase` also stays 61 (the exposed-method-count
/// staircase doesn't step between 157 and 163). This is the single byte the
/// client reads from CREATE_BASE_PLAYER to select which entity method table
/// it binds to the player id — 0x02 → SGWPlayer, 0x03 → SGWGmPlayer. Only
/// flipped for access_level > 0 accounts so the native gm* cell surface
/// (109+) becomes reachable. See
/// `docs/architecture/gm-cell-method-gating.md` and
/// `docs/protocol/cell-method-dispatch-table.md` for the full derivation.
pub(crate) const SGWGMPLAYER_CLASS_ID: u8 = 0x03;
/// Default space ID for CombatSim (matches reference server pcap: 0x10010 = 65552).
pub const DEFAULT_SPACE_ID: u32 = 65552;

/// 16 ARGB skin tint values indexed by SkinTintColorID (0-15).
/// Source: `python/common/Constants.py:4-9` — `SKIN_TINTS` array.
pub const SKIN_TINTS: [u32; 16] = [
    0x2F1308FF, 0x180A08FF, 0x15100DFF, 0x9C4F22FF, 0x370405FF, 0x2F1219FF, 0x6C1F0DFF, 0x4F1A09FF,
    0xB45B32FF, 0x632319FF, 0x3A2417FF, 0xF8B487FF, 0xD57D51FF, 0xC36141FF, 0xDF8250FF, 0x8D3F24FF,
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

/// Flattened ClientMethod indices.
///
/// Both SGWPlayer and SGWMob share indices 0–26 since they have the same
/// parent chain through SGWBeing with identical interface ordering.
pub mod method_idx {
    // SGWSpawnableEntity own (0–11)
    pub const ON_SEQUENCE: u16 = 1;
    pub const INTERACTION_TYPE: u16 = 3;
    pub const ON_ENTITY_FLAGS: u16 = 4;
    pub const ON_ENTITY_PROPERTY: u16 = 7;
    pub const ON_VISIBLE: u16 = 8;
    pub const ON_KISMET_EVENT_SET_UPDATE: u16 = 9;
    pub const ON_ENTITY_TINT: u16 = 10;
    pub const ON_BEING_NAME_ID_UPDATE: u16 = 11;

    // SGWBeing interface (12–19)
    pub const ON_EFFECT_RESULTS: u16 = 14;
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

    // OrganizationMember interface (34–51)
    // Source: entities/defs/interfaces/OrganizationMember.def + organization-wire-formats.md
    pub const ON_ORGANIZATION_INVITE: u16 = 34;
    pub const ON_ORGANIZATION_JOINED: u16 = 35;
    pub const ON_ORGANIZATION_LEFT: u16 = 36;
    pub const ON_MEMBER_JOINED_ORGANIZATION: u16 = 37;
    pub const ON_ORGANIZATION_ROSTER_INFO: u16 = 38;
    pub const ON_MEMBER_LEFT_ORGANIZATION: u16 = 39;
    pub const ON_MEMBER_RANK_CHANGED_ORGANIZATION: u16 = 40;
    pub const ON_STRIKE_TEAM_UPDATE: u16 = 41;
    pub const ON_PVP_ORGANIZATION_LEAVE_REQUEST: u16 = 42;
    pub const ON_ORGANIZATION_NAME_UPDATE: u16 = 43;
    pub const ON_ORGANIZATION_EXPERIENCE_UPDATE: u16 = 44;
    pub const ON_ORGANIZATION_MOTD_UPDATE: u16 = 45;
    pub const ON_ORGANIZATION_NOTE_UPDATE: u16 = 46;
    pub const ON_ORGANIZATION_OFFICER_NOTE_UPDATE: u16 = 47;
    pub const ON_ORGANIZATION_CASH_UPDATE: u16 = 48;
    pub const ON_ORGANIZATION_RANK_UPDATE: u16 = 49;
    pub const ON_ORGANIZATION_RANK_NAME_UPDATE: u16 = 50;
    pub const ON_SQUAD_LOOT_TYPE: u16 = 51;

    // GateTravel interface (65–68)
    pub const SETUP_STARGATE_INFO: u16 = 65;

    // SGWInventoryManager interface (69–75)
    pub const ON_BAG_INFO: u16 = 69;
    pub const ON_ACTIVE_SLOT_UPDATE: u16 = 70;
    pub const ON_REMOVE_ITEM: u16 = 71;
    pub const ON_UPDATE_ITEM: u16 = 72;
    pub const ON_CASH_CHANGED: u16 = 75;

    // SGWMailManager interface (76–79)
    pub const ON_MAIL_HEADER_INFO: u16 = 76;
    pub const ON_MAIL_HEADER_REMOVE: u16 = 77;
    pub const ON_MAIL_READ: u16 = 78;
    pub const SEND_MAIL_RESULT: u16 = 79;

    // SGWVendorStore interface (80–81)
    pub const ON_STORE_OPEN: u16 = 80;
    pub const ON_STORE_UPDATE: u16 = 81;

    // SGWContactListManager interface (85–89)
    pub const ON_CONTACT_LIST_UPDATE: u16 = 85;
    pub const ON_CONTACT_LIST_DELETE: u16 = 86;
    pub const ON_CONTACT_LIST_ADD_MEMBERS: u16 = 87;
    pub const ON_CONTACT_LIST_REMOVE_MEMBERS: u16 = 88;
    pub const ON_CONTACT_LIST_EVENT: u16 = 89;

    // SGWPlayer own methods (base offset 98)
    pub const ON_BEGIN_AID_WAIT: u16 = 98;
    pub const ON_END_AID_WAIT: u16 = 99;
    // onDHDReply = 100
    pub const ON_KNOWN_ABILITIES_UPDATE: u16 = 101;
    pub const ON_TIME_OF_DAY: u16 = 102;
    pub const ON_DIALOG_DISPLAY: u16 = 105;
    pub const ON_TRAINER_OPEN: u16 = 113;
    pub const ON_LOOT_DISPLAY: u16 = 114;
    pub const ON_PLAYER_DATA_LOADED: u16 = 115;
    pub const ON_CLIENT_MAP_LOAD: u16 = 117;
    pub const GIVE_ABILITY: u16 = 118;
    pub const GIVE_XP_FOR_LEVEL: u16 = 119;
    pub const ON_ERROR_CODE: u16 = 121;
    pub const SETUP_WORLD_PARAMETERS: u16 = 122;
    pub const CLEAR_HINTED_REGIONS: u16 = 124;
    pub const ADD_CLIENT_HINTED_GENERIC_REGION: u16 = 125;
    pub const ON_RESET_MAP_INFO: u16 = 126;

    // Extended encoding (>= 128)
    pub const ON_EXTRA_NAME_UPDATE: u16 = 130;
    pub const ON_EXP_UPDATE: u16 = 131;
    pub const ON_MAX_EXP_UPDATE: u16 = 132;
    pub const ON_RING_TRANSPORTER_LIST: u16 = 133;
    pub const ON_UPDATE_DISCIPLINE: u16 = 136;
    pub const ON_UPDATE_KNOWN_CRAFTS: u16 = 139;
    pub const ON_ABILITY_TREE_INFO: u16 = 141;
    pub const ON_TRADE_STATE: u16 = 144;
    pub const ON_TRADE_RESULTS: u16 = 145;
    pub const ON_PLAY_MOVIE: u16 = 155;
}

// ── Entity method encoding ───────────────────────────────────────────────────

/// Append a server→client entity method call to a Mercury message body.
///
/// `idbase` is the per-entity-type sub-slot threshold for the target entity —
/// see [`cimmeria_mercury::channel_bundle::idbase_from_exposed_method_count`].
/// For methods targeting SGWPlayer pass
/// [`cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER`] (`61`); for entities
/// with ≤62 exposed methods pass `62`. The threshold is **not** a global
/// constant — it is computed per entity type per
/// `EntityDescription_AssignClientMethodIds @ ghidra://SGW.exe@0x01590df0`:
/// `idBase = 0x3E - (nExposedCount + 0xC0) / 0xFF`. Spec:
/// [docs/drafts/spec/entity-property-sync.md §1.4][1].
///
/// Wire encodings per C++ `Bundle::beginEntityMessage()`:
/// - **Direct** (`method_index < idbase`): `[(index | 0x80): u8][word_len: u16][entity_id: u32][args...]`
/// - **Extended** (`method_index >= idbase`): `[0xBD: u8][word_len: u16][entity_id: u32][(index - idbase): u8][args...]`
///
/// # Field-width contract
///
/// Mirrors the panics from `ChannelBundle::append_entity_method` so the two
/// encoders are behaviorally aligned — a silent narrowing cast here would
/// emit corrupt wire bytes the client cannot recover from. Panics on inputs
/// the wire format cannot represent:
/// - `method_index - idbase >= 256` (extended sub-index byte overflow — for
///   SGWPlayer's `idbase = 61` that's max representable `61 + 255 = 316`)
/// - `args.len()` such that the per-message length field would exceed
///   `u16::MAX` (~65 KB body)
///
/// [1]: ../../../../docs/drafts/spec/entity-property-sync.md
pub fn append_entity_method(
    body: &mut Vec<u8>,
    method_index: u16,
    idbase: u8,
    entity_id: u32,
    args: &[u8],
) {
    use cimmeria_mercury::channel_bundle::EXTENDED_ENCODING_MARKER;

    let threshold = u16::from(idbase);
    if method_index >= threshold {
        // Extended encoding: marker 0xBD + sub_index
        let sub_index = u8::try_from(method_index - threshold)
            .expect("method_index exceeds Mercury extended-encoding range (idbase + 255 = max)");
        let payload_len = u16::try_from(4 + 1 + args.len())
            .expect("entity-method payload exceeds Mercury u16 length field (~65 KB max)");
        body.push(EXTENDED_ENCODING_MARKER);
        body.extend_from_slice(&payload_len.to_le_bytes());
        body.extend_from_slice(&entity_id.to_le_bytes());
        body.push(sub_index);
    } else {
        // Direct encoding: msg_id = index | 0x80
        let payload_len = u16::try_from(4 + args.len())
            .expect("entity-method payload exceeds Mercury u16 length field (~65 KB max)");
        // Safe: method_index < idbase <= 62 < u8::MAX, so `as u8` cannot
        // truncate. The high bit is then set via `| 0x80` as the direct-
        // encoding marker.
        body.push((method_index as u8) | 0x80);
        body.extend_from_slice(&payload_len.to_le_bytes());
        body.extend_from_slice(&entity_id.to_le_bytes());
    }
    body.extend_from_slice(args);

    // Diagnostic logging for visual methods
    if method_index == method_idx::BEING_APPEARANCE || method_index == method_idx::ON_ENTITY_TINT {
        let method_name = if method_index == method_idx::BEING_APPEARANCE {
            "BeingAppearance"
        } else {
            "onEntityTint"
        };
        tracing::debug!(
            method_index,
            method_name,
            arg_bytes = args.len(),
            args_hex = ?args,
            "Entity method wire data"
        );
    }
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
///
/// `version` is the session's pinned wire version. The whole session — both
/// directions and every handshake builder — must pass the *same* version, so
/// the bytes a client sees are internally consistent (a v2 handshake followed
/// by v1 data, or vice-versa, would fail the peer's decrypt).
pub(crate) fn encrypt_packet(
    plaintext: &[u8],
    key: &[u8; 32],
    version: EncryptionVersion,
) -> Vec<u8> {
    let enc = MercuryEncryption::from_session_key_versioned(*key, version);
    enc.encrypt(plaintext)
        .expect("Mercury packet encryption failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Direct encoding for method indices < 61. Wire layout:
    /// `[(index | 0x80): u8] [word_len: u16 LE] [entity_id: u32 LE] [args]`
    /// where `word_len = 4 (entity_id) + args.len()`.
    #[test]
    fn append_entity_method_direct_encoding_for_low_index() {
        let mut body = Vec::new();
        let args = [0xAA, 0xBB, 0xCC];
        // Pick index 12 (onTimerUpdate). 12 | 0x80 = 0x8C.
        append_entity_method(
            &mut body,
            12,
            cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER,
            0xDEAD_BEEF,
            &args,
        );

        assert_eq!(body[0], 0x8C, "msg_id must be (index | 0x80)");
        let word_len = u16::from_le_bytes([body[1], body[2]]);
        assert_eq!(word_len, 4 + args.len() as u16);
        let entity_id = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
        assert_eq!(entity_id, 0xDEAD_BEEF);
        assert_eq!(&body[7..], &args);
    }

    /// Pin index 122 (setupWorldParameters) — the source comment names
    /// it as one of the indices verified to work over the wire
    /// (alongside onPlayerDataLoaded=115). A regression that flips
    /// the `>= 61` extended-encoding boundary would silently break
    /// this method's wire shape. Note: the existing protocol-tests
    /// cover the boundary case (index 61) and index 128; this test
    /// fills the gap at the named-callsite middle of the range.
    #[test]
    fn append_entity_method_extended_encoding_at_index_122() {
        let mut body = Vec::new();
        append_entity_method(
            &mut body,
            122,
            cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER,
            1,
            &[],
        );

        assert_eq!(body[0], 0xBD);
        let word_len = u16::from_le_bytes([body[1], body[2]]);
        assert_eq!(word_len, 4 + 1, "no args, only entity_id + sub_index");
        assert_eq!(body[7], 122 - 61, "sub_index = index - 61");
    }

    /// Non-BMP code point (emoji) round-trips through a UTF-16 surrogate
    /// pair — char_count is 2, not 1. Pin so a refactor that uses
    /// `s.chars().count()` for the count (instead of the encode_utf16
    /// length) would silently corrupt the wire payload. The basic
    /// empty + ASCII round-trip cases are already covered by
    /// `mercury/protocol/tests.rs::read_wstring_empty` and
    /// `read_wstring_roundtrip`; this test fills the non-BMP gap.
    #[test]
    fn write_wstring_non_bmp_uses_surrogate_pair() {
        let mut buf = Vec::new();
        // "🌟" (U+1F31F) is outside the BMP and encodes to 2 UTF-16 code
        // units (D83C DF1F).
        write_wstring(&mut buf, "🌟");
        let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(count, 2, "non-BMP char encodes to 2 UTF-16 code units");
        assert_eq!(buf.len(), 4 + 4);
        let (s, consumed) = read_wstring(&buf, 0).unwrap();
        assert_eq!(s, "🌟");
        assert_eq!(consumed, buf.len());
    }

    /// `read_wstring` from an offset > 0 must read the count + chars
    /// starting at that offset, returning bytes_consumed relative to
    /// the wstring's start (NOT the buffer's start). Pin so callers
    /// can chain reads of multiple wstrings in one buffer.
    #[test]
    fn read_wstring_with_offset() {
        let mut buf = vec![0xAAu8; 5]; // junk preamble
        write_wstring(&mut buf, "hi");
        // wstring starts at offset 5; total byte len = 4 + 4 = 8.
        let (s, consumed) = read_wstring(&buf, 5).unwrap();
        assert_eq!(s, "hi");
        assert_eq!(
            consumed, 8,
            "consumed must be the wstring's byte length, not absolute end offset"
        );
    }

    /// Truncated wstring returns Err — both for "not enough bytes for
    /// the length field" and "length field claims more bytes than the
    /// buffer holds". Pin both branches; an earlier version that
    /// panicked on the second case would crash on adversarial input.
    #[test]
    fn read_wstring_rejects_truncated_inputs() {
        // Case 1: only 3 bytes available, can't even read the count.
        let buf = [0u8, 0, 0];
        assert!(read_wstring(&buf, 0).is_err());

        // Case 2: count claims 10 chars (20 bytes) but only 4 bytes of
        // payload available.
        let mut buf = vec![10u8, 0, 0, 0]; // count = 10
        buf.extend_from_slice(&[0u8; 4]); // only 2 chars worth of payload
        assert!(read_wstring(&buf, 0).is_err());
    }

    /// `encrypt_packet` with the v1 version produces byte-identical output to
    /// `MercuryEncryption::from_session_key(key).encrypt(plaintext).unwrap()`.
    /// Pin so a refactor that swaps the wrapper for a different code
    /// path can't silently change the wire bytes.
    #[test]
    fn encrypt_packet_matches_direct_encryption_call() {
        let key = [0x42u8; 32];
        let plaintext = b"hello mercury";
        let via_helper = encrypt_packet(plaintext, &key, EncryptionVersion::V1);
        let via_direct = MercuryEncryption::from_session_key(key)
            .encrypt(plaintext)
            .unwrap();
        assert_eq!(via_helper, via_direct);
    }

    /// `encrypt_packet` with the v2 version produces a v2 frame (leading
    /// `0x02`) that is NOT byte-identical to the v1 frame for the same input —
    /// proves the version argument actually switches the wire shape rather
    /// than being ignored.
    #[test]
    fn encrypt_packet_v2_differs_and_is_versioned() {
        let key = [0x42u8; 32];
        let plaintext = b"hello mercury";
        let v1 = encrypt_packet(plaintext, &key, EncryptionVersion::V1);
        let v2 = encrypt_packet(plaintext, &key, EncryptionVersion::V2);
        assert_ne!(v1, v2, "v2 frame must differ from v1 frame");
        assert_eq!(v2[0], 0x02, "v2 frame must begin with the version byte");
    }
}
