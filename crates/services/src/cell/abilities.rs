//! Ability invocation handler for the CellService.
//!
//! Processes `useAbility` calls from the client: validates the ability exists
//! in the entity's known list, checks cooldowns, starts warmup/cooldown timers,
//! resolves damage against the target, and sends results to the client.
//!
//! Reference: `python/cell/AbilityManager.py:1004-1056`

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    TIMER_ABILITY_COOLDOWN, DT_PHYSICAL,
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
/// 4. Start cooldown timer
/// 5. Send `onTimerUpdate` to client
/// 6. If target exists, resolve combat damage
/// 7. Send `onEffectResults` to attacker's client and witnesses
/// 8. Send `onStatUpdate` to target if stats changed
/// 9. Check for death and send `onStateFieldUpdate` if target died
pub async fn handle_use_ability(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
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

    // Apply a default cooldown (2 seconds) since we don't have full ability
    // definitions loaded from DB yet. Real cooldown comes from AbilityDef.cooldown.
    let cooldown_secs = 2.0f32;
    let cooldown_duration = std::time::Duration::from_secs_f32(cooldown_secs);
    entity.abilities.start_ability_cooldown(ability_id, cooldown_duration);

    // Get effect sequence ID for this ability invocation
    let effect_seq = entity.abilities.next_effect_id();

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

    // ── Combat resolution (if target specified) ──

    if target_id <= 0 {
        return; // Self-buff or no-target ability — skip damage
    }

    let target_eid = target_id as u32;

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
    let qr = combat::calculate_qr(&attacker_stats, &target.stats, true);

    // Generate random value for this hit (seeded from ability invocation)
    // For now use a deterministic hash to be reproducible in tests.
    // In production this would use a proper PRNG.
    let random_value = pseudo_random(entity_id, ability_id, effect_seq as u32);
    let qr_result = combat::calculate_result(qr, random_value);

    // Default base damage (stub — real value comes from AbilityDef.effects[].params)
    let base_damage: i32 = 50;

    // Apply damage to target
    let target = match space_mgr.get_entity_mut(target_eid) {
        Some(e) => e,
        None => return,
    };

    let (effect_results, _total_damage) = combat::calculate_damage(
        &qr_result, base_damage, DT_PHYSICAL, HEALTH,
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

    // ── Send state field update if target died ──

    if target_died {
        let mut state_args = Vec::with_capacity(4);
        state_args.extend_from_slice(&state_field.to_le_bytes());
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id: target_eid,
            method_index: 19, // onStateFieldUpdate (SGWBeing interface, flat index 19)
            args: state_args,
        }).await;
    }
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
