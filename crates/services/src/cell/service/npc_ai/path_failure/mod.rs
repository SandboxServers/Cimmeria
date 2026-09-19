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
    /// There is a mesh and the pathfinder found no route: the NPC or its
    /// destination is off-mesh, or they are in disconnected components.
    NoPath,
    /// A route came back with <= 1 waypoint, which is not actionable
    /// movement. The caller keeps whatever path it already had.
    DegeneratePath,
}

impl PathFailReason {
    fn label(self) -> &'static str {
        match self {
            PathFailReason::NoMesh => "no_mesh",
            PathFailReason::NoPath => "no_path",
            PathFailReason::DegeneratePath => "degenerate_path",
        }
    }

    /// Pick between `no_mesh` and `no_path` from the space's state.
    /// Callers that got `None` from `find_path` cannot tell the two
    /// apart themselves, and the distinction is the whole diagnostic
    /// value: `no_mesh` means "this zone needs a `.nav`", `no_path`
    /// means "the mesh is there and has a hole or a split".
    pub(super) fn for_missing_path(space_mgr: &SpaceManager, npc_id: u32) -> Self {
        if space_mgr.space_has_navmesh(npc_id) {
            PathFailReason::NoPath
        } else {
            PathFailReason::NoMesh
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
        target_id,
    } = f;

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
    cimmeria_observability::counter!(
        "npc_path_fail_total",
        "world" => world.clone(),
        "state" => state,
        "reason" => reason_label,
    );

    let Some(suppressed) = space_mgr.movement_telemetry.npc_path_fail_log.admit(
        npc_id,
        now,
        PATH_FAIL_LOG_MIN_INTERVAL,
    ) else {
        return;
    };

    // Two message shapes, one call site: a degenerate path leaves the
    // previous route in place (the NPC keeps walking somewhere stale),
    // which is a different player-visible symptom from falling back to
    // a straight line, and the message has to say which.
    let message = match reason {
        PathFailReason::DegeneratePath => format!(
            "npc_ai.path_fail: {state} repath returned a degenerate path (<=1 waypoint) \
             -- previous path left in place, NPC may walk toward where the target used to be"
        ),
        _ => format!(
            "npc_ai.path_fail: {state} found no navmesh path \
             -- falling back to a straight line through geometry"
        ),
    };

    tracing::warn!(
        target: "npc_ai.path_fail",
        event = "path_fail",
        decision_outcome,
        state,
        reason = reason_label,
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
