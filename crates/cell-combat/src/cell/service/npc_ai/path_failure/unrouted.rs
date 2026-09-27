//! What a non-combat mover does when the pathfinder gives it no usable
//! route (NA41, handoff §9 and §22).
//!
//! `follow`, `patrol`, `wander` and `investigate` used to push the raw
//! destination as a single waypoint on every routing failure. The movement
//! tick grounds Y but does not keep X and Z on the mesh, so the NPC walked
//! straight through walls toward it. Every world has a `.nav` since NA26, so
//! that fired on meshed worlds, not only on the meshless ones it was written
//! for. Chase stopped doing it in NA15.
//!
//! The rule, in order:
//!
//! 1. **No navmesh in the space:** walk straight at the destination. There is
//!    nothing to route on, and standing still forever would be worse. This is
//!    the same policy as `chase::cover_slot`.
//! 2. **A navmesh:** slide from the NPC toward the destination with Detour's
//!    `moveAlongSurface`, which stops at a wall or the edge of the NPC's own
//!    mesh island. If the slide gains at least [`MIN_CLAMP_PROGRESS`]
//!    horizontally, route to where it stopped and walk that route. Routing to
//!    the slide's end, rather than walking straight at it, keeps the walk on
//!    the mesh when the slide went round a corner.
//! 3. **Otherwise** stop with zero velocity and hold. The caller keeps its
//!    state (a follower keeps its target), and the next AI tick tries again.
//!    A patrol skips to its next waypoint and an investigation settles for
//!    where the NPC stands, so neither retries one destination forever.

use cimmeria_common::Vector3;

use super::PathFallback;
use crate::cell::service::npc_ai::detectors::MoveSource;
use crate::cell::service::npc_ai::leash::policy::horizontal_distance;
use crate::cell::space_manager::SpaceManager;

/// The least horizontal distance, in world units, a surface-clamped move
/// must gain to be walked. A shorter one is an NPC already pressed against
/// the wall or the island edge: it holds instead of twitching in place.
pub(in crate::cell::service) const MIN_CLAMP_PROGRESS: f32 = 0.5;

/// The move a mover makes when its route request gave nothing usable.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::cell::service) enum UnroutedMove {
    /// No navmesh: the raw destination, walked in a straight line.
    Direct(Vector3),
    /// The route to where `moveAlongSurface` stopped, start point dropped.
    Clamped(Vec<Vector3>),
    /// Nothing worth walking: stop and hold.
    Hold,
}

impl UnroutedMove {
    /// Decide what an NPC at `from` does about an unrouted `dest`. See the
    /// module docs for the rule.
    pub(in crate::cell::service) fn plan(
        space_mgr: &SpaceManager,
        npc_id: u32,
        from: Vector3,
        dest: Vector3,
    ) -> Self {
        if !space_mgr.space_has_navmesh(npc_id) {
            return Self::Direct(dest);
        }
        let Some(clamp) = space_mgr.move_along_navmesh(npc_id, &from, &dest) else {
            return Self::Hold;
        };
        if horizontal_distance(&from, &clamp) < MIN_CLAMP_PROGRESS {
            return Self::Hold;
        }
        match space_mgr.find_path(npc_id, &from, &clamp) {
            Some(route) if route.len() > 1 => Self::Clamped(route.into_iter().skip(1).collect()),
            _ => Self::Hold,
        }
    }

    /// The `fallback` label this move reports on `npc_ai.path_fail`.
    pub(in crate::cell::service) fn fallback(&self) -> PathFallback {
        match self {
            Self::Direct(_) => PathFallback::DirectWaypoint,
            Self::Clamped(_) => PathFallback::SurfaceClamped,
            Self::Hold => PathFallback::Held,
        }
    }

    /// Whether the move is a hold. A mover with a fixed destination (a
    /// patrol waypoint, an investigate POI) must then give that destination
    /// up, or it retries it and holds forever.
    pub(in crate::cell::service) fn is_hold(&self) -> bool {
        matches!(self, Self::Hold)
    }

    /// Install the move on the NPC: the waypoints, or a stop with zero
    /// velocity. Call it after [`super::report_path_failure`], which reads
    /// the fallback.
    pub(in crate::cell::service) fn apply(self, space_mgr: &mut SpaceManager, npc_id: u32) {
        use crate::cell::service::npc_ai::{replace_nav_path_on, stop_npc_movement, StopReason};
        match self {
            Self::Direct(dest) => {
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    replace_nav_path_on(npc, [dest]);
                }
            }
            Self::Clamped(route) => {
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    replace_nav_path_on(npc, route);
                }
                space_mgr
                    .npc_detectors
                    .note_move_source(npc_id, MoveSource::Path);
            }
            Self::Hold => stop_npc_movement(space_mgr, npc_id, StopReason::HoldNoRoute),
        }
    }
}
