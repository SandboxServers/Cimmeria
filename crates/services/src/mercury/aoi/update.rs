//! Position-update packet builders — `UPDATE_AVATAR (0x10)` for AoI ghost
//! relays and `FORCED_POSITION (0x31)` for authoritative own-avatar snaps.

use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_ACKS};

use crate::mercury::{
    encrypt_packet, BASEMSG_FORCED_POSITION, REPLY_FLAGS_RELIABLE, REPLY_FLAGS_UNRELIABLE,
};

use super::{pack_angle, pack_velocity_xyz, BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR};

/// Build and encrypt `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL (0x10)` for
/// relaying position updates to AoI witnesses.
///
/// **Unreliable** (issue #308 audit) — position updates are continuous
/// and self-correcting; the next update supersedes any lost one within
/// a tick or two, so retransmit overhead would buy nothing.
/// `FLAG_RELIABLE` is cleared in the outbound flags.
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

    let flags = REPLY_FLAGS_UNRELIABLE | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}

/// Build `FORCED_POSITION (0x31)` — authoritative position snap for the
/// player's own avatar.
///
/// This is the engine-level message that `BigWorld::ClientHandler` consumes
/// before user code; it bypasses prediction/interpolation and snaps the pawn
/// to `position`. `onPlayerTeleport` (method 116) only flags a streaming-load
/// waiting state — it does not move the avatar. See SGWPlayer.def's comment
/// on `onPlayerTeleport` and docs/protocol/position-updates.md.
///
/// Wire layout matches `build_enter_world_body`:
/// `[entityID:u32][spaceID:u32][vehicleID:u32=0][pos:3×f32][vel:3×f32=0]
///  [rot:3×f32][flags:u8=0x01]`.
pub fn build_forced_position(
    key: &[u8; 32],
    seq_id: u32,
    acks: &[u32],
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
) -> Vec<u8> {
    let mut body = Vec::with_capacity(50);
    body.push(BASEMSG_FORCED_POSITION);
    body.extend_from_slice(&entity_id.to_le_bytes());
    body.extend_from_slice(&space_id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes()); // vehicleID = 0
    for &c in &position {
        body.extend_from_slice(&c.to_le_bytes());
    }
    body.extend_from_slice(&[0u8; 12]); // velocity = 0,0,0
    body.extend_from_slice(&[0u8; 12]); // rotation = 0,0,0 (yaw/pitch/roll)
    body.push(0x01); // flags

    // Reliable — the spawn snap is one-shot state; losing it leaves the
    // pawn at the wrong position until something else corrects (and
    // bug 1 in issue #288 documented the camera-clip artifact when this
    // is mistimed). Channel retransmit covers loss-in-flight.
    let flags = REPLY_FLAGS_RELIABLE | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS };
    let plaintext = build_outgoing(flags, &body, Some(seq_id), acks, None);
    encrypt_packet(&plaintext, key)
}
