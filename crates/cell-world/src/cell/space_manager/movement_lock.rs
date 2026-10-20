//! The server-side movement hold for a minigame.
//!
//! MG-06 sets `BSF_PLAYING_MINIGAME` and the counted movement lock, so an
//! honest client stops where it stands. A modified client can still walk, so
//! the cell refuses the move here, the same way it refuses an off-navmesh
//! position. The slack is measured from the hold anchor (where the hold found
//! the player), not from the last accepted packet, so small steps cannot add
//! up. Only the horizontal (x/z) displacement counts: a held player may still
//! fall or turn. A stun's movement lock alone is not gated here.

use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::movement_validation::MovementReject;
use cimmeria_wire::state_field::BSF_PLAYING_MINIGAME;

/// Horizontal slack (m) for float noise and the client's last step.
pub(crate) const HELD_SLACK_M: f32 = 0.5;

/// The entity is held in place by the server: a minigame (bit 4) or a
/// movement-only hold such as the Prison Boot (MG-63b).
pub(crate) fn movement_held(e: &CellEntity) -> bool {
    e.state_field & BSF_PLAYING_MINIGAME != 0 || e.holds_movement_only_entry()
}

/// `Some(MovementLocked)` when the x/z displacement of `new_pos` from the hold
/// `anchor` exceeds [`HELD_SLACK_M`]. The caller decides whether the entity is
/// held and exempts game masters.
pub(crate) fn check(new_pos: [f32; 3], anchor: [f32; 3]) -> Option<MovementReject> {
    let dx = new_pos[0] - anchor[0];
    let dz = new_pos[2] - anchor[2];
    ((dx * dx + dz * dz).sqrt() > HELD_SLACK_M).then_some(MovementReject::MovementLocked)
}
