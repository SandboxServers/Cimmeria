//! Ability bucket choice + range geometry for the Fighting handler:
//! pick an off-cooldown ability, resolve its `(max_range, min_range)`,
//! and compute the min-range backup waypoint.

use cimmeria_entity::abilities::AbilityDef;

use crate::cell::space_manager::SpaceManager;

/// The distance at which a given ability can actually be used.
///
/// Two different "use the server default" sentinels collapse here, and
/// which one applies is decided by `is_ranged`, not by the ability row
/// alone:
///
/// - **`is_ranged = false` → [`crate::cell::combat::NPC_MELEE_RANGE`].** A
///   swing has swing reach. This wins even over a non-zero `max_range`,
///   because 148 melee rows in `resources.abilities` carry a `max_range`
///   between 100 and 2500 — plainly not the metres the fight tick measures
///   in — and honouring those would reproduce the very defect this gate
///   exists to remove, in a worse form.
/// - **`is_ranged = true`** → the def's own `max_range` when non-zero, else
///   `npc_attack_range` (the `0` sentinel).
///
/// A **missing def** resolves to `npc_attack_range`, not the melee reach:
/// an ability the server knows nothing about must not be silently confined
/// to 3 m. This keeps the pre-existing behaviour for every def-less ability
/// the fixtures and the `NPC_DEFAULT_ABILITY` fallback rely on.
fn effective_max_range(def: Option<&AbilityDef>, npc_attack_range: f32) -> f32 {
    match def {
        Some(d) if !d.is_ranged => crate::cell::combat::NPC_MELEE_RANGE,
        Some(d) if d.max_range > 0 => d.max_range as f32,
        _ => npc_attack_range,
    }
}

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
///
/// # Multi-ability sets
///
/// The bucket is whatever `ability_set_abilities` held for the template's
/// `ability_set_id` — since Harset packet H09 widened that table's primary
/// key to `(ability_set_id, ability_id)`, a set can carry more than one
/// ability and this walk is what makes the extra rows reachable. Ascending
/// id order means the lowest-id ability is the NPC's primary and the rest
/// are strictly cooldown fallbacks; that is a property of the data, not of
/// this function, so composing a set is a decision about which ability
/// should win the tie.
pub(in crate::cell) fn choose_npc_ability(npc_id: u32, space_mgr: &SpaceManager) -> Option<i32> {
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

/// [`choose_npc_ability`] with a reach filter: prefer the lowest-id
/// off-cooldown ability that can actually be used at `target_dist`, and
/// fall back to the unfiltered pick when none can.
///
/// This is the production entry point; the unfiltered
/// [`choose_npc_ability`] is the fallback arm and stays reachable on its
/// own so the selector tests keep exercising the partition directly.
///
/// # Why the filter exists
///
/// `ability_set_abilities` could hold one row per set until Harset packet
/// H09 widened its primary key, so no NPC could own both a ranged and a
/// melee ability and the question never arose. It does now: set 4 is
/// `584 Staff Auto Attack` + `710 Staff Melee AA` and set 5 is
/// `711 Ribbon Device Melee AA` + `712 Ribbon Device Auto Attack`. Since
/// the unfiltered pick is "lowest off-cooldown id", set 5's *primary* would
/// be the melee half — so without this filter a Goa'uld at 20 m plays a
/// ribbon swing, and a Jaffa plays a staff swing on every tick 584 is
/// cooling. Twelve of the fifteen stationary spawn rows in the seed use set
/// 4, and a pinned sentry can never close the gap to make the swing
/// truthful.
///
/// The 2009 Python selector never range-gated either —
/// `deprecated/python/cell/SGWMob.py:227` carries the literal
/// `# TODO: Check distance, LOS` above its `return ABILITY_Usable`. This is
/// that TODO, server-side and client-compatible.
///
/// # Why a fallback rather than `None`
///
/// Returning `None` when nothing is in reach would make the caller treat
/// the NPC as "all cooling" and hold fire — a melee-only NPC would freeze
/// at distance instead of walking in. Handing back the out-of-reach pick
/// instead lets [`ability_ranges`] report its real (short) `max_range`, so
/// the fight tick's existing out-of-range arm does the right thing for
/// free: a mobile NPC chases, and a stationary one holds and turns to face.
///
/// # Effect on single-ability sets
///
/// None. Every set that predates H09 — 1 (`579`), 2 (`221`), 3 (`559`), and
/// the `NPC_DEFAULT_ABILITY` (`592`) empty-bucket fallback — holds one
/// `is_ranged = true` ability, so the filter either accepts the same pick
/// the unfiltered walk would have made, or accepts nothing and delegates to
/// it. Sets 4 and 5 are the only sets in the seed carrying a melee ability.
pub(in crate::cell) fn choose_npc_ability_within_reach(
    npc_id: u32,
    space_mgr: &SpaceManager,
    target_dist: f32,
    npc_attack_range: f32,
) -> Option<i32> {
    let npc = space_mgr.get_entity(npc_id)?;

    let mut ability_ids = npc.abilities.known_ability_ids();
    ability_ids.sort_unstable();

    let in_reach = ability_ids.into_iter().find(|&id| {
        !npc.abilities.is_on_cooldown(id)
            && effective_max_range(space_mgr.ability_defs.get(&id), npc_attack_range) >= target_dist
    });

    in_reach.or_else(|| choose_npc_ability(npc_id, space_mgr))
}

/// Resolve `(max_range, min_range)` for a chosen ability, falling back
/// to the server-default `NPC_ATTACK_RANGE` when the def is missing or
/// the field carries the `0` sentinel meaning "use server default."
///
/// `max_range` goes through [`effective_max_range`], so a melee ability
/// (`is_ranged = false`) reports `NPC_MELEE_RANGE` rather than the ranged
/// default — that is what turns the fight tick's existing out-of-range arm
/// into "walk in before swinging."
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
    let max_range = effective_max_range(def, npc_attack_range);
    let min_range = def.map_or(0.0, |d| {
        if d.min_range > 0 {
            d.min_range as f32
        } else {
            0.0
        }
    });
    (max_range, min_range)
}

