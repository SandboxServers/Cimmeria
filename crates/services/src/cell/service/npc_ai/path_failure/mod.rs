//! One shape for "the pathfinder didn't give this NPC a route", shared
//! by every AI state that calls `find_path`.
//!
//! # Why this is one function and not five
//!
//! Before this module, `follow` logged its straight-line fallback,
//! `fight` logged two of its three failure shapes at two different
//! levels with a different field set, and `patrol`, `investigate` and
//! `wander` logged **nothing at all** — they silently pushed the raw
//! destination as a single waypoint and let the NPC walk through
//! geometry toward it. The 2026-09-18 Castle playtest saw exactly that:
//! NPCs cutting through walls, with the only evidence being the follow
//! legs, because follow happened to be the one state somebody had
//! instrumented.
//!
//! A single emitter means adding an AI state cannot accidentally add a
//! silent one, and a SigNoz query on `scope_name = "npc_ai.path_fail"`
//! grouped by `state` answers "which behaviour is failing to route, in
//! which world" without knowing which handlers exist.
//!
//! # Why it is throttled
//!
//! These fire per AI tick. An NPC parked against a wall with no route
//! home is the *normal* steady state for a mesh hole, not a transient —
//! and it would otherwise produce a row every tick for as long as the
//! zone is up. The throttle is the same primitive the movement-reject
//! log uses ([`crate::cell::space_manager::MovementTelemetry`]): first
//! occurrence immediately, then at most one per
//! [`PATH_FAIL_LOG_MIN_INTERVAL`], carrying `suppressed = N` for the
//! ticks elided in between. The `npc_path_fail_total` counter is
//! incremented on every occurrence regardless.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::PathStatus;

use crate::cell::space_manager::SpaceManager;

#[cfg(test)]
mod tests;

/// Minimum gap between two emitted `npc_ai.path_fail` rows for the same
/// NPC. Longer than the movement-reject window (1 s) because the AI tick
/// itself is slower (~2 s natural cadence, ~100 ms on the retry sweep)
/// and a stuck NPC is a standing condition rather than an event.
const PATH_FAIL_LOG_MIN_INTERVAL: Duration = Duration::from_secs(5);

/// Why the NPC has no route. Low-cardinality; used as a `reason` log
/// field and a metric label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PathFailReason {
    /// The space has no navmesh at all — every consumer fails open, so
    /// this NPC was never going to route. A zone-level content gap, not
    /// a per-NPC one.
    NoMesh,
    /// There is a mesh and the pathfinder found no route, for a reason the
    /// caller could not name. Kept for callers that have no
    /// [`PathStatus`]; every AI state now reports one of the three below.
    NoPath,
    /// No polygon within `find_path`'s `±0.5` start box: the NPC is
    /// hovering, sunk or off the mesh (audit S9).
    NoStartPoly,
    /// No polygon within `±3.0` of the destination.
    NoEndPoly,
    /// Both ends snapped but A* found no corridor.
    NoCorridor,
    /// The corridor stops at the start's mesh island edge (audit S8). The
    /// caller still walks it, exactly as before NA02.
    Partial,
    /// A route came back with <= 1 waypoint, which is not actionable
    /// movement. The caller keeps whatever path it already had.
    DegeneratePath,
}

