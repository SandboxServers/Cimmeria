//! Cone fan-out dispatch — apply each cone effect's damage to the
//! secondaries it geometrically matched.
//!
//! Sits **above** `apply_damage_to_target` (called once per ability
//! invocation from `use_ability` after the primary commits) so secondary
//! calls don't recursively fan out.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{AbilityDef, EffectDef};

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;
use super::super::damage_apply::apply_damage_to_target;

use super::geometry::{collect_cone_targets, is_cone_effect};

/// Dispatch every cone effect on `ability_def` against secondary targets
/// found around `(attacker, primary_target)`. Each secondary takes a
/// fresh `effect_seq` (so per-target wire packets remain correlatable)
/// and `needs_ammo_stat_send = false` (the primary call already flushed
/// the bandolier).
///
/// Returns the entity ids of every NPC that **died** during the cone
/// fan-out so the caller can fire `entity_death` events for kill-count
/// missions.
pub async fn fan_out_cone_effects(
    entity_id: u32,
    primary_target_id: u32,
    ability_id: i32,
    ability_def: &Option<AbilityDef>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Vec<u32> {
    let Some(def) = ability_def else {
        return Vec::new();
    };

    // Per-effect cone specs — each cone effect carries its own length +
    // half-angle AND its own NVP damage values. We CANNOT union targets
    // across effects and then apply the whole `ability_def`: that would
    // damage every secondary with every cone effect's NVP, even when
    // they only geometrically matched one cone. Instead we collect
    // (effect_id, targets) per cone effect and apply each effect
    // independently using a scoped-down `AbilityDef` carrying only that
    // one effect's id (so the NVP-pick loop inside `apply_damage_to_target`
    // sees only this cone's damage values).
    let cone_effect_ids: Vec<i32> = def
        .effect_ids
        .iter()
        .copied()
        .filter(|eid| space_mgr.effect_defs.get(eid).is_some_and(is_cone_effect))
        .collect();

    if cone_effect_ids.is_empty() {
        return Vec::new();
    }

    // Resolve geometry per cone effect.
    let mut per_effect_targets: Vec<(i32, Vec<u32>)> = Vec::with_capacity(cone_effect_ids.len());
    let mut union_for_death_snapshot: Vec<u32> = Vec::new();
    for &eid in &cone_effect_ids {
        let Some(effect) = space_mgr.effect_defs.get(&eid) else {
            continue;
        };
        let length = EffectDef::tcm_range_meters(&effect.tcm_param1);
        let half = EffectDef::tcm_half_angle_radians(&effect.tcm_param2);
        let targets = collect_cone_targets(space_mgr, entity_id, primary_target_id, length, half);
        for &t in &targets {
            if !union_for_death_snapshot.contains(&t) {
                union_for_death_snapshot.push(t);
            }
        }
        per_effect_targets.push((eid, targets));
    }

    if union_for_death_snapshot.is_empty() {
        tracing::debug!(
            entity_id,
            primary_target_id,
            ability_id,
            cone_count = cone_effect_ids.len(),
            "cone_aoe: no secondaries in any cone spec — primary-only damage"
        );
        return Vec::new();
    }

    tracing::info!(
        entity_id,
        primary_target_id,
        ability_id,
        cone_count = cone_effect_ids.len(),
        unique_secondary_count = union_for_death_snapshot.len(),
        "cone_aoe: fanning out to cone secondaries (per-effect)"
    );

    // Snapshot HEALTH for the UNION of all cone targets — we need this
    // pre-damage to detect alive→dead transitions afterward. Done once
    // (not per-effect) because an entity hit by two effects on the same
    // tick that was alive_before should still count as one kill.
    let alive_before: Vec<(u32, bool)> = union_for_death_snapshot
        .iter()
        .map(|&eid| {
            let alive = space_mgr.get_entity(eid).is_some_and(|e| {
                e.stats
                    .get(cimmeria_entity::stats::HEALTH)
                    .is_some_and(|s| s.cur > 0)
            });
            (eid, alive)
        })
        .collect();

    // Apply damage per-effect: for each cone effect, build a scoped
    // `AbilityDef` carrying ONLY that effect's id, then apply to that
    // effect's geometrically-matched targets. The scoped-down def causes
    // `apply_damage_to_target`'s NVP scan to read only this cone effect's
    // `HealthDamage` / `FocusDamage` / `script_name`, isolating per-effect
    // damage from cross-pollination across cone effects on the same ability.
    for (effect_id, targets) in &per_effect_targets {
        if targets.is_empty() {
            continue;
        }
        let scoped_def = AbilityDef {
            effect_ids: vec![*effect_id],
            ..def.clone()
        };
        let scoped_def_opt = Some(scoped_def);
        for &secondary_eid in targets {
            let secondary_seq = space_mgr
                .get_entity_mut(entity_id)
                .map(|e| e.abilities.next_effect_id())
                .unwrap_or(0);
            apply_damage_to_target(
                entity_id,
                secondary_eid,
                ability_id,
                &scoped_def_opt,
                secondary_seq as u32,
                // Primary already flushed; secondaries must not re-flush
                // or the wire-packet log shows N+1 ammo updates per fire.
                false,
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // Collect alive→dead transitions so the caller can fire entity_death
    // for each kill (kill-count missions on AoE secondaries).
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
