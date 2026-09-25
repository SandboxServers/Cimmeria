//! `npc_ai.los`: the evidence behind a line of sight that was not clear
//! (audit T9 / S11 / S15).
//!
//! One DEBUG row per `(entity_a, entity_b)` pair per [`LOS_SAMPLE_INTERVAL`],
//! with the raw endpoints, the points the ray was actually cast between, the
//! eye height added, and where it stopped. `source` says what answered:
//! `occluder` (the world's collision-geometry `.occ`, NA27: eye to eye) or
//! `navmesh` (the Detour ray along the floor, no eye height). This replaces the unsampled `movement.navmesh
//! reason=los_unknown_off_mesh` debug line, which fired on every query with
//! an off-mesh endpoint once NA00 exported `movement.navmesh`.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{LineOfSight, LosProbe};

use crate::cell::space_manager::SpaceManager;

pub(in crate::cell) const LOS_SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// What answered a line-of-sight query.
#[derive(Debug, Clone, Copy)]
pub(in crate::cell) enum LosSource<'a> {
    /// The navmesh ray, with the mesh's short hash. It adds no eye height:
    /// `NavMesh::line_of_sight` projects both entities onto the walkable
    /// surface and ray-casts along it, which is why it cannot see ceilings
    /// or the floor between storeys (S15).
    Navmesh(&'a str),
    /// The collision-geometry occluder (NA27), with its short hash and the
    /// eye height added to the looker.
    Occluder { hash: &'a str, eye_height: f32 },
}

impl LosSource<'_> {
    fn label(self) -> &'static str {
        match self {
            Self::Navmesh(_) => "navmesh",
            Self::Occluder { .. } => "occluder",
        }
    }

    fn eye_height(self) -> f32 {
        match self {
            Self::Navmesh(_) => 0.0,
            Self::Occluder { eye_height, .. } => eye_height,
        }
    }
}

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
    source: LosSource<'_>,
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
        eye_height_used = source.eye_height(),
        source = source.label(),
        ray_from = ?probe.from,
        ray_to = ?probe.to,
        hit_xyz = ?probe.hit,
        dy = b_pos.y - a_pos.y,
        dist = a_pos.distance_to(&b_pos),
        navmesh_hash = match source {
            LosSource::Navmesh(h) => Some(h),
            LosSource::Occluder { .. } => None,
        },
        occluder_hash = match source {
            LosSource::Occluder { hash, .. } => Some(hash),
            LosSource::Navmesh(_) => None,
        },
        origin,
        suppressed,
        "npc_ai.los: line of sight not clear ({result})"
    );
}