/// Step back from the target, horizontally, to a point `min_range + 1.0`
/// from it in X and Z, at the NPC's own height. Returns `None` when the
/// target is straight above or below the NPC (no horizontal direction to
/// back away along).
///
/// The +1.0 margin keeps the next tick's range check from oscillating
/// at exactly `min_range`; without it floating-point jitter would push
/// the NPC back inside the dead zone every other tick. A horizontal
/// distance of `min_range + 1.0` is at least that in 3D, so the margin
/// holds whatever the height difference.
///
/// # Why horizontal
///
/// This used to extrapolate the full 3D target→NPC vector, keeping its
/// vertical component. A player standing 5 u above the NPC put the backup
/// point under the floor, and one below put it in the air (audit M5).
/// Nothing re-grounded it: the waypoint went straight into `nav_path`.
/// The point returned here is still raw; the fight handler goes through
/// [`backup_waypoint_on_mesh`], which slides it across the navmesh.
pub(super) fn compute_backup_waypoint(
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
    min_range: f32,
) -> Option<cimmeria_common::Vector3> {
    let dx = npc_pos.x - target_pos.x;
    let dz = npc_pos.z - target_pos.z;
    let dist = (dx * dx + dz * dz).sqrt();
    if dist < f32::EPSILON {
        return None;
    }
    let scale = (min_range + 1.0) / dist;
    Some(cimmeria_common::Vector3::new(
        target_pos.x + dx * scale,
        npc_pos.y,
        target_pos.z + dz * scale,
    ))
}

/// The min-range backup waypoint, on the walkable surface.
///
/// [`compute_backup_waypoint`] picks the direction; the navmesh then slides
/// the NPC from where it stands toward that point with Detour's
/// `moveAlongSurface`. The slide stops at a wall or ledge instead of
/// passing through it, and the result sits on the floor of the storey the
/// NPC is on. Without a navmesh, or with the NPC off it, the raw point is
/// used, which is still at the NPC's own height.
pub(super) fn backup_waypoint_on_mesh(
    space_mgr: &SpaceManager,
    npc_id: u32,
    npc_pos: cimmeria_common::Vector3,
    target_pos: cimmeria_common::Vector3,
    min_range: f32,
) -> Option<cimmeria_common::Vector3> {
    let raw = compute_backup_waypoint(npc_pos, target_pos, min_range)?;
    Some(
        space_mgr
            .move_along_navmesh(npc_id, &npc_pos, &raw)
            .unwrap_or(raw),
    )
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
