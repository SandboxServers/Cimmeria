//! `npc_ai.los`: the evidence behind a line of sight that was not clear
//! (audit T9 / S11 / S15).
//!
//! One DEBUG row per `(entity_a, entity_b)` pair per [`LOS_SAMPLE_INTERVAL`],
//! with the raw endpoints, the points the ray was actually cast between, the
//! eye height added (none — the navmesh ray runs along the floor), and where
//! it stopped. This replaces the unsampled `movement.navmesh
//! reason=los_unknown_off_mesh` debug line, which fired on every query with
//! an off-mesh endpoint once NA00 exported `movement.navmesh`.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{LineOfSight, LosProbe};

use crate::cell::space_manager::SpaceManager;

pub(in crate::cell) const LOS_SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// Eye height the line-of-sight query adds to each endpoint. Zero today:
/// `NavMesh::line_of_sight` projects both entities onto the walkable surface
/// and ray-casts along it, which is why it cannot see ceilings or the floor
/// between storeys (S15). Logged so a query can prove it.
const EYE_HEIGHT_USED: f32 = 0.0;

/// Report a non-clear probe, sampled. `a` is the looker (the NPC on every AI
/// call site), `b` the target. `origin` says where the ray started: `npc`
/// (the looker's position, `a_pos`) or `cover_peek` (NA23: `a_pos` is the
/// peek point of the cover slot the NPC stands at).
pub(in crate::cell) fn report(
    space_mgr: &SpaceManager,
    a: u32,
    b: u32,
    a_pos: Vector3,
    b_pos: Vector3,
    probe: &LosProbe,
    navmesh_hash: Option<&str>,
    origin: &'static str,
    now: Instant,
) {
    if probe.result == LineOfSight::Clear {
        return;
    }
    let result = probe.result.label();
    let Some(suppressed) = space_mgr
        .npc_detectors
        .los_log
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .admit(a, b, "los", now, LOS_SAMPLE_INTERVAL)
    else {
        return;
    };
    let (tag, template_id) = space_mgr
        .get_entity(a)
        .map(|e| {
            (
                e.tag.clone().unwrap_or_default(),
                e.template_id.unwrap_or(0),
            )
        })
        .unwrap_or_default();
    let world = super::super::world_label(space_mgr, a);
    tracing::debug!(
        target: "npc_ai.los",
        event = "blocked",
        npc_id = a,
        target_id = b,
        tag = %tag,
        template_id,
        world = %world,
        space_id = space_mgr.get_entity_space_id(a).unwrap_or(0),
        result,
        from_xyz = ?[a_pos.x, a_pos.y, a_pos.z],
        to_xyz = ?[b_pos.x, b_pos.y, b_pos.z],
        eye_height_used = EYE_HEIGHT_USED,
        ray_from = ?probe.from,
        ray_to = ?probe.to,
        hit_xyz = ?probe.hit,
        dy = b_pos.y - a_pos.y,
        dist = a_pos.distance_to(&b_pos),
        navmesh_hash,
        origin,
        suppressed,
        "npc_ai.los: line of sight not clear ({result})"
    );
}
