//! The one way to stop an NPC, or to swap the route it is walking.
//!
//! Every 100 ms the AoI tick sends each witness an `EntityMoved` carrying the
//! NPC's stored `position`, `direction` and **`velocity`**, whether or not the
//! NPC moved (`space_manager/aoi.rs`). The client scales that velocity by 100
//! in `EntityManager::onEntityMoveWithError` (`0x00dd1650`) and writes it into
//! the UE3 actor in `GameEntityBase::ApplyTransform` (`0x00e68a30`). Velocity
//! is the only "I am moving" signal the client gets. No server-to-client
//! movement-type message exists; see `broadcast_movement_type`.
//!
//! Before NA10, eleven sites cleared `nav_path` in the middle of a leg and left
//! `velocity` at the chase speed (about 6 u/s). The movement tick skips NPCs
//! that have no path, so nothing zeroed it again. The client was then told
//! "moving at 6 u/s" about an NPC standing still, and it ran in place (audit
//! S1). The leash was worse: it wrote `position` directly, which skipped the
//! spatial-grid update, and it left the chase path in place, so the NPC walked
//! back out from spawn (S4).
//!
//! The rule is now:
//!
//! - **Stopping** clears the path and zeroes velocity together:
//!   [`stop_movement_on`] when you already hold `&mut CellEntity`, and
//!   [`stop_npc_movement`] otherwise, which also logs the stop.
//! - **Rerouting** goes through [`replace_nav_path_on`]. An empty route counts
//!   as a stop.
//! - **Teleporting** an NPC (the leash snap) goes through
//!   [`snap_npc_to`], which uses the grid-updating position writer.
//! - Every real AI state change stops the NPC too. `set_ai_state_on` calls
//!   [`stop_movement_on`], because a route planned in one state is never valid
//!   in the next one.
//!
//! The only other path writer is the movement tick itself
//! (`ticks/npc_movement.rs`). It consumes waypoints and zeroes velocity when
//! it reaches the last one. A guard test at the bottom of this file scans
//! `crates/services/src` for any other `nav_path.clear()` or `nav_path =`.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::CellEntity;

use super::detectors::MoveSource;
use crate::cell::space_manager::SpaceManager;

/// Why an NPC was stopped. Enumerated because it is a SigNoz group-by key on
/// the `movement.npc event=stop` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cell) enum StopReason {
    /// In range with line of sight: the NPC stands and shoots.
    AttackInPlace,
    /// The NPC's cover slot was flanked and released.
    CoverReleased,
    /// At the end of a route that cannot reach the target: hold (NA15).
    HoldUnreachable,
    /// A repath came back with no usable leg: the stale route is dropped
    /// rather than walked toward where the target used to be (NA15).
    RepathDegenerate,
}

impl StopReason {
    /// Stable snake_case label. Treat as API.
    pub(in crate::cell) fn label(self) -> &'static str {
        match self {
            Self::AttackInPlace => "attack_in_place",
            Self::CoverReleased => "cover_released",
            Self::HoldUnreachable => "hold_unreachable",
            Self::RepathDegenerate => "repath_degenerate",
        }
    }
}

/// Clear the path and zero velocity. Returns `true` when that changed
/// anything, meaning the NPC had a path or a non-zero velocity.
///
/// Position does not change, so the spatial grid needs no update. The zeroed
/// velocity reaches every witness on the next AoI tick without any extra
/// fan-out.
pub(in crate::cell) fn stop_movement_on(npc: &mut CellEntity) -> bool {
    let was_moving = !npc.nav_path.is_empty() || npc.velocity != [0.0; 3];
    npc.nav_path.clear();
    npc.velocity = [0.0; 3];
    was_moving
}

/// [`stop_movement_on`] by id, with a `movement.npc event=stop` DEBUG row
/// when it actually stopped something. It is silent when the NPC was already
/// still, so an attack-in-place hold does not log on every AI tick.
pub(in crate::cell) fn stop_npc_movement(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    reason: StopReason,
) {
    let Some(npc) = space_mgr.get_entity_mut(npc_id) else {
        return;
    };
    let nav_path_len = npc.nav_path.len();
    let [vx, vy, vz] = npc.velocity;
    if stop_movement_on(npc) {
        tracing::debug!(
            target: "movement.npc",
            event = "stop",
            npc_id,
            reason = reason.label(),
            nav_path_len,
            prior_speed = (vx * vx + vy * vy + vz * vz).sqrt(),
            "NPC stopped: path cleared, velocity zeroed"
        );
    }
}

/// Replace the NPC's route with `waypoints`. Velocity is left for the
/// movement tick to set on its next step toward the new first waypoint,
/// unless the new route is empty. In that case the NPC has stopped, and
/// velocity is zeroed here.
pub(in crate::cell) fn replace_nav_path_on(
    npc: &mut CellEntity,
    waypoints: impl IntoIterator<Item = Vector3>,
) {
    npc.nav_path.clear();
    npc.nav_path.extend(waypoints);
    if npc.nav_path.is_empty() {
        npc.velocity = [0.0; 3];
    }
}

/// Teleport an NPC to `pos` and stop it there. The position goes through
/// `update_position_preserving_facing`, which keeps the AoI spatial grid in
/// sync. `facing`, when given, replaces the yaw. The leash passes the
/// authored `spawn_direction`, as the respawn tick does.
pub(in crate::cell) fn snap_npc_to(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    pos: Vector3,
    facing: Option<Vector3>,
) {
    snap_npc_from(space_mgr, npc_id, pos, facing, MoveSource::Leash);
}

/// [`snap_npc_to`] with the mover named for NA02's
/// `npc_off_mesh.last_move_source`: the leash snaps home, the chase snaps an
/// off-mesh NPC onto the nearest polygon (NA15).
pub(in crate::cell) fn snap_npc_from(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    pos: Vector3,
    facing: Option<Vector3>,
    source: MoveSource,
) {
    space_mgr.update_position_preserving_facing(npc_id, [pos.x, pos.y, pos.z], [0.0; 3]);
    space_mgr.npc_detectors.note_move_source(npc_id, source);
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        stop_movement_on(npc);
        if let Some(dir) = facing {
            npc.direction = dir;
        }
    }
}

#[cfg(test)]
mod tests;
