//! Ability invocation handler for the CellService.
//!
//! Processes `useAbility` calls from the client: validates the ability exists
//! in the entity's known list, checks cooldowns, starts warmup/cooldown timers,
//! resolves damage against the target, and sends results to the client.
//!
//! Reference: `python/cell/AbilityManager.py:1004-1056`

use tokio::sync::mpsc;

use std::sync::Arc;

use cimmeria_entity::abilities::{
    AbilityRegistry, TIMER_ABILITY_COOLDOWN, DT_PHYSICAL, DT_UNTYPED,
    ClientEffectResult, RC_HIT, SRC_NONE,
    serialize_timer_update, serialize_effect_results,
};
use cimmeria_entity::stats::HEALTH;

use super::combat;
use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

/// Handle a `useAbility(abilityId, targetId)` cell method call.
///
/// Flow:
/// 1. Look up entity in space manager
/// 2. Check entity has the ability
/// 3. Check ability not on cooldown
/// 4. Start cooldown timer (from AbilityDef or default)
/// 5. Send `onTimerUpdate` to client
/// 6. If target exists, resolve combat damage using effect definitions
/// 7. Send `onEffectResults` to attacker's client and witnesses
/// 8. Send `onStatUpdate` to target if stats changed
/// 9. Check for death and send `onStateFieldUpdate` if target died
pub async fn handle_use_ability(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    ability_registry: &Arc<AbilityRegistry>,
    loot_cache: &Arc<super::loot::LootCache>,
    effect_mgr: &mut super::effects::EffectManager,
) {
    // ── Validation (requires attacker entity) ──

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "useAbility: entity not found");
            return;
        }
    };

    // Check if entity knows this ability
    if !entity.abilities.has_ability(ability_id) {
        tracing::debug!(entity_id, ability_id, "useAbility: entity does not have ability");
        return;
    }

    // Check cooldown
    if entity.abilities.is_on_cooldown(ability_id) {
        tracing::debug!(entity_id, ability_id, "useAbility: ability on cooldown");
        return;
    }

    // Look up ability definition for real cooldown, or fall back to default
    let ability_def = ability_registry.get_ability(ability_id);
    let cooldown_secs = ability_def.map_or(2.0f32, |a| a.cooldown);
    let cooldown_duration = std::time::Duration::from_secs_f32(cooldown_secs);

    // Start cooldowns (ability + moniker groups if def available)
    if let Some(def) = ability_def {
        entity.abilities.start_cooldowns(def, cooldown_secs);
    } else {
        entity.abilities.start_ability_cooldown(ability_id, cooldown_duration);
    }

    // Get effect sequence ID for this ability invocation
    let effect_seq = entity.abilities.next_effect_id();

    // Track this ability for auto-cycle repeat
    if entity.abilities.auto_cycle {
        entity.abilities.auto_cycle_ability_id = Some(ability_id);
    }

    tracing::info!(entity_id, ability_id, target_id, cooldown_secs, "useAbility: launched");

    // ── Send cooldown timer to attacker ──

    let timer_args = serialize_timer_update(
        ability_id,
        TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        cooldown_secs,
        0.0, // TODO: bigWorldTimeComplete = gameTime + cooldown
    );

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 12, // onTimerUpdate (SGWBeing interface, flat index 12)
        args: timer_args,
    }).await;

    // ── Heal resolution (self-heal when target_id <= 0) ──

    if target_id <= 0 {
        // Check if this ability has any heal effects — if so, apply to caster.
        // Non-heal self-buff abilities (e.g., stat buffs) are handled by the
        // duration effect system below and don't need special treatment here.
        let effects = ability_registry.get_ability_effects(ability_id);
        let total_heal: i32 = effects.iter().map(|e| e.heal_amount()).sum();

        if total_heal > 0 {
            apply_heal(
                entity_id, entity_id, ability_id, effect_seq,
                total_heal, HEALTH, tx, space_mgr,
            ).await;
        }

        // Still run duration effects for self-targeted abilities
        apply_duration_effects(
            entity_id, entity_id, ability_id,
            ability_registry, effect_mgr, tx, space_mgr,
        ).await;

        return;
    }

    let target_eid = target_id as u32;

    // ── Friendly heal (target is a player) ──
    // If the target is a friendly player, check for heal effects instead of
    // running the damage pipeline. Heals skip QR entirely — they always land.
    let target_is_player = space_mgr.get_entity(target_eid)
        .map_or(false, |e| e.is_player);

    if target_is_player {
        let effects = ability_registry.get_ability_effects(ability_id);
        let total_heal: i32 = effects.iter().map(|e| e.heal_amount()).sum();

        if total_heal > 0 {
            apply_heal(
                entity_id, target_eid, ability_id, effect_seq,
                total_heal, HEALTH, tx, space_mgr,
            ).await;

            // Duration heal-over-time effects on the friendly target
            apply_duration_effects(
                entity_id, target_eid, ability_id,
                ability_registry, effect_mgr, tx, space_mgr,
            ).await;

            return;
        }
        // If a friendly-targeted ability has no heal effects, fall through
        // to damage path (e.g., friendly-fire or debuff abilities).
    }

    // ── Combat resolution (hostile target) ──

    // We need both attacker and target stats. Since we can't borrow two
    // entities mutably at once, we snapshot the attacker stats first.
    let attacker_stats = match space_mgr.get_entity(entity_id) {
        Some(e) => e.stats.clone(),
        None => return,
    };

    // Calculate QR
    let target = match space_mgr.get_entity(target_eid) {
        Some(e) => e,
        None => {
            tracing::debug!(target_id, "useAbility: target not found");
            return;
        }
    };
    let is_ranged = ability_def.map_or(true, |a| a.is_ranged);
    let qr = combat::calculate_qr(&attacker_stats, &target.stats, is_ranged);

    // Generate random value for this hit (seeded from ability invocation)
    // For now use a deterministic hash to be reproducible in tests.
    // In production this would use a proper PRNG.
    let random_value = pseudo_random(entity_id, ability_id, effect_seq as u32);
    let qr_result = combat::calculate_result(qr, random_value);

    // Get base damage and damage type from the ability's first damage effect,
    // or fall back to defaults if no effect definitions are loaded.
    let (base_damage, damage_type, stat_id) = {
        let effects = ability_registry.get_ability_effects(ability_id);
        if let Some(effect) = effects.first() {
            (effect.base_damage().max(1), effect.damage_type(), effect.stat_id())
        } else {
            (50i32, DT_PHYSICAL, HEALTH) // fallback when no DB data
        }
    };

    // Apply damage to target
    let target = match space_mgr.get_entity_mut(target_eid) {
        Some(e) => e,
        None => return,
    };

    let (effect_results, _total_damage) = combat::calculate_damage(
        &qr_result, base_damage, damage_type, stat_id,
        &attacker_stats, &mut target.stats,
    );

    // Check if target died
    let target_died = target.stats.get(HEALTH).map_or(false, |s| s.cur <= 0);
    let mut state_field = 0u32;
    if target_died {
        combat::set_dead_state(&mut state_field);
        tracing::info!(
            attacker = entity_id, target = target_eid,
            ability_id, "Target killed!"
        );
    }

    // Serialize dirty stats for the target
    let target_stat_update = target.stats.serialize_dirty();
    let _target_public_stat_update = target.stats.serialize_dirty_public();
    target.stats.clear_dirty();

    // ── Send effect results ──

    // onEffectResults to attacker (so they see damage numbers)
    let effect_args = serialize_effect_results(
        entity_id as i32,  // source
        ability_id,
        effect_seq as i32, // effect ID (using sequence as stub)
        target_eid as i32, // target
        qr_result.result_code,
        &effect_results,
    );
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 14, // onEffectResults (SGWBeing interface, flat index 14)
        args: effect_args.clone(),
    }).await;

    // onEffectResults to target (so they see incoming damage)
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: target_eid,
        method_index: 14,
        args: effect_args,
    }).await;

    // ── Send stat updates ──

    // onStatUpdate to target (their health bar changes)
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: target_eid,
        method_index: 20, // onStatUpdate (SGWCombatant interface, flat index 20)
        args: target_stat_update,
    }).await;

    // ── Apply duration effects (buffs/debuffs/HoTs) from ability ──
    if !target_died {
        apply_duration_effects(
            entity_id, target_eid, ability_id,
            ability_registry, effect_mgr, tx, space_mgr,
        ).await;
    }

    // ── Send state field update if target died ──

    if target_died {
        let mut state_args = Vec::with_capacity(4);
        state_args.extend_from_slice(&state_field.to_le_bytes());
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: target_eid,
            method_index: 19, // onStateFieldUpdate (SGWBeing interface, flat index 19)
            args: state_args,
        }).await;

        // Grant XP to the attacker if the target is a non-player entity
        if let Some(target) = space_mgr.get_entity(target_eid) {
            if !target.is_player {
                let xp = kill_xp(target.level);
                tracing::info!(
                    attacker = entity_id, target = target_eid,
                    mob_level = target.level, xp, "Granting kill XP"
                );
                let _ = tx.send(CellToBaseMsg::GrantXP {
                    entity_id,
                    xp_amount: xp,
                }).await;

                // Generate and send loot drops
                super::loot::generate_and_send_loot(
                    entity_id, target_eid, loot_cache, tx, space_mgr,
                ).await;

                // Queue NPC for respawn (30 seconds)
                space_mgr.queue_npc_respawn(target_eid, 30.0);

                // Remove dead NPC from space after a short delay
                // (so death animation can play on witnesses)
                space_mgr.destroy_entity(target_eid);
            }
        }
    }
}

