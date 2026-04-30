//! The main `useAbility(abilityId, targetId)` flow.
//!
//! Validates the call, consumes ammo, starts the cooldown, fires animations,
//! resolves damage, and pushes the resulting wire messages (timer, sequence,
//! state-field, effect-results, stat-update, death) back to the relevant
//! clients. Splits cleanly into a validation phase, a consume/fire phase,
//! and a target-resolution phase — kept as one function because the borrow
//! ordering between immutable and mutable space_manager access is delicate
//! and a hand-rolled split here would just trade lines for `&mut` plumbing.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    TIMER_ABILITY_COOLDOWN, DT_PHYSICAL,
    serialize_timer_update, serialize_effect_results,
};
use cimmeria_entity::stats::HEALTH;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

use super::loot_drop::{generate_loot_on_death, kill_xp};
use super::messaging::{flush_attacker_ammo_stat, send_entity_method};
use super::rng::pseudo_random;

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

    // ── Validation (immutable checks first to avoid borrow conflicts) ──

    // Pre-checks with immutable borrows
    let mut out_of_range = false;
    {
        let entity = match space_mgr.get_entity(entity_id) {
            Some(e) => e,
            None => {
                tracing::warn!(entity_id, "useAbility: entity not found");
                return;
            }
        };

        if combat::is_dead_state(entity.state_field) {
            return;
        }
        if !entity.abilities.has_ability(ability_id) {
            tracing::debug!(entity_id, ability_id, "useAbility: entity does not have ability");
            return;
        }
        if entity.abilities.is_on_cooldown(ability_id) {
            tracing::debug!(entity_id, ability_id, "useAbility: ability on cooldown");
            return;
        }

        // Range + target validation
        if target_id > 0 {
            if let Some(target) = space_mgr.get_entity(target_id as u32) {
                // Don't attack dead targets
                if combat::is_dead_state(target.state_field) {
                    tracing::debug!(entity_id, ability_id, target_id, "useAbility: target is dead");
                    return;
                }
                // Range check
                let max_range = ability_def.as_ref().map_or(30.0, |d| if d.max_range > 0 { d.max_range as f32 } else { 30.0 });
                let dist = entity.position.distance_to(&target.position);
                if dist > max_range {
                    tracing::debug!(entity_id, ability_id, distance = dist, max_range, "useAbility: target out of range");
                    out_of_range = true;
                }
            }
        }
    }

    if out_of_range {
        // Send onErrorCode to player: ERRORCODE_SYSTEM_Ability=0, CONDITION_FEEDBACK_OutsideWeaponRange=42
        let mut err_args = Vec::with_capacity(7);
        err_args.push(0u8);                                     // SystemID
        err_args.extend_from_slice(&ability_id.to_le_bytes());  // InstanceID
        err_args.extend_from_slice(&42u16.to_le_bytes());       // ErrorCodeID
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: 121, // ON_ERROR_CODE
            args: err_args,
        }).await;
        return;
    }

    // Mutable borrow for state changes
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    // Check ammo for ranged abilities (players only — NPCs have infinite ammo).
    // Stage C: read through the bandolier helpers; Stage B's reload tick is the
    // sole refill path, so the eager promotion that used to live here is gone.
    let required_ammo = ability_def.as_ref().map_or(0, |d| d.required_ammo);

    // Block firing while a reload is in flight — checked as `is_some()`, not
    // `now < deadline`. Between the deadline elapsing and the next 100 ms
    // `reload_completion_tick`, the warmup is "over" by clock but the magazine
    // hasn't been refilled yet; allowing fire in that window would decrement
    // against pre-refill ammo and then be silently overwritten by the tick,
    // effectively granting free ammo. The tick is the sole authority that
    // clears `reload_complete_at`, so we gate on its presence.
    if required_ammo > 0
        && entity.is_player
        && entity.reload_complete_at.is_some()
    {
        tracing::debug!(entity_id, ability_id, "useAbility: reload in progress, blocking fire");
        return;
    }

    let current_ammo = entity.active_ammo();
    if required_ammo > 0 && entity.is_player && current_ammo < required_ammo {
        tracing::debug!(entity_id, ability_id, current = current_ammo, required = required_ammo, "useAbility: not enough ammo");
        return;
    }

    let cooldown_secs = ability_def.as_ref().map_or(2.0, |d| if d.cooldown > 0.0 { d.cooldown } else { 0.5 });
    let cooldown_duration = std::time::Duration::from_secs_f32(cooldown_secs);
    entity.abilities.start_ability_cooldown(ability_id, cooldown_duration);

    // Consume ammo (players only). Routes through `set_slot_ammo` so the
    // AmmoSlot{N} stat updates and the slot is marked dirty for batched
    // persistence (drained on reload completion / slot swap / ammo change /
    // logout — Stage D wires the swap and logout flushes).
    let mut needs_ammo_stat_send = false;
    if required_ammo > 0 && entity.is_player {
        let new_ammo = entity.active_ammo() - required_ammo;
        let slot = entity.active_bandolier_slot;
        entity.set_slot_ammo(slot, new_ammo);
        needs_ammo_stat_send = true;
        tracing::debug!(entity_id, ability_id, ammo_remaining = entity.active_ammo(), "useAbility: consumed ammo");
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

    // ── Set combat state on the attacker ──
    // BSF_InCombat (bit 3): The client's SeqEvent_CombatStateChanged fires when
    // this changes, transitioning the animation state machine.
    // BSF_Holster (bit 8): When set, weapon is holstered. Must be CLEARED for
    // combat — USGWAnim_BlendByWeapon uses active weapon to select animations.
    // Reference: SGWBeing.py:751-754, SGWMob.py:162
    {
        const BSF_IN_COMBAT: u32 = 1 << 3;
        const BSF_HOLSTER: u32 = 1 << 8;
        let entity = space_mgr.get_entity_mut(entity_id);
        if let Some(e) = entity {
            let old_state = e.state_field;
            e.state_field |= BSF_IN_COMBAT;   // Enter combat
            e.state_field &= !BSF_HOLSTER;     // Unholster weapon
            if e.state_field != old_state {
                let new_state = e.state_field;
                send_entity_method(entity_id, 19, new_state.to_le_bytes().to_vec(), tx, space_mgr).await;
            }
        }
    }

    // ── Send attack animation (onSequence) to attacker + witnesses ──
    // Look up the correct sequence_id from the event set. The client expects
    // the sequence_id from resources.sequences, NOT the event_set_id.
    // Reference: AbilityManager.py — self.manager.playSequence(beginSeq.seqId, ...)
    if let Some(event_set_id) = ability_def.as_ref().and_then(|d| d.event_set_id) {
        use super::super::spawner::{EVENT_ABILITY_BEGIN, EVENT_ABILITY_END};

        // Send Ability_Begin (event_id 1000) if the ability has a warmup phase
        let warmup = ability_def.as_ref().map_or(0.0, |d| d.warmup);
        if warmup > 0.0 {
            if let Some(&begin_seq_id) = space_mgr.sequence_map.get(&(event_set_id, EVENT_ABILITY_BEGIN)) {
                let mut seq_args = Vec::with_capacity(28);
                seq_args.extend_from_slice(&begin_seq_id.to_le_bytes());       // KismetEventSetSeqID (sequence_id)
                seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
                seq_args.extend_from_slice(&target_id.to_le_bytes());          // TargetID
                seq_args.push(1);                                               // PrimaryTarget = true
                seq_args.extend_from_slice(&0.0f32.to_le_bytes());            // ImpactTime
                seq_args.extend_from_slice(&0u32.to_le_bytes());               // NameValuePairs array count = 0
                seq_args.push(0);                                               // ViewType = KISMET_VIEW_Witness
                seq_args.extend_from_slice(&(effect_seq as i32).to_le_bytes()); // InstanceId
                send_entity_method(entity_id, 1, seq_args, tx, space_mgr).await; // 1 = onSequence
            }
        }

        // Send Ability_End (event_id 1001) — the main ability fire animation
        if let Some(&end_seq_id) = space_mgr.sequence_map.get(&(event_set_id, EVENT_ABILITY_END)) {
            let mut seq_args = Vec::with_capacity(28);
            seq_args.extend_from_slice(&end_seq_id.to_le_bytes());         // KismetEventSetSeqID (sequence_id)
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
            seq_args.extend_from_slice(&target_id.to_le_bytes());          // TargetID
            seq_args.push(1);                                               // PrimaryTarget = true
            seq_args.extend_from_slice(&0.0f32.to_le_bytes());            // ImpactTime
            seq_args.extend_from_slice(&0u32.to_le_bytes());               // NameValuePairs array count = 0
            seq_args.push(0);                                               // ViewType = KISMET_VIEW_Witness
            seq_args.extend_from_slice(&(effect_seq as i32).to_le_bytes()); // InstanceId
            send_entity_method(entity_id, 1, seq_args, tx, space_mgr).await; // 1 = onSequence
        } else {
            tracing::debug!(
                entity_id, ability_id, event_set_id,
                "onSequence: no Ability_End sequence found for event_set"
            );
        }
    }

    // ── Combat resolution (if target specified) ──

    if target_id <= 0 {
        // Self-buff or no-target ability — skip damage but still flush any
        // dirty ammo stat (e.g. ground-targeted ability that consumed ammo
        // without picking up a target via auto-aim).
        if needs_ammo_stat_send {
            flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
        }
        return;
    }

    let target_eid = target_id as u32;

    // We need both attacker and target stats. Since we can't borrow two
    // entities mutably at once, we snapshot the attacker stats first.
    let attacker_stats = match space_mgr.get_entity(entity_id) {
        Some(e) => e.stats.clone(),
        None => {
            // Defensive: entity vanished after the consume mutation. Flush
            // before exiting so the client still sees the ammo decrement.
            if needs_ammo_stat_send {
                flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
            }
            return;
        }
    };

    // Calculate QR
    let target = match space_mgr.get_entity(target_eid) {
        Some(e) => e,
        None => {
            tracing::debug!(target_id, "useAbility: target not found");
            // Flush ammo decrement even when target lookup fails — the shot
            // still left the chamber from the player's perspective.
            if needs_ammo_stat_send {
                flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
            }
            return;
        }
    };
    // Use the ability's actual ranged flag for the QR calculation. Defaults
    // to false when the AbilityDef is missing (unknown ability falls back to
    // a generic melee swing).
    let ability_is_ranged = ability_def.as_ref().map(|d| d.is_ranged).unwrap_or(false);
    let qr = combat::calculate_qr(&attacker_stats, &target.stats, ability_is_ranged);

    // Generate random value for this hit (seeded from ability invocation)
    // For now use a deterministic hash to be reproducible in tests.
    // In production this would use a proper PRNG.
    let random_value = pseudo_random(entity_id, ability_id, effect_seq as u32);
    let qr_result = combat::calculate_result(qr, random_value);

    // Look up damage values from the ability's effect NVPs. When the
    // ability is known but exposes no positive HealthDamage (e.g. focus
    // drains, heals, buff-only abilities) we keep health_base_damage at 0
    // so the ability doesn't accidentally read as a 15-HP physical hit.
    // The 15-HP fallback is reserved for the unknown-ability case.
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
        (h_dmg, f_dmg)
    } else {
        (15, 0)
    };
    // Temp: 2x player damage so players can kill NPCs before dying
    let is_player_attacker = space_mgr.get_entity(entity_id).map_or(false, |e| e.is_player);
    let health_base_damage = if is_player_attacker { health_base_damage * 2 } else { health_base_damage };

    // Apply health damage to target
    let target = match space_mgr.get_entity_mut(target_eid) {
        Some(e) => e,
        None => {
            // Flush ammo decrement before exiting — the shot was fired even
            // if the target was removed between the immutable lookup above
            // and the mutable lookup here.
            if needs_ammo_stat_send {
                flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
            }
            return;
        }
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

    // Check if target died — use entity's state_field so we preserve other flags
    let target_died = target.stats.get(HEALTH).map_or(false, |s| s.cur <= 0);
    if target_died {
        combat::set_dead_state(&mut target.state_field);
        target.state_field |= 1 << 6; // BSF_MovementLock — prevent movement while dead
        // Transition NPC AI to Dead so it stops fighting and moving
        if !target.is_player {
            target.ai_state = cimmeria_entity::cell_entity::AiState::Dead;
            target.threat_list.clear();
            target.nav_path.clear();
            target.velocity = [0.0; 3]; // Stop movement interpolation
            target.interaction_type_flags = 0; // Clear attackable flags; loot generation below may re-set
        }
        tracing::info!(
            attacker = entity_id, target = target_eid,
            ability_id, is_npc = !target.is_player,
            "Target killed!"
        );
    }
    let target_state = target.state_field;

    // Serialize dirty stats for the target
    let target_stat_update = target.stats.serialize_dirty();
    let _target_public_stat_update = target.stats.serialize_dirty_public();
    target.stats.clear_dirty();

    // ── Send effect results ──

    // onEffectResults — send to both attacker and target, but avoid double-sending.
    // If attacker is a player and target is an NPC, the witness routing on the NPC
    // already reaches the player. So only send to the attacker directly + NPC witnesses.
    let effect_args = serialize_effect_results(
        entity_id as i32,  // source
        ability_id,
        effect_seq as i32, // effect ID (using sequence as stub)
        target_eid as i32, // target
        qr_result.result_code,
        &effect_results,
    );

    let attacker_is_player = space_mgr.get_entity(entity_id).map_or(false, |e| e.is_player);
    let target_is_player = space_mgr.get_entity(target_eid).map_or(false, |e| e.is_player);

    // Send to attacker
    send_entity_method(entity_id, 14, effect_args.clone(), tx, space_mgr).await;

    // Only send to target if the target is a player (so they see incoming damage).
    // If target is an NPC, the attacker (as witness) already received it above via
    // send_entity_method routing.
    if target_is_player && !attacker_is_player {
        send_entity_method(target_eid, 14, effect_args, tx, space_mgr).await;
    }

    // ── Send stat updates ──

    // onStatUpdate to target (their health bar changes)
    send_entity_method(target_eid, 20, target_stat_update, tx, space_mgr).await;

    // onStatUpdate to attacker — drains AmmoSlot{N} dirty bits set by
    // `set_slot_ammo` on the consume path so the bandolier UI updates on every
    // shot. Skipped if the consume path didn't run (no ammo cost or NPC
    // attacker).
    if needs_ammo_stat_send {
        flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
    }

    // ── Send state field update if target died ──

    if target_died {
        let mut state_args = Vec::with_capacity(4);
        state_args.extend_from_slice(&target_state.to_le_bytes());
        send_entity_method(target_eid, 19, state_args, tx, space_mgr).await;

        // Clear the attacker's target so the targeting reticle disappears.
        // The reticle stays at the NPC's last standing position otherwise.
        if attacker_is_player {
            send_entity_method(
                entity_id, 16, // onTargetUpdate(INT32 targetId = 0)
                0i32.to_le_bytes().to_vec(),
                tx, space_mgr,
            ).await;
        }

        // Generate loot from the NPC's loot table, then update interaction type.
        // Reference: python/cell/SGWMob.py:onDead() + Lootable.generateLoot()
        if !target_is_player {
            generate_loot_on_death(target_eid, space_mgr);

            // Send InteractionType: INT_NormalLoot if loot was generated, 0 otherwise
            let interaction_flags = space_mgr.get_entity(target_eid)
                .map_or(0i64, |e| e.interaction_type_flags);
            send_entity_method(
                target_eid, 3,
                interaction_flags.to_le_bytes().to_vec(),
                tx, space_mgr,
            ).await;
        }

        // Send death animation via onSequence (Entity_Death = event_id 5001)
        // Look up the death sequence from the target's event set via sequence_map
        {
            const EVENT_ENTITY_DEATH: i32 = 5001;
            // Use event set 1025 (Mob) for NPCs, 1025 for players too (same death anim)
            let event_set_id = space_mgr.get_entity(target_eid)
                .and_then(|e| if e.is_player { Some(1025) } else { Some(1025) });
            if let Some(esid) = event_set_id {
                if let Some(&death_seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ENTITY_DEATH)) {
                    let mut seq_args = Vec::with_capacity(28);
                    seq_args.extend_from_slice(&death_seq_id.to_le_bytes());       // KismetEventSetSeqID
                    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // SourceID (dying entity)
                    seq_args.extend_from_slice(&(target_eid as i32).to_le_bytes()); // TargetID (also dying entity — NOT killer, or client plays death anim on killer)
                    seq_args.push(1);                                                // PrimaryTarget
                    seq_args.extend_from_slice(&0.0f32.to_le_bytes());             // ImpactTime
                    seq_args.extend_from_slice(&0u32.to_le_bytes());                // NameValuePairs count
                    seq_args.push(0);                                                // ViewType
                    seq_args.extend_from_slice(&0i32.to_le_bytes());                // InstanceId
                    send_entity_method(target_eid, 1, seq_args, tx, space_mgr).await; // onSequence
                }
            }
        }

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

        // Send onBeginAidWait to player so they see the Defeat Window
        // Reference: python/cell/SGWPlayer.py:1278 — self.client.onBeginAidWait(100, respawnerList)
        if target_is_player {
            // Look up respawners for the player's current world
            let world_name = space_mgr.get_entity_world_name(target_eid);
            let matching_respawners: Vec<_> = if let Some(ref wn) = world_name {
                space_mgr.respawners.iter()
                    .filter(|r| r.world_name == *wn)
                    .collect()
            } else {
                vec![]
            };

            let mut aid_args = Vec::with_capacity(64);
            // INT32: TimeToAid (seconds until auto-respawn)
            aid_args.extend_from_slice(&30i32.to_le_bytes());

            if matching_respawners.is_empty() {
                // Fallback: single entry with chardef spawn position
                aid_args.extend_from_slice(&1u32.to_le_bytes()); // array count
                aid_args.extend_from_slice(&0i32.to_le_bytes()); // respawnerID = 0 (default)
                crate::mercury::write_wstring(&mut aid_args, "Respawn Point");
            } else {
                aid_args.extend_from_slice(&(matching_respawners.len() as u32).to_le_bytes());
                for resp in &matching_respawners {
                    aid_args.extend_from_slice(&resp.respawner_id.to_le_bytes());
                    crate::mercury::write_wstring(&mut aid_args, &resp.name);
                }
            }

            send_entity_method(
                target_eid,
                crate::mercury::method_idx::ON_BEGIN_AID_WAIT,
                aid_args,
                tx,
                space_mgr,
            ).await;
            tracing::info!(
                target = target_eid,
                world = ?world_name,
                respawner_count = if matching_respawners.is_empty() { 1 } else { matching_respawners.len() },
                "Sent onBeginAidWait (Defeat Window)"
            );
        }
    }

    // Generate threat on surviving NPCs so they aggro back
    if !target_died {
        combat::generate_threat(space_mgr, entity_id, target_eid, _total_health_damage as f32);
    }
}
