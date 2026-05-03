//! World entry builders: create player, enter world, and standalone entity method packets.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use super::{
    append_entity_method, build_world_params_args, client_map_for_world, encrypt_packet,
    method_idx, world_id_for_name, write_wstring, WorldEntryInfo, BASEMSG_CREATE_BASE_PLAYER,
    BASEMSG_CREATE_CELL_PLAYER, BASEMSG_FORCED_POSITION, BASEMSG_SPACE_VIEWPORT_INFO, REPLY_FLAGS,
};

// ── World entry builders ─────────────────────────────────────────────────────

/// Create player step: CREATE_BASE_PLAYER + onClientMapLoad.
///
/// Sends only the base entity creation and terrain load notification.
/// The client will load terrain geometry and respond with `mapLoaded` (cell
/// method index 25, msg_id 0x99). Only after receiving that should the server
/// send the enter-world packet (viewport + cell player + forced position + entity data).
///
/// In C++, BaseApp's `enableEntities()` sends CREATE_BASE_PLAYER then triggers
/// CellApp `sendConnectEntity`. CellApp's `connected()` callback sends
/// `onClientMapLoad`. The client loads terrain, sends `mapLoaded`, and *then*
/// CellApp responds with viewport + cell + position + the full setup sequence.
pub fn build_create_player(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
) -> Vec<u8> {
    let mut body = Vec::with_capacity(256);

    // 1. CREATE_BASE_PLAYER (WORD_LENGTH = 6)
    // class_id: SGWPlayer (0x02) or SGWGmPlayer (0x03) based on access_level.
    body.push(BASEMSG_CREATE_BASE_PLAYER);
    body.extend_from_slice(&6u16.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.push(info.class_id);
    body.push(0x00); // propertyCount = 0

    // 2. onClientMapLoad — tells the client which terrain to load.
    //    Client loads geometry then sends mapLoaded (0x99) when ready.
    {
        let mut args = Vec::new();
        // areaName (WSTRING): world display name
        write_wstring(&mut args, &info.world_name);
        // mapPath (WSTRING): client terrain path (matches client_map column in worlds table)
        let client_map = client_map_for_world(&info.world_name);
        write_wstring(&mut args, client_map);
        // WorldID (INT32)
        args.extend_from_slice(&world_id_for_name(&info.world_name).to_le_bytes());
        // Location (VECTOR3)
        for &c in &info.pos {
            args.extend_from_slice(&c.to_le_bytes());
        }
        // Direction (VECTOR3) — Y/Z swapped per BigWorld convention (heading in Z)
        args.extend_from_slice(&0.0f32.to_le_bytes()); // X
        args.extend_from_slice(&0.0f32.to_le_bytes()); // Y = 0
        args.extend_from_slice(&info.rot[1].to_le_bytes()); // Z = heading
        append_entity_method(
            &mut body,
            method_idx::ON_CLIENT_MAP_LOAD,
            info.player_entity_id,
            &args,
        );
    }

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build raw body bytes for the enter-world step: VIEWPORT + CELL_PLAYER + FORCED_POSITION.
///
/// Returns just the Mercury message bytes (~99 bytes) without packet framing.
/// The C++ server adds these messages to `channel_->bundle()` — the same bundle
/// that contains the mapLoaded entity methods — rather than sending them as a
/// separate packet. This function provides the raw bytes so the caller can
/// prepend them to the mapLoaded body before fragmenting into a single bundle.
pub fn build_enter_world_body(info: &WorldEntryInfo) -> Vec<u8> {
    let mut body = Vec::with_capacity(128);

    // 1. spaceViewportInfo (0x08, CONSTANT_LENGTH = 13)
    body.push(BASEMSG_SPACE_VIEWPORT_INFO);
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.player_entity_id.to_le_bytes());
    body.extend_from_slice(&info.space_id.to_le_bytes());
    body.push(0x00); // viewportID = 0

    // 2. createCellPlayer (0x06, WORD_LENGTH = 32)
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

    // 3. forcedPosition (0x31, CONSTANT_LENGTH = 49)
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

    body
}

/// Enter world step: VIEWPORT + CELL_PLAYER + FORCED_POSITION as a standalone packet.
///
/// Wraps [`build_enter_world_body`] in packet framing and encryption.
/// Kept for tests/reference; the live path now embeds the body into the
/// mapLoaded fragmented bundle instead.
pub fn build_enter_world(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    info: &WorldEntryInfo,
) -> Vec<u8> {
    let body = build_enter_world_body(info);
    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Cell->client entity method: `onPlayerDataLoaded` (no args).
///
/// Flattened ClientMethods index 115 -> msg_id = 0xF3.
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

/// Cell->client entity method: `setupWorldParameters` (22 args: 5xi32 + 17xf32).
///
/// Flattened ClientMethods index 122 -> extended encoding (0xBD + sub_index 61).
/// Sets world physics constants (gravity, movement speeds, etc.).
pub fn build_setup_world_parameters(
    key: &[u8; 32],
    seq: u32,
    acks: &[u32],
    entity_id: u32,
) -> Vec<u8> {
    let mut body = Vec::new();
    let args = build_world_params_args("CombatSim");
    append_entity_method(
        &mut body,
        method_idx::SETUP_WORLD_PARAMETERS,
        entity_id,
        &args,
    );

    let flags = REPLY_FLAGS | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq), acks, None);
    encrypt_packet(&plaintext, key)
}
