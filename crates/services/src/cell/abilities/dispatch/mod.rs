//! Ground-target AoE entry point.
//!
//! `useAbilityOnGroundTarget` arrives from the client when the player fires an
//! ability without picking an explicit entity (point-and-click on terrain).
//! We collect every hostile NPC inside the AoE radius (read from the ability's
//! effect definition, defaulting to `DEFAULT_GROUND_TARGET_RADIUS` when the
//! NVP is absent) and apply damage to all of them. The cooldown/ammo are
//! consumed exactly once via the primary-target call to `handle_use_ability`;
//! additional targets get `damage_apply::apply_damage_to_target` directly so
//! we don't re-charge per target. When the primary has a warmup (AT-10) the
//! launch parks it with the ground point, and the warmup tick fires the
//! primary and the secondaries together (`fire_ground_cast_after_warmup`).

use tokio::sync::mpsc;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::damage_apply::apply_damage_to_target;
use super::use_ability::handle_use_ability;

/// Fallback radius for ground-target abilities when the effect definition's
/// `Radius` NVP is missing. Matches the pre-#81 hardcoded value so existing
/// abilities without explicit radius data behave the same as before.
const DEFAULT_GROUND_TARGET_RADIUS: f32 = 5.0;

/// Read the AoE radius from the ability's **first** effect definition's
/// `Radius` NVP. Returns `DEFAULT_GROUND_TARGET_RADIUS` when the ability is
/// unknown, has no effects, the first effect isn't loaded, or the first
/// effect doesn't specify a positive radius. We deliberately don't scan
/// later effects for a radius — chains author the primary-effect radius on
/// the first effect, and silently picking up a different effect's
/// `Radius` would diverge from the authored intent.
fn ability_radius(
    ability_def: &Option<cimmeria_entity::abilities::AbilityDef>,
    space_mgr: &SpaceManager,
) -> f32 {
    let Some(def) = ability_def else {
        return DEFAULT_GROUND_TARGET_RADIUS;
    };
    let Some(&first_effect_id) = def.effect_ids.first() else {
        return DEFAULT_GROUND_TARGET_RADIUS;
    };
    let Some(effect) = space_mgr.effect_defs.get(&first_effect_id) else {
        return DEFAULT_GROUND_TARGET_RADIUS;
    };
    let r = effect.param_f32("Radius");
    if r > 0.0 {
        r
    } else {
        DEFAULT_GROUND_TARGET_RADIUS
    }
}

