//! The Fighting handler's chase: route toward a target the NPC cannot hit
//! from where it stands, and decide when that is hopeless (NA15, audit S8-S10
//! and S14).
//!
//! - [`policy`] holds the numbers and the pure rules: the stop distance, the
//!   offset goal, the repath threshold and the snap reaches.
//! - This file is the per-tick step. It keeps walking a route that is still
//!   good for the target, plans a new one when the target moved, and:
//!   - walks a **partial** route (target on another mesh island) to its end
//!     and then holds with zero velocity, instead of repathing every tick into
//!     a near-zero route at the island edge; after [`policy::UNREACHABLE_GRACE`]
//!     the NPC walks home;
//!   - snaps an NPC the pathfinder cannot **start** from onto the nearest
//!     polygon and retries once; with no polygon near, it walks home (the leash
//!     tick then snaps it to spawn);
//!   - routes toward the nearest on-mesh point when the **target** is off the
//!     mesh, so a GM on unmeshed props does not freeze every chaser;
//!   - clears the stale route when a repath comes back **degenerate**;
//!   - stops [`policy::stop_distance`] short of the target, never inside it.
//! - [`cover_slot`] is the leg to a cover slot (NA22). None of the target
//!   rules above apply to it: the NPC stands on the slot, and a slot it
//!   cannot reach is given up by the cover step rather than held.

mod cover_slot;
pub(in crate::cell::service) mod policy;
pub(super) use cover_slot::walk_to_cover_slot;
#[cfg(test)]
mod tests;

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::ChaseRoute;
use cimmeria_entity::navigation::PathStatus;
use tokio::sync::mpsc;

use super::detectors::MoveSource;
use super::leash::policy::horizontal_distance;
use super::path_failure::{report_path_failure, PathFailReason, PathFailure, PathFallback};
use super::path_request::{request_path, PathRequest, RoutedPath};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// What the fight handler knows when it decides to chase.
pub(super) struct ChaseStep {
    pub npc_id: u32,
    pub target_id: u32,
    pub npc_pos: Vector3,
    pub target_pos: Vector3,
    /// The target, or another routed-as-given goal. Since NA22 the fight
    /// handler always passes the target here: the walk to a cover slot is
    /// [`walk_to_cover_slot`].
    pub nav_target_pos: Vector3,
    /// See [`policy::stop_distance`].
    pub stop_distance: f32,
    pub in_range: bool,
    pub has_los: bool,
    pub dist_to_target: f32,
}

/// One chase tick for a mobile NPC that is out of range or out of sight.
pub(super) async fn chase(
    step: ChaseStep,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = Instant::now();
    // A cover slot is where the NPC should stand; only a route at the target
    // itself is pulled up short of it.
    let goal = if step.nav_target_pos == step.target_pos {
        policy::chase_goal(&step.npc_pos, &step.target_pos, step.stop_distance)
    } else {
        step.nav_target_pos
    };
    let Some((path_end, record)) = space_mgr
        .get_entity(step.npc_id)
        .map(|e| (e.nav_path.back().copied(), e.leash.chase_route))
    else {
        return;
    };
    let current = record.filter(|r| !policy::goal_moved(&r.goal, &goal));
    match (path_end, current) {
        // Still walking the route planned for this goal (partial or not).
        (Some(end), Some(r)) if r.end == end => {
            hold_no_repath(&step);
            return;
        }
        // At the end of a route that cannot reach the goal, and the goal has
        // not moved: another plan would find the same dead end.
        (None, Some(r))
            if !r.reaches_goal
                && horizontal_distance(&step.npc_pos, &r.end) <= policy::ROUTE_END_REACHED =>
        {
            hold_unreachable(&step, now, tx, space_mgr).await;
            return;
        }
        // A route installed by something else (content, a cover move) that
        // still ends near the goal.
        (Some(end), None) if record.is_none() && !policy::goal_moved(&end, &goal) => {
            hold_no_repath(&step);
            return;
        }
        _ => {}
    }
    plan(&step, goal, now, tx, space_mgr).await;
}

