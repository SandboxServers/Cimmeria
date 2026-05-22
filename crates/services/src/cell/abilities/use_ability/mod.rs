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

use cimmeria_entity::abilities::{serialize_timer_update, TIMER_ABILITY_COOLDOWN};

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

use super::messaging::{flush_attacker_ammo_stat, send_entity_method};

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
///
/// Returns `true` when the cast committed (validation passed and the
/// cooldown/ammo consume took effect — which is also when the target's
/// damage resolution and wire packets fired). Returns `false` when any
/// pre-consume guard rejected the call (entity missing/dead, no
/// ability, on cooldown, reload in flight, no ammo, or out-of-range
/// for an explicit target). Ground-target AoE callers gate
/// secondary-target damage on this return value.
pub async fn handle_use_ability(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
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
                return false;
            }
        };

        if combat::is_dead_state(entity.state_field) {
            return false;
        }
        if !entity.abilities.has_ability(ability_id) {
            tracing::debug!(
                entity_id,
                ability_id,
                "useAbility: entity does not have ability"
            );
            return false;
        }
        if entity.abilities.is_on_cooldown(ability_id) {
            tracing::debug!(entity_id, ability_id, "useAbility: ability on cooldown");
            return false;
        }

        // Range + target validation
        if target_id > 0 {
            if let Some(target) = space_mgr.get_entity(target_id as u32) {
                // Don't attack dead targets
                if combat::is_dead_state(target.state_field) {
                    tracing::debug!(
                        entity_id,
                        ability_id,
                        target_id,
                        "useAbility: target is dead"
                    );
                    return false;
                }
                // Range check
                let max_range = ability_def.as_ref().map_or(30.0, |d| {
                    if d.max_range > 0 {
                        d.max_range as f32
                    } else {
                        30.0
                    }
                });
                let dist = entity.position.distance_to(&target.position);
                if dist > max_range {
                    tracing::debug!(
                        entity_id,
                        ability_id,
                        distance = dist,
                        max_range,
                        "useAbility: target out of range"
                    );
                    out_of_range = true;
                }
            }
        }
    }

    if out_of_range {
        // Send onErrorCode to player: ERRORCODE_SYSTEM_Ability=0, CONDITION_FEEDBACK_OutsideWeaponRange=42
        let mut err_args = Vec::with_capacity(7);
        err_args.push(0u8); // SystemID
        err_args.extend_from_slice(&ability_id.to_le_bytes()); // InstanceID
        err_args.extend_from_slice(&42u16.to_le_bytes()); // ErrorCodeID
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 121, // ON_ERROR_CODE
                args: err_args,
            })
            .await;
        return false;
    }

    // Attack-while-holstered queue: when the player presses fire while
    // the weapon is holstered, defer the ability dispatch until the
    // draw animation has had time to play. Mirrors the
    // reload-while-holstered Phase A — draw the weapon, fire
    // `Item_Equip`, stash the ability + target, and let
    // `pending_attack_tick` re-invoke `handle_use_ability` after
    // `UNHOLSTER_DRAW_DURATION`.
    //
    // Only weapon attacks (`required_ammo > 0`) gate on this queue.
    // Non-weapon abilities (heals, buffs, self-casts) bypass entirely
    // — they don't need the weapon drawn to function, and they
    // shouldn't be locked out while a queued weapon shot is mid-draw.
    //
    // Subsequent weapon-attack presses during the draw window are
    // rejected so the first press locks in the queue. Ammo is NOT
    // checked here — the deferred re-invocation runs the normal ammo
    // check at fire time.
    let is_weapon_attack = ability_def.as_ref().is_some_and(|d| d.required_ammo > 0);
    let queued_attack_already_pending = space_mgr
        .get_entity(entity_id)
        .is_some_and(|e| e.pending_attack_at.is_some());
    if queued_attack_already_pending && is_weapon_attack {
        tracing::debug!(
            entity_id,
            ability_id,
            "useAbility: weapon attack already queued (mid-draw), ignoring input"
        );
        return false;
    }

    // Block weapon attacks while a bandolier slot swap is in progress.
    // The player's hands are physically holstering the old weapon and
    // drawing the new one; firing through that window would defeat the
    // animation penalty that makes weapon swaps a real loadout choice.
    // Non-weapon abilities (heals, buffs) are still permitted — the
    // queue is about the FIRE pose, not a global ability lockout.
    let slot_swap_in_progress = space_mgr.get_entity(entity_id).is_some_and(|e| {
        e.pending_slot_swap_at
            .is_some_and(|t| std::time::Instant::now() < t)
    });
    if slot_swap_in_progress && is_weapon_attack {
        tracing::debug!(
            entity_id,
            ability_id,
            "useAbility: bandolier slot swap in progress, weapon attack blocked"
        );
        return false;
    }

    let needs_unholster_queue = is_weapon_attack
        && !queued_attack_already_pending
        && space_mgr
            .get_entity(entity_id)
            .is_some_and(|e| e.is_player && e.weapon_holstered && e.threatened_mobs.is_empty());
    if needs_unholster_queue {
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.set_weapon_holstered(false);
            e.combat_exit_at = Some(std::time::Instant::now());
            e.holster_animation_complete_at = None;
            e.pending_attack_at = Some(
                std::time::Instant::now()
                    + super::super::cell_methods::player::world::UNHOLSTER_DRAW_DURATION,
            );
            e.pending_attack_ability_id = Some(ability_id);
            e.pending_attack_target_id = Some(target_id);
        }
        tracing::info!(
            entity_id,
            ability_id,
            target_id,
            "useAbility: holstered → queueing attack, drawing weapon first"
        );
        super::messaging::request_appearance_refresh(entity_id, tx, space_mgr).await;
        super::super::cell_methods::player::world::fire_item_sequence(
            entity_id,
            super::super::spawner::EVENT_ITEM_EQUIP,
            tx,
            space_mgr,
        )
        .await;
        return false;
    }

    // Mutable borrow for state changes
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return false,
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
    if required_ammo > 0 && entity.is_player && entity.reload_complete_at.is_some() {
        tracing::debug!(
            entity_id,
            ability_id,
            "useAbility: reload in progress, blocking fire"
        );
        return false;
    }

    let current_ammo = entity.active_ammo();
    if required_ammo > 0 && entity.is_player && current_ammo < required_ammo {
        tracing::debug!(
            entity_id,
            ability_id,
            current = current_ammo,
            required = required_ammo,
            "useAbility: not enough ammo"
        );
        return false;
    }

    let cooldown_secs =
        ability_def
            .as_ref()
            .map_or(2.0, |d| if d.cooldown > 0.0 { d.cooldown } else { 0.5 });
    let cooldown_duration = std::time::Duration::from_secs_f32(cooldown_secs);
    entity
        .abilities
        .start_ability_cooldown(ability_id, cooldown_duration);

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
        tracing::debug!(
            entity_id,
            ability_id,
            ammo_remaining = entity.active_ammo(),
            "useAbility: consumed ammo"
        );
    }

    // Get effect sequence ID for this ability invocation
    let effect_seq = entity.abilities.next_effect_id();

    tracing::info!(
        entity_id,
        ability_id,
        target_id,
        cooldown_secs,
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

    // Note on BSF_InCombat (bit 3): intentionally NOT set here. The bit is
    // derived from `threatened_mobs` and flips on via
    // `combat::generate_threat` → `enter_player_combat` when this attack
    // actually generates threat on a surviving NPC target (handled in
    // `damage_apply::apply_damage_to_target`). Setting it raw here used to
    // strand it for one-shot kills (target dies before generate_threat
    // runs) and target-less casts (early-return before damage_apply).

    // ── Send attack animation (onSequence) to attacker + witnesses ──
    // Look up the correct sequence_id from the event set. The client expects
    // the sequence_id from resources.sequences, NOT the event_set_id.
    // Reference: AbilityManager.py — self.manager.playSequence(beginSeq.seqId, ...)
    if let Some(event_set_id) = ability_def.as_ref().and_then(|d| d.event_set_id) {
        use super::super::spawner::{EVENT_ABILITY_BEGIN, EVENT_ABILITY_END};

        // Send Ability_Begin (event_id 1000) if the ability has a warmup phase
        let warmup = ability_def.as_ref().map_or(0.0, |d| d.warmup);
        if warmup > 0.0 {
            if let Some(&begin_seq_id) = space_mgr
                .sequence_map
                .get(&(event_set_id, EVENT_ABILITY_BEGIN))
            {
                let mut seq_args = Vec::with_capacity(28);
                seq_args.extend_from_slice(&begin_seq_id.to_le_bytes()); // KismetEventSetSeqID (sequence_id)
                seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
                seq_args.extend_from_slice(&target_id.to_le_bytes()); // TargetID
                seq_args.push(1); // PrimaryTarget = true
                seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
                seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs array count = 0
                seq_args.push(0); // ViewType = KISMET_VIEW_Witness
                seq_args.extend_from_slice(&effect_seq.to_le_bytes()); // InstanceId
                send_entity_method(entity_id, 1, seq_args, tx, space_mgr).await;
                // 1 = onSequence
            }
        }

        // Send Ability_End (event_id 1001) — the main ability fire animation
        if let Some(&end_seq_id) = space_mgr
            .sequence_map
            .get(&(event_set_id, EVENT_ABILITY_END))
        {
            let mut seq_args = Vec::with_capacity(28);
            seq_args.extend_from_slice(&end_seq_id.to_le_bytes()); // KismetEventSetSeqID (sequence_id)
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
            seq_args.extend_from_slice(&target_id.to_le_bytes()); // TargetID
            seq_args.push(1); // PrimaryTarget = true
            seq_args.extend_from_slice(&0.0f32.to_le_bytes()); // ImpactTime
            seq_args.extend_from_slice(&0u32.to_le_bytes()); // NameValuePairs array count = 0
            seq_args.push(0); // ViewType = KISMET_VIEW_Witness
            seq_args.extend_from_slice(&effect_seq.to_le_bytes()); // InstanceId
            send_entity_method(entity_id, 1, seq_args, tx, space_mgr).await; // 1 = onSequence
        } else {
            tracing::debug!(
                entity_id,
                ability_id,
                event_set_id,
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
        // Cooldown + ammo were consumed; the cast committed even though no
        // target was resolved. Ground-target callers see this as "primary
        // succeeded" and proceed with any AoE secondaries (which they
        // wouldn't have when no targets were in radius anyway).
        return true;
    }

    super::damage_apply::apply_damage_to_target(
        entity_id,
        target_id as u32,
        ability_id,
        &ability_def,
        effect_seq as u32,
        needs_ammo_stat_send,
        tx,
        space_mgr,
    )
    .await;
    true
}

#[cfg(test)]
mod tests;