/// Apply an instant heal to a target entity. Sends `onEffectResults` (green
/// numbers) to both caster and target, and `onStatUpdate` to the target.
///
/// Heals always land (RC_HIT) — there is no QR roll for friendly abilities.
async fn apply_heal(
    caster_id: u32,
    target_id: u32,
    ability_id: i32,
    effect_seq: i32,
    heal_amount: i32,
    stat_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(target_id) {
        Some(e) => e,
        None => {
            tracing::warn!(target_id, "apply_heal: target entity not found");
            return;
        }
    };

    // Apply healing (positive delta, clamped to max by Stat::change)
    let actual_heal = match entity.stats.get_mut(stat_id) {
        Some(stat) => stat.change(heal_amount),
        None => {
            tracing::warn!(target_id, stat_id, "apply_heal: target missing stat");
            return;
        }
    };
    if actual_heal == 0 {
        tracing::debug!(target_id, heal_amount, "apply_heal: target already at full");
        // Still send effect results so the client sees "0" heal feedback
    }

    tracing::info!(
        caster = caster_id, target = target_id,
        ability_id, heal_amount, actual_heal,
        "Heal applied"
    );

    let stat_update = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();

    // Build effect results with positive delta (client renders green numbers)
    let effect_results = vec![ClientEffectResult {
        stat_id: stat_id as i8,
        delta: actual_heal,
        damage_code: DT_UNTYPED, // heals have no damage type
        stat_result_code: SRC_NONE,
    }];

    let effect_args = serialize_effect_results(
        caster_id as i32,
        ability_id,
        effect_seq as i32,
        target_id as i32,
        RC_HIT,
        &effect_results,
    );

    // onEffectResults to caster (so they see heal numbers on target)
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: caster_id,
        method_index: 14,
        args: effect_args.clone(),
    }).await;

    // onEffectResults to target (so they see incoming heal)
    if target_id != caster_id {
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: target_id,
            method_index: 14,
            args: effect_args,
        }).await;
    }

    // onStatUpdate to target
    if !stat_update.is_empty() {
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: target_id,
            method_index: 20,
            args: stat_update,
        }).await;
    }
}

