//! Ability bucket choice + range geometry for the Fighting handler:
//! pick an off-cooldown ability, resolve its `(max_range, min_range)`,
//! and compute the min-range backup waypoint.

use crate::cell::space_manager::SpaceManager;

/// Pick an off-cooldown ability for the NPC's fight tick. `None` → all
/// cooling → caller holds fire. Empty bucket falls back to
/// `NPC_DEFAULT_ABILITY` so a misconfigured template doesn't wedge silently.
///
/// Why no ammo gate: NPCs have infinite ammo (the `required_ammo > 0` check
/// at the dispatch site is player-only). Gating here would permanently
/// disable abilities like Pistol Shot 592 (`required_ammo = 1`) that every
/// stock NPC carries.
///
/// Stable sort over `known_ability_ids` keeps selection deterministic
/// tick-to-tick; a future "prefer higher threat_level_id" refinement
/// changes the ordering without touching the partition.
pub(in crate::cell::service) fn choose_npc_ability(
    npc_id: u32,
    space_mgr: &SpaceManager,
) -> Option<i32> {
    use crate::cell::combat;

    let npc = space_mgr.get_entity(npc_id)?;
    if npc.abilities.known_count() == 0 {
        return Some(combat::NPC_DEFAULT_ABILITY);
    }

    let mut ability_ids = npc.abilities.known_ability_ids();
    ability_ids.sort_unstable();

    ability_ids
        .into_iter()
        .find(|&id| !npc.abilities.is_on_cooldown(id))
}

/// Resolve `(max_range, min_range)` for a chosen ability, falling back
/// to the server-default `NPC_ATTACK_RANGE` when the def is missing or
/// the field carries the `0` sentinel meaning "use server default."
///
/// `min_range` is `0.0` when the def carries `0` (no minimum). Distinct
/// from `max_range` which never zeroes legitimately — `0` always means
/// "default to `npc_attack_range`."
///
/// `chosen_ability == None` → all-cooling case; we still need a
/// max_range for the "should we walk toward the target?" gate, so the
/// fallback applies the same way as a missing def.
///
/// Returned as `(max, min)` because the call site reads `max` first in
/// the in-range check.
pub(super) fn ability_ranges(
    chosen_ability: Option<i32>,
    space_mgr: &SpaceManager,
    npc_attack_range: f32,
) -> (f32, f32) {
    let def = chosen_ability.and_then(|id| space_mgr.ability_defs.get(&id));
    let max_range = def.map_or(npc_attack_range, |d| {
        if d.max_range > 0 {
            d.max_range as f32
        } else {
            npc_attack_range
        }
    });
    let min_range = def.map_or(0.0, |d| {
        if d.min_range > 0 {
            d.min_range as f32
        } else {
            0.0
        }
    });
    (max_range, min_range)
}

/// Step back along the target→NPC vector to a point at distance
/// `min_range + 1.0` from the target. Returns `None` if the NPC and
/// target are co-located (degenerate vector — can't normalize).
///
/// The +1.0 margin keeps the next tick's range check from oscillating
/// at exactly `min_range`; without it floating-point jitter would push
/// the NPC back inside the dead zone every other tick.
///
/// # Vertical-axis caveat
///
/// The returned waypoint preserves the NPC's Y-axis offset from the
/// target — if the NPC is uphill of the target, the backup point is
/// also uphill. This can yield a Y that the navmesh would reject (in
/// mid-air over a ledge, or under the floor). The waypoint is fed
/// into the same path-follower as `find_path` output, which clamps
/// invalid Y via the navmesh on consume. Callers that bypass that
/// path-follower must clamp themselves.
pub(super) fn compute_backup_waypoint(
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
    min_range: f32,
) -> Option<cimmeria_common::Vector3> {
    let dx = npc_pos.x - target_pos.x;
    let dy = npc_pos.y - target_pos.y;
    let dz = npc_pos.z - target_pos.z;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist < f32::EPSILON {
        return None;
    }
    let scale = (min_range + 1.0) / dist;
    Some(cimmeria_common::Vector3::new(
        target_pos.x + dx * scale,
        target_pos.y + dy * scale,
        target_pos.z + dz * scale,
    ))
}

/// Test-only re-export of the private `compute_backup_waypoint` so
/// the sibling `tests/npc_ai.rs` module can exercise its degenerate
/// (co-located NPC + target) branch without making the helper `pub`.
///
/// The helper stays private to enforce the convention that only
/// `npc_ai_fight` calls it (the `+1.0` margin assumption is tied to
/// that caller); production callers must go through the fight pass.
#[cfg(test)]
pub(in crate::cell::service) fn compute_backup_waypoint_for_test(
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
    min_range: f32,
) -> Option<cimmeria_common::Vector3> {
    compute_backup_waypoint(npc_pos, target_pos, min_range)
}
