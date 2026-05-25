//! AoI (Area of Interest) packet builders: entity creation, leave, avatar
//! updates, and entity method packets for ghost entities.
//!
//! Split by builder family — each file owns one wire-layout group:
//!
//! - [`create`] — `build_create_entity_base` + `build_create_entity_cascade`
//!   (CREATE_ENTITY 0x09 + UPDATE_AVATAR 0x10 then the property cascade)
//! - [`leave`]  — `build_entity_invisible` + `build_entity_leave`
//!   (ENTITY_INVISIBLE 0x0B alone or paired with LEAVE_AOI 0x0C)
//! - [`update`] — `build_avatar_update` + `build_forced_position`
//!   (UPDATE_AVATAR 0x10 relay + FORCED_POSITION 0x31 snap)
//! - [`method`] — `build_entity_method_packet` (single entity-method call)
//!
//! Wire-byte constants and the angle/velocity packers stay here so each
//! builder file can pull them from `super::` without an extra hop.

mod create;
mod leave;
mod method;
mod update;

#[cfg(test)]
mod tests;

pub use create::{build_create_entity_base, build_create_entity_cascade};
pub(crate) use create::{compose_create_entity_base_body, compose_create_entity_cascade_body};
pub use leave::{build_entity_invisible, build_entity_leave};
pub use method::{build_entity_method_packet, build_player_entity_method_packet};
pub(crate) use update::compose_forced_position_body;
pub use update::{build_avatar_update, build_forced_position};

/// `BASEMSG_CREATE_ENTITY` — create a ghost (non-player) entity on the client (0x09).
/// Sent when an entity enters a player's Area of Interest.
/// Wire: `[msg_id:0x09][wordLen:u16=8][entityId:u32][idAlias:0xFF][classId:u8][0x00][0x00]`
pub(crate) const BASEMSG_CREATE_ENTITY: u8 = 0x09;
/// `BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YAW_PITCH_ROLL` — position update
/// for ghost entities (0x10, CONSTANT_LENGTH = 25).
pub(crate) const BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR: u8 = 0x10;
/// `BASEMSG_ENTITY_INVISIBLE` — mark entity invisible before removal (0x0B, CONSTANT_LENGTH = 5).
pub(crate) const BASEMSG_ENTITY_INVISIBLE: u8 = 0x0B;
/// `BASEMSG_LEAVE_AOI` — remove entity from client's AoI (0x0C, WORD_LENGTH).
pub(crate) const BASEMSG_LEAVE_AOI: u8 = 0x0C;

/// Pack a float angle (radians) into a single byte (256 steps per circle).
///
/// Matches C++ `(uint8_t)(angle / 0.024543693f)`.
pub(super) fn pack_angle(radians: f32) -> u8 {
    const SCALE: f32 = 0.024543693;
    (radians / SCALE) as u8
}

/// Pack a velocity Vec3 into 5 bytes using the C++ `packXYZ` format.
///
/// Exact port of `ClientHandler::packXYZ()` from `client_handler.cpp:647-687`.
pub(super) fn pack_velocity_xyz(v: [f32; 3]) -> [u8; 5] {
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