impl PathFailReason {
    fn label(self) -> &'static str {
        match self {
            PathFailReason::NoMesh => "no_mesh",
            PathFailReason::NoPath => "no_path",
            PathFailReason::NoStartPoly => "no_start_poly",
            PathFailReason::NoEndPoly => "no_end_poly",
            PathFailReason::NoCorridor => "no_corridor",
            PathFailReason::Partial => "partial",
            PathFailReason::DegeneratePath => "degenerate_path",
        }
    }

    /// Why `find_path` returned no waypoints. `status` is the navmesh's
    /// [`PathStatus`] (`None` when the space has no navmesh, or the NPC is
    /// not in a space). `no_mesh` means "this zone needs a `.nav`"; the
    /// stage reasons say where on an existing mesh the query failed.
    pub(super) fn for_missing_path(
        space_mgr: &SpaceManager,
        npc_id: u32,
        status: Option<PathStatus>,
    ) -> Self {
        match status {
            Some(PathStatus::NoStartPoly) => PathFailReason::NoStartPoly,
            Some(PathStatus::NoEndPoly) => PathFailReason::NoEndPoly,
            Some(PathStatus::NoCorridor) => PathFailReason::NoCorridor,
            Some(PathStatus::Partial) => PathFailReason::Partial,
            _ if space_mgr.space_has_navmesh(npc_id) => PathFailReason::NoPath,
            _ => PathFailReason::NoMesh,
        }
    }

    /// Classify any `find_path` result that did not yield actionable
    /// movement — i.e. `None`, or a `Some` of at most one waypoint.
    ///
    /// `patrol`, `wander`, `investigate` and `follow` used to
    /// `unwrap_or_default()` the result *before* classifying, which
    /// collapsed `None` and `Some(one_waypoint)` into the same shape and
    /// reported both as `no_mesh` / `no_path`. Those are different
    /// findings: `None` is the pathfinder declining, a one-waypoint path
    /// is the pathfinder answering with something that cannot be walked.
    /// Only the second is `degenerate_path`.
    pub(super) fn classify(
        space_mgr: &SpaceManager,
        npc_id: u32,
        status: Option<PathStatus>,
        path: Option<&[Vector3]>,
    ) -> Self {
        match path {
            None => Self::for_missing_path(space_mgr, npc_id, status),
            Some(_) => PathFailReason::DegeneratePath,
        }
    }
}

/// What the caller did *after* the routing failure.
///
/// The emitter cannot infer this from [`PathFailReason`], and guessing
/// was wrong: the shared message told operators every state was
/// "falling back to a straight line through geometry", but `fight`
/// enqueues nothing on either of its failure branches. An NPC cutting
/// through a wall and an NPC standing still are two different
/// player-visible symptoms with two different first questions, so the
/// caller — the only code that knows — passes it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PathFallback {
    /// `nav_path` was cleared and the raw destination pushed as a single
    /// waypoint. The NPC walks straight at it, through whatever geometry
    /// is in the way.
    DirectWaypoint,
    /// Nothing was enqueued. Whatever path the NPC already had is still
    /// in place, so it keeps walking a stale route — or stands still, if
    /// it had none.
    PathUnchanged,
    /// The partial corridor was installed as-is: the NPC walks to the edge
    /// of its mesh island, stops short of the destination, and repaths.
    PartialRoute,
}

impl PathFallback {
    fn label(self) -> &'static str {
        match self {
            PathFallback::DirectWaypoint => "direct_waypoint",
            PathFallback::PathUnchanged => "path_unchanged",
            PathFallback::PartialRoute => "partial_route",
        }
    }
}

/// One NPC's failed routing attempt.
pub(super) struct PathFailure {
    pub npc_id: u32,
    /// The AI state that was routing: `fight` | `follow` | `patrol` |
    /// `investigate` | `wander`. Metric label, so keep it to the
    /// handler set.
    pub state: &'static str,
    /// The `decision_outcome` vocabulary value for this failure, per
    /// `docs/architecture/observability.md`. A **log field only** — the
    /// handler still records its own terminal outcome onto the span and
    /// the `npc_ai_decisions_total` counter, because the NPC does
    /// usually still move (down the straight-line fallback).
    pub decision_outcome: &'static str,
    pub from: Vector3,
    pub to: Vector3,
    pub reason: PathFailReason,
    /// What the handler did about it — see [`PathFallback`]. The
    /// message is chosen from this, not from `reason`.
    pub fallback: PathFallback,
    /// The NPC's current target, when the state has one (`fight`,
    /// `follow`). Omitted from the row entirely when `None`.
    pub target_id: Option<u32>,
}

