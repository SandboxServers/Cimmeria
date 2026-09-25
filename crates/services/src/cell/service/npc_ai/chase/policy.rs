//! Chase policy: where a chasing NPC routes to, when it plans again, and how
//! far off the mesh it may be put back on. Pure functions over positions, so
//! each rule has a unit test that needs no `SpaceManager` (NA15).

use std::time::Duration;

use cimmeria_common::Vector3;

use super::super::leash::policy::horizontal_distance;

/// Two bodies' combined radius, in world units: the closest a chasing NPC
/// stops to its target when its ability sets no larger minimum. Hallway05_Guard2
/// stopped 0.35-0.7 u from its target before NA15 (audit S10), which is
/// inside the player's model.
pub(in crate::cell::service) const COMBINED_RADII: f32 = 1.0;

/// A target whose routed goal moved further than this horizontally from the
/// goal the route was planned for gets a new route.
pub(in crate::cell::service) const REPATH_HORIZONTAL: f32 = 5.0;

/// ...or further than this vertically. A route is planned per storey, so a
/// player walking down a ramp toward the NPC changes the route long before
/// the 3D distance the old test used reached 5 u.
pub(in crate::cell::service) const REPATH_VERTICAL: f32 = 1.5;

/// How long an NPC holds at the end of a route that cannot reach its target,
/// with its target still out of range or out of sight, before it walks home.
pub(in crate::cell::service) const UNREACHABLE_GRACE: Duration = Duration::from_secs(8);

/// Horizontal reach of the snap that puts an NPC the pathfinder cannot start
/// from back on the mesh. Wider than the pathfinder's 0.5 u start box, much
/// narrower than a room.
pub(in crate::cell::service) const OFF_MESH_SNAP_RADIUS: f32 = 2.0;

/// Vertical reach of the same snap. Below half the smallest storey gap on a
/// shipped mesh (about 7.9 u on `castle_cellblock`), so it cannot move the NPC
/// onto another floor.
pub(in crate::cell::service) const OFF_MESH_SNAP_HALF_HEIGHT: f32 = 4.0;

/// Horizontal reach when the *target* is off the mesh (a GM standing on
/// unmeshed props, audit S14): the NPC routes to the nearest on-mesh point
/// within this of the target. Wider than the pathfinder's 3 u destination box,
/// which already failed.
pub(in crate::cell::service) const TARGET_SNAP_RADIUS: f32 = 8.0;

/// Vertical reach of the target snap: the client's jump apex plus margin, the
/// same band `NavMesh::is_point_valid` accepts above a floor.
pub(in crate::cell::service) const TARGET_SNAP_HALF_HEIGHT: f32 = 4.0;

/// An NPC this close (horizontally) to the end of its route counts as having
/// reached it.
pub(in crate::cell::service) const ROUTE_END_REACHED: f32 = 1.0;

/// How far from its target a chasing NPC stops: its ability's minimum range,
/// but never closer than [`COMBINED_RADII`], and never beyond the ability's
/// maximum range when that range allows it (so the NPC can still fire from
/// where it stops).
pub(in crate::cell::service) fn stop_distance(min_range: f32, max_range: f32) -> f32 {
    let stop = min_range.max(COMBINED_RADII);
    if max_range >= COMBINED_RADII {
        stop.min(max_range)
    } else {
        stop
    }
}

/// The point a chase routes to: `target_pos` moved `stop` units toward the
/// NPC in the horizontal plane, at the target's height. With the NPC directly
/// above or below the target there is no bearing, and the offset is taken
/// along +X so the goal is still not inside the target.
pub(in crate::cell::service) fn chase_goal(
    npc_pos: &Vector3,
    target_pos: &Vector3,
    stop: f32,
) -> Vector3 {
    let (dx, dz) = (npc_pos.x - target_pos.x, npc_pos.z - target_pos.z);
    let len = (dx * dx + dz * dz).sqrt();
    let (ux, uz) = if len > f32::EPSILON {
        (dx / len, dz / len)
    } else {
        (1.0, 0.0)
    };
    // Always the full offset: an NPC already closer than `stop` routes back
    // out to it rather than standing inside the target.
    Vector3::new(
        target_pos.x + ux * stop,
        target_pos.y,
        target_pos.z + uz * stop,
    )
}

/// Whether a route planned toward `planned` is stale for a goal now at
/// `goal`: moved beyond [`REPATH_HORIZONTAL`] across the map, or beyond
/// [`REPATH_VERTICAL`] up or down.
pub(in crate::cell::service) fn goal_moved(planned: &Vector3, goal: &Vector3) -> bool {
    horizontal_distance(planned, goal) > REPATH_HORIZONTAL
        || (planned.y - goal.y).abs() > REPATH_VERTICAL
}