fn hold_no_repath(step: &ChaseStep) {
    super::note_outcome("hold_no_repath");
    tracing::debug!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = "hold_no_repath",
        npc_id = step.npc_id,
        target_id = step.target_id,
        in_range = step.in_range,
        has_los = step.has_los,
        dist_to_target = step.dist_to_target,
        "NPC AI: out of range/LoS but the route is still good for where the target is -- no new order this tick"
    );
}

/// Stand still at the end of a route that cannot reach the target, facing
/// it, and give up once the grace period has passed.
async fn hold_unreachable(
    step: &ChaseStep,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    super::stop_npc_movement(space_mgr, step.npc_id, super::StopReason::HoldUnreachable);
    super::fight::face_target(space_mgr, step.npc_id, step.npc_pos, step.target_pos);
    let Some(since) = space_mgr
        .get_entity_mut(step.npc_id)
        .map(|npc| *npc.leash.unreachable_since.get_or_insert(now))
    else {
        return;
    };
    let held = now.saturating_duration_since(since);
    if held >= policy::UNREACHABLE_GRACE {
        give_up(step, "held_unreachable", held.as_secs_f32(), tx, space_mgr).await;
        return;
    }
    super::note_outcome("hold_unreachable");
    tracing::debug!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = "hold_unreachable",
        npc_id = step.npc_id,
        target_id = step.target_id,
        in_range = step.in_range,
        has_los = step.has_los,
        dist_to_target = step.dist_to_target,
        held_secs = held.as_secs_f32(),
        "NPC AI: at the end of a route that cannot reach the target -- holding"
    );
}

/// Walk home: the target cannot be reached. The leash tick walks the NPC home,
/// or snaps it there when it cannot route from where it stands.
async fn give_up(
    step: &ChaseStep,
    cause: &'static str,
    held_secs: f32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    super::note_outcome("leashed");
    tracing::info!(
        target: "npc_ai",
        event = "decision",
        decision_outcome = "leashed",
        npc_id = step.npc_id,
        target_id = step.target_id,
        trigger = "unreachable",
        cause,
        held_secs,
        dist_to_target = step.dist_to_target,
        "NPC AI: target unreachable, walking home"
    );
    super::leash::begin_leash(
        step.npc_id,
        super::AiTransitionReason::Unreachable,
        "unreachable",
        Some((step.target_id, step.target_pos)),
        tx,
        space_mgr,
    )
    .await;
}

fn route(
    space_mgr: &mut SpaceManager,
    step: &ChaseStep,
    from: Vector3,
    to: Vector3,
    now: Instant,
) -> RoutedPath {
    request_path(
        space_mgr,
        PathRequest {
            npc_id: step.npc_id,
            state: "fight",
            from,
            to,
            target_id: Some(step.target_id),
            partial_outcome: "chase_partial",
        },
        now,
    )
}

