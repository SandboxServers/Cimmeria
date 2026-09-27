//! The chase leg to a cover slot (NA22).
//!
//! A slot is a place to stand, not a target to close on, so none of the
//! target rules in [`super`] apply: no stop-distance offset (the NPC stands
//! on the slot), no hold-then-walk-home at the end of a route that cannot
//! reach it (the cover step gives the slot up instead and defers the next
//! seek), no target snap. A route that still ends at the slot is kept; a
//! partial one is walked like any other route and, if it does not end at
//! the slot, the arrival test in `fight_cover` never passes and the next
//! tick re-plans.

use std::time::Instant;

use cimmeria_common::Vector3;

use super::policy::ROUTE_END_REACHED;
use crate::cell::cover::horizontal;
use crate::cell::space_manager::SpaceManager;

/// Install (or keep) a route to `slot`. Returns `false` when the navmesh
/// has no route there. In a space with no navmesh at all the slot becomes a
/// direct waypoint: it is at most `MAX_COVER_DISTANCE` away and there is no
/// containment to break.
pub(in crate::cell::service::npc_ai) fn walk_to_cover_slot(
    space_mgr: &mut SpaceManager,
    npc_id: u32,
    target_id: u32,
    npc_pos: Vector3,
    slot: Vector3,
    now: Instant,
) -> bool {
    let on_route = space_mgr.get_entity(npc_id).is_some_and(|e| {
        e.nav_path
            .back()
            .is_some_and(|end| horizontal(end, &slot) <= ROUTE_END_REACHED)
    });
    if on_route {
        return true;
    }
    let routed = super::super::path_request::request_path(
        space_mgr,
        super::super::path_request::PathRequest {
            npc_id,
            state: "fight",
            from: npc_pos,
            to: slot,
            target_id: Some(target_id),
            partial_outcome: "cover_partial",
        },
        now,
    );
    let waypoints: Vec<Vector3> = match (routed.waypoints, routed.status) {
        (Some(path), _) if path.len() > 1 => path.into_iter().skip(1).collect(),
        // Start and slot share a polygon: the one point is the slot.
        (Some(path), _) if !path.is_empty() => path,
        (None, None) => vec![slot],
        _ => return false,
    };
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        // A leg to a cover slot is not a chase route: drop NA15's record so
        // a later target chase plans afresh instead of treating this route
        // as its own.
        npc.leash.chase_route = None;
        super::super::replace_nav_path_on(npc, waypoints);
    }
    true
}