/// Emit the throttled `npc_ai.path_fail` row and count the occurrence.
///
/// Level is WARN for every state. An NPC that cannot route is
/// player-visible (it walks through a wall, or stands still while the
/// player waits for it) and it is not self-correcting, which is the
/// negative-logging convention's bar for WARN. This raises `fight`'s
/// `no_path`, previously INFO, to match — with the throttle above, the
/// volume is lower than it was at INFO.
pub(super) fn report_path_failure(space_mgr: &mut SpaceManager, f: PathFailure, now: Instant) {
    let PathFailure {
        npc_id,
        state,
        decision_outcome,
        from,
        to,
        reason,
        fallback,
        target_id,
    } = f;

    // The caller is about to walk the raw destination: remember that for
    // `npc_off_mesh`'s `last_move_source`.
    if fallback == PathFallback::DirectWaypoint {
        space_mgr
            .npc_detectors
            .note_move_source(npc_id, super::detectors::MoveSource::Fallback);
    }

    // Immutable lookups first: the throttle below takes `&mut`, and
    // `world_name_for_space` hands back a borrow of `self`.
    let world = space_mgr
        .get_entity_space_id(npc_id)
        .and_then(|sid| space_mgr.world_name_for_space(sid))
        .unwrap_or("unknown")
        .to_string();
    let navmesh_hash = space_mgr.navmesh_short_hash(npc_id).map(str::to_owned);
    let reason_label = reason.label();

    // Counted every tick, logged on some of them — the throttle must
    // not deflate the rate an operator alerts on.
    // A partial route is walked, not failed: it has its own counter and
    // its own throttle window, so a chase repathing into an island edge
    // every tick cannot hold back a real `no_start_poly` / `no_corridor`
    // row for the same NPC (and `npc_path_fail_total` keeps meaning "no
    // usable route").
    let partial = reason == PathFailReason::Partial;
    if partial {
        cimmeria_observability::counter!(
            "npc_path_partial_total",
            "world" => world.clone(),
            "state" => state,
        );
    } else {
        cimmeria_observability::counter!(
            "npc_path_fail_total",
            "world" => world.clone(),
            "state" => state,
            "reason" => reason_label,
        );
    }

    // One kind: every AI state's path failure shares a window, because
    // a stuck NPC does not change state and the row already names which
    // handler was routing.
    let Some(suppressed) = space_mgr.movement_telemetry.npc_path_fail_log.admit(
        npc_id,
        if partial { "path_partial" } else { "path_fail" },
        now,
        PATH_FAIL_LOG_MIN_INTERVAL,
    ) else {
        return;
    };

    // Two message shapes, one call site — keyed on what the handler
    // actually did, not on why the pathfinder failed. The two are
    // independent: `fight` leaves the path alone on a degenerate repath
    // *and* on an outright no-path, while `patrol` / `wander` /
    // `investigate` / `follow` push the raw destination in both cases.
    let message = match fallback {
        PathFallback::PathUnchanged => format!(
            "npc_ai.path_fail: {state} got no usable navmesh route and enqueued nothing \
             -- the NPC keeps whatever path it already had (walking toward where the \
             target used to be), or stands still if it had none"
        ),
        PathFallback::DirectWaypoint => format!(
            "npc_ai.path_fail: {state} got no usable navmesh route \
             -- falling back to a straight line through geometry toward the destination"
        ),
        PathFallback::PartialRoute => format!(
            "npc_ai.path_fail: {state} got a partial navmesh route -- the destination \
             is on another mesh island, so the NPC walks to the edge of its own and stops short"
        ),
    };

    tracing::warn!(
        target: "npc_ai.path_fail",
        event = "path_fail",
        decision_outcome,
        state,
        reason = reason_label,
        fallback = fallback.label(),
        npc_id,
        target_id,
        world = %world,
        npc_x = from.x,
        npc_y = from.y,
        npc_z = from.z,
        dest_x = to.x,
        dest_y = to.y,
        dest_z = to.z,
        dist = from.distance_to(&to),
        // The air-climb signature: a large positive dy on a fallback
        // means the straight line will drag the NPC upward through
        // geometry rather than around it.
        dy = to.y - from.y,
        navmesh_hash = navmesh_hash.as_deref(),
        suppressed,
        "{message}"
    );
}
