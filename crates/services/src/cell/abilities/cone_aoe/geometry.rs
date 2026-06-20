//! Cone geometry — the containment test and the per-effect recognizer.
//!
//! `is_cone_effect` classifies an effect as cone-collected; `collect_cone_targets`
//! does the planar (X/Z) cone-containment scan against hostile NPCs.

use cimmeria_entity::abilities::{EffectDef, TCM_AE_CONE};

use super::super::super::combat;
use super::super::super::space_manager::SpaceManager;

use super::super::super::combat::HOSTILE_FACTION;

/// Should this effect drive a cone fan-out at primary-cast time?
///
/// Returns `true` only for effects authored as TCM_AECone. Other TCM
/// values are handled elsewhere (single, radius via ground-target).
pub fn is_cone_effect(effect: &EffectDef) -> bool {
    effect.target_collection_method == TCM_AE_CONE
}

/// Collect every hostile entity inside the cone defined by `(source,
/// primary_target, length, half_angle)`. The primary target is excluded
/// because it already took damage on the single-target path.
///
/// Returns entity ids sorted by distance from the apex so callers can
/// reason deterministically (e.g. limit to N nearest secondaries when
/// that becomes a feature).
pub fn collect_cone_targets(
    space_mgr: &SpaceManager,
    attacker_id: u32,
    primary_target_id: u32,
    cone_length: f32,
    half_angle: f32,
) -> Vec<u32> {
    let attacker = match space_mgr.get_entity(attacker_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let primary = match space_mgr.get_entity(primary_target_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let attacker_space = attacker.space_id;
    let apex = [
        attacker.position.x,
        attacker.position.y,
        attacker.position.z,
    ];

    // Direction vector from attacker to primary target in the X/Z plane
    // (Y is up in this engine). A vertical-only offset between attacker
    // and primary (rare — same X/Z) collapses the cone direction; skip
    // fan-out in that degenerate case rather than picking an arbitrary
    // facing.
    let dx = primary.position.x - apex[0];
    let dz = primary.position.z - apex[2];
    let dir_len = (dx * dx + dz * dz).sqrt();
    if dir_len < 1e-3 {
        tracing::debug!(
            attacker_id,
            primary_target_id,
            "cone_aoe: primary stacked on attacker — skipping cone fan-out"
        );
        return Vec::new();
    }
    let cone_dx = dx / dir_len;
    let cone_dz = dz / dir_len;

    let cos_half_angle = half_angle.cos();
    let length_sq = cone_length * cone_length;

    let mut hits: Vec<(u32, f32)> = Vec::new();
    for npc_eid in space_mgr.all_npc_entity_ids() {
        if npc_eid == primary_target_id || npc_eid == attacker_id {
            continue;
        }
        let npc = match space_mgr.get_entity(npc_eid) {
            Some(e) => e,
            None => continue,
        };
        if npc.space_id != attacker_space {
            continue;
        }
        if combat::is_dead_state(npc.state_field) {
            continue;
        }
        if npc.faction != HOSTILE_FACTION {
            continue;
        }
        let ex = npc.position.x - apex[0];
        let ez = npc.position.z - apex[2];
        let dist_sq_xz = ex * ex + ez * ez;
        // Use planar distance for cone containment; the original game's
        // cones are effectively 2D (cylindrical sections in 3D), and
        // anyone within the X/Z cone is "in the line of fire" regardless
        // of vertical offset short of going through a floor — which the
        // cell doesn't validate anyway pre-navmesh-LOS.
        if dist_sq_xz > length_sq {
            continue;
        }
        // Skip the apex itself (entities directly on the attacker would
        // make `to_npc_len` zero and the dot-product undefined).
        if dist_sq_xz < 1e-6 {
            continue;
        }
        let to_npc_len = dist_sq_xz.sqrt();
        let nx = ex / to_npc_len;
        let nz = ez / to_npc_len;
        let dot = nx * cone_dx + nz * cone_dz;
        if dot >= cos_half_angle {
            hits.push((npc_eid, dist_sq_xz));
        }
    }
    hits.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    hits.into_iter().map(|(eid, _)| eid).collect()
}
