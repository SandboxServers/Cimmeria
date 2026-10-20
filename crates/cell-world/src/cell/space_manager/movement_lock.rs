//! The server-side movement hold for a minigame.
//!
//! MG-06 sets `BSF_PLAYING_MINIGAME` and the counted movement lock, so an
//! honest client stops where it stands. A modified client can still walk, so
//! the cell refuses the move here, the same way it refuses an off-navmesh
//! position. Only the horizontal (x/z) displacement counts: a held player may
//! still fall or turn. A stun's movement lock alone is not gated here.

use cimmeria_commands::permissions::AccessLevel;
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_wire::state_field::BSF_PLAYING_MINIGAME;

use crate::cell::dispatch::access_level_from_u32;

/// Horizontal slack (m) for float noise and the client's last step.
pub(crate) const HELD_SLACK_M: f32 = 0.5;

/// The entity is held in place by the server (a minigame; MG-63b adds the
/// prison boot).
pub(crate) fn movement_held(e: &CellEntity) -> bool {
    e.state_field & BSF_PLAYING_MINIGAME != 0
}

/// `Some(MovementLocked)` when the entity is held and its x/z displacement
/// from `last_valid` exceeds [`HELD_SLACK_M`]. Game masters are not gated.
pub(crate) fn check(
    e: &CellEntity,
    new_pos: [f32; 3],
    last_valid: [f32; 3],
) -> Option<MovementReject> {
    if !movement_held(e)
        || access_level_from_u32(e.access_level).can_execute(AccessLevel::GameMaster)
    {
        return None;
    }
    let dx = new_pos[0] - last_valid[0];
    let dz = new_pos[2] - last_valid[2];
    ((dx * dx + dz * dz).sqrt() > HELD_SLACK_M).then_some(MovementReject::MovementLocked)
}