fn report(
    space_mgr: &mut SpaceManager,
    step: &ChaseStep,
    (from, to): (Vector3, Vector3),
    (decision_outcome, reason, fallback): (&'static str, PathFailReason, PathFallback),
    now: Instant,
) {
    report_path_failure(
        space_mgr,
        PathFailure {
            npc_id: step.npc_id,
            state: "fight",
            decision_outcome,
            from,
            to,
            reason,
            fallback,
            target_id: Some(step.target_id),
        },
        now,
    );
}

/// Plan a route to `goal` and act on what came back.
async fn plan(
    step: &ChaseStep,
    goal: Vector3,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut from = step.npc_pos;
    let mut routed = route(space_mgr, step, from, goal, now);

    // Start off the mesh (audit S9): put the NPC back on the nearest polygon
    // through the grid-updating writer, then try once more from there.
    if routed.status == Some(PathStatus::NoStartPoly) {
        let Some(onto) = space_mgr.nearest_navmesh_point_within(
            step.npc_id,
            &from,
            policy::OFF_MESH_SNAP_RADIUS,
            policy::OFF_MESH_SNAP_HALF_HEIGHT,
        ) else {
            report(
                space_mgr,
                step,
                (from, goal),
                (
                    "no_path",
                    PathFailReason::NoStartPoly,
                    PathFallback::PathUnchanged,
                ),
                now,
            );
            give_up(step, "off_mesh", 0.0, tx, space_mgr).await;
            return;
        };
        report(
            space_mgr,
            step,
            (from, goal),
            (
                "off_mesh_snap",
                PathFailReason::NoStartPoly,
                PathFallback::SnappedToMesh,
            ),
            now,
        );
        tracing::info!(
            target: "npc_ai.path",
            event = "off_mesh_snap",
            npc_id = step.npc_id,
            from = ?[from.x, from.y, from.z],
            to = ?[onto.x, onto.y, onto.z],
            snap_dy = onto.y - from.y,
            snap_horizontal = horizontal_distance(&from, &onto),
            "npc_ai.path: NPC could not start a route where it stood -- snapped onto the nearest polygon"
        );
        super::movement_stop::snap_npc_from(
            space_mgr,
            step.npc_id,
            onto,
            None,
            MoveSource::MeshSnap,
        );
        from = onto;
        routed = route(space_mgr, step, from, goal, now);
    }

    // Target off the mesh (audit S14): route to the nearest on-mesh point to
    // it. The route cannot reach the target itself, so the NPC holds at its
    // end if it still cannot hit from there.
    let mut reaches_goal = true;
    if routed.status == Some(PathStatus::NoEndPoly) {
        if let Some(near) = space_mgr.nearest_navmesh_point_within(
            step.npc_id,
            &goal,
            policy::TARGET_SNAP_RADIUS,
            policy::TARGET_SNAP_HALF_HEIGHT,
        ) {
            report(
                space_mgr,
                step,
                (from, goal),
                (
                    "chase_nearest_on_mesh",
                    PathFailReason::NoEndPoly,
                    PathFallback::NearestOnMesh,
                ),
                now,
            );
            routed = route(space_mgr, step, from, near, now);
            reaches_goal = false;
        }
    }
    reaches_goal &= routed.status != Some(PathStatus::Partial);

    let status = routed.status;
    match routed.waypoints {
        Some(path) if path.len() > 1 => {
            let end = *path.last().expect("len > 1");
            if let Some(npc) = space_mgr.get_entity_mut(step.npc_id) {
                super::replace_nav_path_on(npc, path.into_iter().skip(1));
                npc.leash.chase_route = Some(ChaseRoute {
                    goal,
                    end,
                    reaches_goal,
                });
                if reaches_goal {
                    npc.leash.unreachable_since = None;
                }
            }
            super::note_outcome("chase");
            tracing::debug!(
                target: "npc_ai",
                event = "decision",
                decision_outcome = "chase",
                npc_id = step.npc_id,
                target_id = step.target_id,
                in_range = step.in_range,
                has_los = step.has_los,
                dist_to_target = step.dist_to_target,
                reaches_target = reaches_goal,
                stop_distance = step.stop_distance,
                "NPC AI: pathfinding toward target"
            );
        }
        Some(_) => {
            // A one-point route: the NPC is already where the pathfinder can
            // take it. Drop the stale route rather than walking on toward
            // where the target used to be (audit S10), and hold here.
            super::stop_npc_movement(space_mgr, step.npc_id, super::StopReason::RepathDegenerate);
            if let Some(npc) = space_mgr.get_entity_mut(step.npc_id) {
                npc.leash.chase_route = Some(ChaseRoute {
                    goal,
                    end: from,
                    reaches_goal: false,
                });
            }
            super::note_outcome("repath_degenerate");
            report(
                space_mgr,
                step,
                (from, goal),
                (
                    "repath_degenerate",
                    PathFailReason::DegeneratePath,
                    PathFallback::PathCleared,
                ),
                now,
            );
        }
        None => {
            super::note_outcome("no_path");
            let reason = PathFailReason::for_missing_path(space_mgr, step.npc_id, status);
            // `fight` does not enqueue the raw target as a direct waypoint:
            // the chaser stands still, or keeps walking a stale route.
            report(
                space_mgr,
                step,
                (from, goal),
                ("no_path", reason, PathFallback::PathUnchanged),
                now,
            );
        }
    }
}
