//! AoI (Area of Interest) packet builders: entity creation, leave, avatar updates,
//! and entity method packets for ghost entities.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use super::{encrypt_packet, append_entity_method, REPLY_FLAGS};

/// `BASEMSG_CREATE_ENTITY` — create a ghost (non-player) entity on the client (0x09).
/// Sent when an entity enters a player's Area of Interest.
/// Wire: `[msg_id:0x09][wordLen:u16=5][entityId:u32][idAlias:0xFF][classId:u8][0x00][0x00]`
pub(crate) const BASEMSG_CREATE_ENTITY: u8 = 0x09;
/// `BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` — position update
/// for ghost entities (0x10, CONSTANT_LENGTH = 25).
pub(crate) const BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR: u8 = 0x10;
/// `BASEMSG_ENTITY_INVISIBLE` — mark entity invisible before removal (0x0B, CONSTANT_LENGTH = 5).
pub(crate) const BASEMSG_ENTITY_INVISIBLE: u8 = 0x0B;
/// `BASEMSG_LEAVE_AOI` — remove entity from client's AoI (0x0C, WORD_LENGTH).
pub(crate) const BASEMSG_LEAVE_AOI: u8 = 0x0C;

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

/// Pack a float angle (radians) into a single byte (256 steps per circle).
///
/// Matches C++ `(uint8_t)(angle / 0.024543693f)`.
pub(crate) fn pack_angle(radians: f32) -> u8 {
    const SCALE: f32 = 0.024543693;
    (radians / SCALE) as u8
}

/// Pack a velocity Vec3 into 5 bytes using the C++ `packXYZ` format.
///
/// Exact port of `ClientHandler::packXYZ()` from `client_handler.cpp:647-687`.
pub(crate) fn pack_velocity_xyz(v: [f32; 3]) -> [u8; 5] {
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