/// Apply duration effects (HoTs, DoTs, buffs, debuffs) from an ability's
/// effect definitions. Extracted so it can be used by both heal and damage paths.
async fn apply_duration_effects(
    caster_id: u32,
    target_id: u32,
    ability_id: i32,
    ability_registry: &Arc<AbilityRegistry>,
    effect_mgr: &mut super::effects::EffectManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let effects = ability_registry.get_ability_effects(ability_id);
    for effect_def in &effects {
        // Duration effects have pulse_duration > 0
        if effect_def.pulse_duration > 0.0 {
            let heal = effect_def.heal_amount();
            let dmg = effect_def.base_damage();
            let sid = effect_def.stat_id();

            // Build stat modifiers from the effect's NVPs
            let mut modifiers = Vec::new();
            if heal > 0 {
                modifiers.push(super::effects::StatModifier {
                    stat_id: sid,
                    delta: heal, // positive = buff/heal
                });
            } else if dmg > 0 {
                modifiers.push(super::effects::StatModifier {
                    stat_id: sid,
                    delta: -dmg, // negative = debuff/DoT
                });
            }

            if !modifiers.is_empty() {
                effect_mgr.apply_effect(
                    target_id, caster_id, ability_id,
                    modifiers, effect_def.pulse_duration,
                    tx, space_mgr,
                ).await;
            }
        }
    }
}

/// Calculate XP reward for killing a mob of the given level.
/// Formula: 10 * mob_level.
fn kill_xp(mob_level: u32) -> u64 {
    10 * mob_level as u64
}

/// Generate a pseudo-random value in [0.0, 1.0) from entity/ability/sequence.
///
/// This is a simple hash-based PRNG for reproducible combat results.
/// A proper PRNG (e.g., `rand` crate) should replace this in production.
fn pseudo_random(entity_id: u32, ability_id: i32, sequence: u32) -> f64 {
    let mut h = entity_id.wrapping_mul(2654435761);
    h ^= (ability_id as u32).wrapping_mul(2246822519);
    h ^= sequence.wrapping_mul(3266489917);
    h = h.wrapping_mul(668265263);
    h ^= h >> 15;
    (h as f64) / (u32::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_update_has_correct_format() {
        let data = serialize_timer_update(597, TIMER_ABILITY_COOLDOWN, 100, 5.0, 12345.0);
        assert_eq!(data.len(), 21);
        let id = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(id, 597);
        assert_eq!(data[4], 2); // TIMER_ABILITY_COOLDOWN
        let source = i32::from_le_bytes([data[5], data[6], data[7], data[8]]);
        assert_eq!(source, 100);
    }

    #[test]
    fn pseudo_random_is_deterministic() {
        let v1 = pseudo_random(1, 597, 0);
        let v2 = pseudo_random(1, 597, 0);
        assert_eq!(v1, v2);
    }

    #[test]
    fn pseudo_random_varies_with_inputs() {
        let v1 = pseudo_random(1, 597, 0);
        let v2 = pseudo_random(2, 597, 0);
        let v3 = pseudo_random(1, 598, 0);
        let v4 = pseudo_random(1, 597, 1);
        assert_ne!(v1, v2);
        assert_ne!(v1, v3);
        assert_ne!(v1, v4);
    }

    #[test]
    fn pseudo_random_in_range() {
        for i in 0..100 {
            let v = pseudo_random(i, i as i32 * 7, i * 13);
            assert!(v >= 0.0 && v < 1.0, "value out of range: {v}");
        }
    }
}
