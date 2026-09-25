//! Keeping a walking NPC on the floor (NA11, audit M1-M3).
//!
//! Detour's straight path is string-pulled in X and Z only. It adds a corner
//! where the route turns, not where the ground changes slope, so a run
//! across flat floor and then up a ramp comes back as a single segment from
//! the floor to the top of the ramp. The movement tick used to lerp Y along
//! that chord: the NPC climbed an invisible slope over the flat floor, then
//! sank into the stairs (colo: a 45 u leg from Y 68.5 to 73.6, and 0.64 u
//! under the stairs on another). Intermediate corners are also poly-mesh
//! portal vertices, not detail-surface points, so even a snap to a corner
//! could land ~0.15 u off the floor.
//!
//! The fix is a per-step ground query, [`grounded_y`], on the storey nearest
//! the lerped Y. The lerp is still computed, but only as the *reference* that
//! picks the storey, and as the fallback when the mesh has nothing there.
//! The broadcast `vy` comes from the grounded delta
//! ([`grounded_vertical_speed`]), because the client's avatar filter
//! extrapolates along the velocity between updates and a chord `vy` carried
//! the climb on past the last packet.

use std::collections::HashSet;
use std::sync::Mutex;

use super::super::super::space_manager::SpaceManager;
// One enum for both the step row and NA02's `ground_deviation` detector, so
// the two can never disagree about what `y_source` means.
pub(super) use super::super::npc_ai::detectors::movement::YSource;

/// The movement tick's period. `move_speed` is in world units per tick.
pub(super) const MOVEMENT_TICK_SECS: f32 = 0.1;

/// Spaces already reported as meshless, so the DEBUG row fires once per
/// space id per process rather than once per NPC step.
static MESHLESS_SPACES_LOGGED: Mutex<Option<HashSet<u32>>> = Mutex::new(None);

/// The floor under `(x, z)` on the storey nearest `y_ref`, or `y_ref` itself
/// when there is no floor to read.
///
/// `y_ref` is the lerped Y. Each step starts from the previous step's
/// grounded Y and moves at most one step of chord slope from it, so the
/// reference stays within a step's rise of the real floor. That is far
/// inside the ±4 u search window, and the window is under half the smallest
/// storey gap on `castle_cellblock` (~7.9 u), so the query cannot read a
/// floor above or below.
pub(super) fn grounded_y(
    space_mgr: &SpaceManager,
    npc_id: u32,
    x: f32,
    y_ref: f32,
    z: f32,
) -> (f32, YSource) {
    if !space_mgr.space_has_navmesh(npc_id) {
        note_meshless_space(space_mgr, npc_id);
        return (y_ref, YSource::Lerp);
    }
    match space_mgr.get_navmesh_height(npc_id, x, y_ref, z) {
        Some(y) => (y, YSource::Clamp),
        None => (y_ref, YSource::Lerp),
    }
}

fn note_meshless_space(space_mgr: &SpaceManager, npc_id: u32) {
    let Some(space_id) = space_mgr.get_entity_space_id(npc_id) else {
        return;
    };
    let mut guard = MESHLESS_SPACES_LOGGED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if guard.get_or_insert_with(HashSet::new).insert(space_id) {
        tracing::debug!(
            target: "movement.npc",
            event = "ground_clamp_unavailable",
            space_id,
            npc_id,
            "NPC movement: space has no navmesh, walking NPCs keep the lerped Y"
        );
    }
}

/// Vertical speed for the broadcast velocity: the grounded rise over one
/// tick, capped at the NPC's own speed.
///
/// The cap matters on the first step after the clamp takes over from a
/// hovering NPC (one saved before NA11, or one whose last step read no
/// floor): that step drops the NPC by up to a unit in one tick. Sent raw it
/// is ~10 u/s downward, and the client filter would extrapolate the NPC into
/// the floor until the next update. Walkable slopes on the shipped meshes
/// are under 45 degrees, so a real climb never reaches the cap.
pub(super) fn grounded_vertical_speed(cur_y: f32, new_y: f32, speed_per_sec: f32) -> f32 {
    // `f32::clamp` panics on a NaN or inverted bound; a speed comes from a
    // seed column scaled by a GM-settable stat, so do not trust its sign.
    let cap = if speed_per_sec.is_finite() {
        speed_per_sec.abs()
    } else {
        0.0
    };
    ((new_y - cur_y) / MOVEMENT_TICK_SECS).clamp(-cap, cap)
}

#[cfg(test)]
#[path = "npc_ground_tests.rs"]
mod tests;