/// Handle a ground-targeted ability — applies damage to **every** hostile
/// NPC within the AoE radius, not just the nearest. The radius comes from
/// the ability's first effect definition (`Radius` NVP), falling back to
/// a hardcoded default. Cooldown and ammo are consumed exactly once: the
/// primary (nearest) target goes through `handle_use_ability`, then each
/// additional target gets `apply_damage_to_target` directly with
/// `needs_ammo_stat_send: false` so we don't double-flush the bandolier
/// stat.
///
/// **Secondary targets only fire if the primary cast committed** —
/// `handle_use_ability` returns `false` when the call is rejected by a
/// pre-consume guard (cooldown, no ammo, attacker dead, target dead
/// pre-cast, out-of-range). The AoE loop bails on `false` so a rejected
/// ability doesn't deliver free damage to bystanders.
///
/// **Targets are constrained to the attacker's space.** `all_npc_entity_ids`
/// scans every cell, so without filtering by `space_id` we'd happily
/// damage NPCs in completely different worlds that overlap in coordinate
/// space (instanced dungeons, multi-map deployments).
///
/// If no enemy is in range, the call still consumes cooldown and ammo via
/// `handle_use_ability(target_id=0)` so the player can't spam.
///
/// Returns the entity IDs of every NPC that **died** during this cast
/// (primary + secondaries). Empty Vec when the cast was rejected, no
/// targets in radius, or nothing died. The caller fires
/// `fire_entity_death` for each id so kill-count missions and other
/// death-triggered chains advance for AoE kills, not just the primary.
pub async fn handle_use_ability_on_ground(
    entity_id: u32,
    ability_id: i32,
    ground: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Vec<u32> {
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();
    let radius = ability_radius(&ability_def, space_mgr);
    let radius_sq = radius * radius;

    // Pin AoE candidates to the attacker's space — `all_npc_entity_ids`
    // walks every loaded space and we don't want a ground click in
    // Castle_Cellblock to damage NPCs at the same coords in Agnos.
    let attacker_space = match space_mgr.get_entity(entity_id) {
        Some(e) => e.space_id,
        None => {
            tracing::warn!(
                entity_id,
                "useAbilityOnGroundTarget: attacker entity not found"
            );
            return Vec::new();
        }
    };

    // Collect every hostile NPC within the AoE radius, sorted by distance
    // from the click point. The first becomes the primary (consumes
    // cooldown/ammo), the rest get damage applied directly.
    let targets = collect_ground_targets(space_mgr, attacker_space, ground, radius_sq);

    // Primary range check: the closest target must also be within the
    // ability's own `max_range` from the attacker, otherwise
    // handle_use_ability bails before the cooldown starts and the player
    // can spam-click. Falling back to target_id = 0 keeps the
    // cooldown/ammo charge in that case.
    let max_range = ability_def.as_ref().map_or(30.0, |d| {
        if d.max_range > 0 {
            d.max_range as f32
        } else {
            30.0
        }
    });

    let primary_in_range = targets.first().is_some_and(|&(target_eid, _)| {
        match (
            space_mgr.get_entity(entity_id),
            space_mgr.get_entity(target_eid),
        ) {
            (Some(attacker), Some(target)) => {
                attacker.position.distance_to(&target.position) <= max_range
            }
            _ => false,
        }
    });

    if targets.is_empty() {
        tracing::debug!(
            entity_id, ability_id, ?ground, radius,
            "useAbilityOnGroundTarget: no enemy in AoE radius; consuming cooldown/ammo without damage"
        );
        handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
        return Vec::new();
    }

    if !primary_in_range {
        let (primary_eid, _) = targets[0];
        tracing::debug!(
            entity_id, ability_id, ?ground, primary_eid, max_range,
            "useAbilityOnGroundTarget: nearest target outside attacker's ability max_range; charging cooldown/ammo only"
        );
        handle_use_ability(entity_id, ability_id, 0, tx, space_mgr).await;
        return Vec::new();
    }

    // Snapshot HEALTH for every target BEFORE damage so we can detect
    // alive→dead transitions per-target after the cast resolves and
    // surface them to the caller for `fire_entity_death`.
    let alive_before = alive_snapshot(space_mgr, &targets);

    // Primary target: full handle_use_ability path (consumes ammo,
    // starts cooldown, sends timer/sequence/state-field, applies damage).
    let (primary_eid, _) = targets[0];
    tracing::debug!(
        entity_id,
        ability_id,
        ?ground,
        primary_eid,
        radius,
        target_count = targets.len(),
        "useAbilityOnGroundTarget: AoE — primary target via handle_use_ability"
    );
    let primary_committed =
        handle_use_ability(entity_id, ability_id, primary_eid as i32, tx, space_mgr).await;

    if !primary_committed {
        // Pre-consume guard rejected the cast (cooldown, ammo, dead, etc.).
        // Don't apply secondary damage — that would deal free hits despite
        // the primary failing validation. Empty Vec means no kills.
        tracing::debug!(
            entity_id,
            ability_id,
            primary_eid,
            "useAbilityOnGroundTarget: primary cast rejected; suppressing AoE secondaries"
        );
        return Vec::new();
    }

    // The primary went into its warmup (AT-10): nothing has been damaged
    // yet. Keep the ground point on the parked cast; the warmup tick fires
    // the primary and collects the secondaries then
    // (`fire_ground_cast_after_warmup`).
    if super::use_ability::attach_ground_point(space_mgr, entity_id, ground) {
        tracing::debug!(
            entity_id,
            ability_id,
            primary_eid,
            "useAbilityOnGroundTarget: primary is warming up; AoE secondaries deferred to the fire"
        );
        return Vec::new();
    }

    apply_secondaries(
        entity_id,
        ability_id,
        &ability_def,
        &targets[1..],
        tx,
        space_mgr,
    )
    .await;
    deaths_since(space_mgr, alive_before)
}

/// Fire a ground cast whose primary finished its warmup (AT-10).
///
/// The warmup tick has already re-validated `primary_eid` and taken the
/// parked cast. The primary fires through the normal post-warmup path;
/// the secondaries are collected around `ground` now, not at launch, since
/// the NPCs have had the warmup to move. Returns every NPC that died, as
/// [`handle_use_ability_on_ground`] does.
pub(super) async fn fire_ground_cast_after_warmup(
    entity_id: u32,
    ability_id: i32,
    primary_eid: u32,
    effect_seq: i32,
    ground: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Vec<u32> {
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();
    let radius = ability_radius(&ability_def, space_mgr);
    let Some(attacker_space) = space_mgr.get_entity(entity_id).map(|e| e.space_id) else {
        return Vec::new();
    };
    let mut targets = vec![(primary_eid, 0.0)];
    targets.extend(
        collect_ground_targets(space_mgr, attacker_space, ground, radius * radius)
            .into_iter()
            .filter(|&(eid, _)| eid != primary_eid),
    );
    let alive_before = alive_snapshot(space_mgr, &targets);

    super::use_ability::fire_cast(
        entity_id,
        ability_id,
        primary_eid as i32,
        effect_seq,
        &ability_def,
        tx,
        space_mgr,
    )
    .await;
    apply_secondaries(
        entity_id,
        ability_id,
        &ability_def,
        &targets[1..],
        tx,
        space_mgr,
    )
    .await;
    deaths_since(space_mgr, alive_before)
}

/// Every live hostile NPC in `attacker_space` within `radius_sq` of
/// `ground`, nearest first.
///
/// Hostile-faction sentinel matches `cell_methods/player/interaction.rs`'s
/// hostile check. Without it, AoE would happily damage vendors,
/// quest givers, and neutral wildlife. Imported from `combat::`
/// so the single sentinel is the source of truth.
fn collect_ground_targets(
    space_mgr: &SpaceManager,
    attacker_space: cimmeria_common::SpaceId,
    ground: [f32; 3],
    radius_sq: f32,
) -> Vec<(u32, f32)> {
    use crate::cell::combat::HOSTILE_FACTION;
    let mut targets: Vec<(u32, f32)> = Vec::new();
    for npc_eid in space_mgr.all_npc_entity_ids() {
        if let Some(npc) = space_mgr.get_entity(npc_eid) {
            if npc.space_id != attacker_space {
                continue;
            }
            if combat::is_dead_state(npc.state_field) {
                continue;
            }
            if npc.faction != HOSTILE_FACTION {
                continue;
            }
            let dx = npc.position.x - ground[0];
            let dy = npc.position.y - ground[1];
            let dz = npc.position.z - ground[2];
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq <= radius_sq {
                targets.push((npc_eid, dist_sq));
            }
        }
    }
    targets.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    targets
}

/// `(entity, alive)` for each target, sampled before damage.
fn alive_snapshot(space_mgr: &SpaceManager, targets: &[(u32, f32)]) -> Vec<(u32, bool)> {
    targets
        .iter()
        .map(|&(eid, _)| {
            let alive = space_mgr.get_entity(eid).is_some_and(|e| {
                e.stats
                    .get(cimmeria_entity::stats::HEALTH)
                    .is_some_and(|s| s.cur > 0)
            });
            (eid, alive)
        })
        .collect()
}

/// Apply the AoE damage to each secondary target.
async fn apply_secondaries(
    entity_id: u32,
    ability_id: i32,
    ability_def: &Option<cimmeria_entity::abilities::AbilityDef>,
    secondaries: &[(u32, f32)],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Secondary targets: damage only, fresh effect_seq per target so the
    // client can correlate per-target effect packets independently. Each
    // `next_effect_id()` call mints a unique value off the attacker's
    // ability manager.
    for &(secondary_eid, _) in secondaries {
        let secondary_seq = space_mgr
            .get_entity_mut(entity_id)
            .map(|e| e.abilities.next_effect_id())
            .unwrap_or(0);
        tracing::debug!(
            entity_id,
            ability_id,
            secondary_eid,
            secondary_seq,
            "useAbilityOnGroundTarget: AoE — secondary target via apply_damage_to_target"
        );
        apply_damage_to_target(
            entity_id,
            secondary_eid,
            ability_id,
            ability_def,
            secondary_seq as u32,
            // No ammo flush on secondaries — primary already flushed.
            false,
            tx,
            space_mgr,
        )
        .await;
    }
}

/// Alive→dead transitions across all targets, so the caller can fire
/// entity_death for each kill. Without this, AoE secondary kills miss
/// kill-count missions and other death-triggered chains.
fn deaths_since(space_mgr: &SpaceManager, alive_before: Vec<(u32, bool)>) -> Vec<u32> {
    let mut deaths = Vec::new();
    for (eid, was_alive) in alive_before {
        if !was_alive {
            continue;
        }
        let now_dead = space_mgr.get_entity(eid).is_some_and(|e| {
            e.stats
                .get(cimmeria_entity::stats::HEALTH)
                .is_some_and(|s| s.cur <= 0)
        });
        if now_dead {
            deaths.push(eid);
        }
    }
    deaths
}

#[cfg(test)]
mod tests;
