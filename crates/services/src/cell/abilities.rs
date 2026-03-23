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

/// Send an entity method call, routing to the entity's client if it's a player,
/// or broadcasting to all witnessing players if it's an NPC (ghost entity).
///
/// In BigWorld, method calls on ghost entities are forwarded to all players who
/// have that entity in their AoI. This is how players see NPC attack animations,
/// health changes, death states, etc.
pub(crate) async fn send_entity_method(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let is_player = space_mgr.get_entity(entity_id)
        .map_or(false, |e| e.is_player);

    if is_player {
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        }).await;
    } else {
        let witnesses = space_mgr.get_witnesses_of(entity_id);
        if witnesses.is_empty() {
            tracing::warn!(
                entity_id, method_index,
                "send_entity_method: NPC has no witnesses, method dropped"
            );
        }
        for witness_id in witnesses {
            tracing::debug!(
                witness_id, entity_id, method_index,
                "send_entity_method: routing NPC method to witness"
            );
            let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args: args.clone(),
            }).await;
        }
    }
}

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
    // ── Look up ability definition from DB (before mutable borrow) ──
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();

    // ── Validation (requires attacker entity) ──

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "useAbility: entity not found");
            return;
        }
    };

    // Check if entity knows this ability (skip for non-entity abilities like reload)
    if !entity.abilities.has_ability(ability_id) {
        tracing::debug!(entity_id, ability_id, "useAbility: entity does not have ability");
        return;
    }

    // Check cooldown
    if entity.abilities.is_on_cooldown(ability_id) {
        tracing::debug!(entity_id, ability_id, "useAbility: ability on cooldown");
        return;
    }

    // Check ammo for ranged abilities (players only — NPCs have infinite ammo)
    let required_ammo = ability_def.as_ref().map_or(0, |d| d.required_ammo);
    if required_ammo > 0 && entity.is_player && entity.current_ammo < required_ammo {
        tracing::debug!(entity_id, ability_id, current = entity.current_ammo, required = required_ammo, "useAbility: not enough ammo");
        return;
    }

    let cooldown_secs = ability_def.as_ref().map_or(2.0, |d| if d.cooldown > 0.0 { d.cooldown } else { 0.5 });
    let cooldown_duration = std::time::Duration::from_secs_f32(cooldown_secs);
    entity.abilities.start_ability_cooldown(ability_id, cooldown_duration);

    // Consume ammo (players only)
    if required_ammo > 0 && entity.is_player {
        entity.current_ammo -= required_ammo;
        tracing::debug!(entity_id, ability_id, ammo_remaining = entity.current_ammo, "useAbility: consumed ammo");
    }

    // Get effect sequence ID for this ability invocation
    let effect_seq = entity.abilities.next_effect_id();

    tracing::info!(
        entity_id, ability_id, target_id, cooldown_secs,
        ability_name = ability_def.as_ref().map_or("unknown", |d| &d.name),
        "useAbility: launched"
    );

    // ── Send cooldown timer to attacker ──

    let timer_args = serialize_timer_update(
        ability_id,
        TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        cooldown_secs,
        0.0, // TODO: bigWorldTimeComplete = gameTime + cooldown
    );

    send_entity_method(entity_id, 12, timer_args, tx, space_mgr).await;

    // ── Send attack animation (onSequence) to attacker + witnesses ──
    // The event_set_id from the ability def determines which kismet animation plays.
    if let Some(event_set_id) = ability_def.as_ref().and_then(|d| d.event_set_id) {
        let mut seq_args = Vec::with_capacity(28);
        seq_args.extend_from_slice(&event_set_id.to_le_bytes());       // KismetEventSetSeqID
        seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
        seq_args.extend_from_slice(&target_id.to_le_bytes());         // TargetID
        seq_args.push(1);                                               // PrimaryTarget = true
        seq_args.extend_from_slice(&0.0f32.to_le_bytes());            // ImpactTime
        seq_args.extend_from_slice(&0u32.to_le_bytes());               // NameValuePairs array count = 0
        seq_args.push(0);                                               // ViewType = KISMET_VIEW_Witness
        seq_args.extend_from_slice(&(effect_seq as i32).to_le_bytes()); // InstanceId
        send_entity_method(entity_id, 1, seq_args, tx, space_mgr).await; // 1 = onSequence
    }

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

    // Look up damage values from the ability's effect NVPs.
    let (health_base_damage, focus_base_damage) = if let Some(ref def) = ability_def {
        let mut h_dmg = 0i32;
        let mut f_dmg = 0i32;
        for &eid in &def.effect_ids {
            if let Some(effect) = space_mgr.effect_defs.get(&eid) {
                let hd = effect.param_i32("HealthDamage");
                let fd = effect.param_i32("FocusDamage");
                if hd > 0 { h_dmg = hd; }
                if fd > 0 { f_dmg = fd; }
            }
        }
        (if h_dmg > 0 { h_dmg } else { 15 }, f_dmg)
    } else {
        (15, 0)
    };

    // Apply health damage to target
    let target = match space_mgr.get_entity_mut(target_eid) {
        Some(e) => e,
        None => return,
    };

    let (effect_results, _total_health_damage) = combat::calculate_damage(
        &qr_result, health_base_damage, DT_PHYSICAL, HEALTH,
        &attacker_stats, &mut target.stats,
    );

    // Apply focus damage if present
    if focus_base_damage > 0 {
        let _ = combat::calculate_damage(
            &qr_result, focus_base_damage, DT_PHYSICAL, cimmeria_entity::stats::FOCUS,
            &attacker_stats, &mut target.stats,
        );
    }

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
    send_entity_method(entity_id, 14, effect_args.clone(), tx, space_mgr).await;

    // onEffectResults to target (so they see incoming damage)
    send_entity_method(target_eid, 14, effect_args, tx, space_mgr).await;

    // ── Send stat updates ──

    // onStatUpdate to target (their health bar changes)
    send_entity_method(target_eid, 20, target_stat_update, tx, space_mgr).await;

    // ── Send state field update if target died ──

    if target_died {
        let mut state_args = Vec::with_capacity(4);
        state_args.extend_from_slice(&state_field.to_le_bytes());
        send_entity_method(target_eid, 19, state_args, tx, space_mgr).await;

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
            }
        }
    }

    // Generate threat on surviving NPCs so they aggro back
    if !target_died {
        combat::generate_threat(space_mgr, entity_id, target_eid, _total_health_damage as f32);
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
