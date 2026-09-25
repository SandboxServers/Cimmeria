//! Every AI `find_path`, logged once with the NPC's identity:
//! `npc_ai.path event=request`.
//!
//! The navmesh returns a typed [`PathOutcome`] (which Detour stage decided,
//! how far each end snapped); this module turns it into the DEBUG row, the
//! `npc_path_requests_total{world,state,status}` counter, and — for a
//! partial corridor, which callers still walk exactly as before — a
//! `npc_ai.path_fail reason=partial` WARN through the shared emitter.
//!
//! It returns the same `Option<Vec<Vector3>>` the handlers used before, so
//! routing behaviour is unchanged (NA02 is detectors only).

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{PathOutcome, PathStatus};

use super::detectors::MoveSource;
use super::path_failure::{report_path_failure, PathFailReason, PathFailure, PathFallback};
use crate::cell::space_manager::SpaceManager;

/// One AI routing request.
pub(super) struct PathRequest {
    pub npc_id: u32,
    /// `fight` | `follow` | `patrol` | `investigate` | `wander`.
    pub state: &'static str,
    pub from: Vector3,
    pub to: Vector3,
    /// The entity the destination was derived from, when there is one.
    pub target_id: Option<u32>,
    /// The handler's `decision_outcome` if the result is only partial.
    pub partial_outcome: &'static str,
}

/// The routing result a handler acts on.
pub(super) struct RoutedPath {
    /// Exactly what `SpaceManager::find_path` returned before NA02.
    pub waypoints: Option<Vec<Vector3>>,
    /// `None` when the space has no navmesh.
    pub status: Option<PathStatus>,
}

/// Route, log and count one AI path request.
pub(super) fn request_path(
    space_mgr: &mut SpaceManager,
    req: PathRequest,
    now: Instant,
) -> RoutedPath {
    let outcome = space_mgr.find_path_outcome(req.npc_id, &req.from, &req.to);
    let status = outcome.as_ref().map(|o| o.status);
    log_request(space_mgr, &req, outcome.as_ref());

    let waypoints = outcome.and_then(PathOutcome::into_waypoints);
    let usable = waypoints.as_ref().is_some_and(|w| w.len() > 1);
    if usable {
        space_mgr
            .npc_detectors
            .note_move_source(req.npc_id, MoveSource::Path);
    }
    if status == Some(PathStatus::Partial) && usable {
        // Walked exactly as before — to the edge of the start's mesh island
        // — but no longer silently (audit S8).
        report_path_failure(
            space_mgr,
            PathFailure {
                npc_id: req.npc_id,
                state: req.state,
                decision_outcome: req.partial_outcome,
                from: req.from,
                to: req.to,
                reason: PathFailReason::Partial,
                fallback: PathFallback::PartialRoute,
                target_id: req.target_id,
            },
            now,
        );
    }
    RoutedPath { waypoints, status }
}

fn log_request(space_mgr: &SpaceManager, req: &PathRequest, outcome: Option<&PathOutcome>) {
    let status = outcome.map_or("no_mesh", |o| o.status.label());
    let world = super::world_label(space_mgr, req.npc_id);
    cimmeria_observability::counter!(
        "npc_path_requests_total",
        "world" => world.clone(),
        "state" => req.state,
        "status" => status,
    );
    let (tag, template_id, space_id) = space_mgr
        .get_entity(req.npc_id)
        .map(|e| {
            (
                e.tag.clone().unwrap_or_default(),
                e.template_id.unwrap_or(0),
                e.space_id.0,
            )
        })
        .unwrap_or_default();
    let target = req.target_id.and_then(|t| space_mgr.get_entity(t));
    let target_is_gm = target.map(|t| t.is_player && crate::cell::console::is_gm(t.access_level));
    let target_pos = target.map(|t| t.position);
    let wps = outcome.map(|o| o.waypoints.as_slice()).unwrap_or_default();
    let max_leg_dy = wps
        .windows(2)
        .map(|w| (w[1].y - w[0].y).abs())
        .fold(None, |m: Option<f32>, d| Some(m.map_or(d, |m| m.max(d))));
    let end_to_target_dist = wps
        .last()
        .zip(target_pos)
        .map(|(end, t)| end.distance_to(&t));
    tracing::debug!(
        target: "npc_ai.path",
        event = "request",
        npc_id = req.npc_id,
        tag = %tag,
        template_id,
        world = %world,
        space_id,
        state = req.state,
        status,
        from = ?[req.from.x, req.from.y, req.from.z],
        to = ?[req.to.x, req.to.y, req.to.z],
        target_id = req.target_id,
        target_is_gm,
        start_snap_dy = outcome.and_then(|o| o.start_snap_dy(&req.from)),
        end_snap_dist = outcome.and_then(|o| o.end_snap_dist(&req.to)),
        n_waypoints = wps.len(),
        max_leg_dy,
        end_to_target_dist,
        end_to_dest_dist = wps.last().map(|end| end.distance_to(&req.to)),
        "npc_ai.path: route requested ({status})"
    );
}
