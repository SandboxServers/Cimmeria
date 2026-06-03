//! The main `useAbility(abilityId, targetId)` flow.
//!
//! Validates the call, consumes ammo, starts the cooldown, fires animations,
//! resolves damage, and pushes the resulting wire messages (timer, sequence,
//! state-field, effect-results, stat-update, death) back to the relevant
//! clients. Splits cleanly into a validation phase, a consume/fire phase,
//! and a target-resolution phase — kept as one function because the borrow
//! ordering between immutable and mutable space_manager access is delicate
//! and a hand-rolled split here would just trade lines for `&mut` plumbing.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, AF_DEACTIVATE_AUTO_CYCLE, TIMER_ABILITY_COOLDOWN,
};
use cimmeria_entity::stats::HEALTH;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

use super::messaging::{flush_attacker_ammo_stat, send_entity_method};

/// Broadcast `onStateFieldUpdate` after a `BSF_AUTO_CYCLING` transition.
/// Self-only routing (like BSF_InCombat changes) — kept in one place so
/// arm/clear sites don't drift apart on the wire rule.
async fn send_state_field(
    entity_id: u32,
    new_state: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    send_entity_method(
        entity_id,
        crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
        new_state.to_le_bytes().to_vec(),
        tx,
        space_mgr,
    )
    .await;
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
///
/// Returns `true` when the cast committed (validation passed and the
/// cooldown/ammo consume took effect — which is also when the target's
/// damage resolution and wire packets fired). Returns `false` when any
/// pre-consume guard rejected the call (entity missing/dead, no
/// ability, on cooldown, reload in flight, no ammo, or out-of-range
/// for an explicit target). Ground-target AoE callers gate
/// secondary-target damage on this return value.
#[tracing::instrument(
    name = "combat.use_ability",
    level = "info",
    skip_all,
    fields(entity_id, ability_id, target_id)
)]
pub async fn handle_use_ability(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // ── Look up ability definition from DB (before mutable borrow) ──
    let ability_def = space_mgr.ability_defs.get(&ability_id).cloned();

    // ── Auto-cycle manual-override gate ──
    //
    // If the player has auto-cycle armed for one ability and manually
    // fires a different ability, cancel the loop before validation.
    // Matches python `AbilityManager.useAbility` (line 1019:
    // `self.autoCycle = False`) — the manual click is intent to break
    // the cycle. Tick-driven re-fires always invoke with the stashed
    // ability_id, so this never trips for loop-driven shots. Same-
    // ability manual fire is NOT a cancel: clicking the same weapon
    // on a different target should let the loop continue and let the
    // next tick redirect via `current_target_id`.
    let override_clears_loop = space_mgr.get_entity(entity_id).is_some_and(|e| {
        e.is_player
            && e.abilities.auto_cycle
            && e.abilities
                .auto_cycle_ability_id
                .is_some_and(|id| id != ability_id)
    });
    if override_clears_loop {
        if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
            tracing::info!(
                entity_id,
                ability_id,
                "auto-cycle: cleared by manual override (different ability fired)"
            );
            send_state_field(entity_id, new_state, tx, space_mgr).await;
        }
    }

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
            // PR #420 follow-up: weapon-granted abilities are resolved
            // at fire time from `items_event_sets` (see `resolve.rs` and
            // the hostile-NPC right-click path in `interaction.rs`).
            // They are NOT injected into `entity.abilities` on equip —
            // `entity.abilities` carries the player's *trained / known /
            // archetype-starter* set, distinct from weapon-granted IDs.
            //
            // Without this fallback, every weapon fire is rejected with
            // "entity does not have ability". Live observation 2026-06-02
            // (player 72.206.34.241): 25 consecutive useAbility(579)
            // rejections for a player holding the pistol that grants 579
            // — fire button effectively dead for that session.
            //
            // We need to drop the immutable `entity` borrow before the
            // call into the helper (which takes `&SpaceManager`) — the
            // outer `&mut space_mgr` is fine to re-borrow as `&` here
            // since we're inside the immutable-checks block.
            let granted_by_weapon = super::resolve::is_ability_granted_by_active_weapon(
                space_mgr, entity_id, ability_id,
            );
            if !granted_by_weapon {
                // WARN, not DEBUG: a rejected fire is an actionable
                // signal that the player is mashing a button and the
                // server is silently dropping every attempt. The 25-
                // per-second rate observed during the regression that
                // motivated this fix is exactly the volume that
                // disappears at DEBUG and burns at WARN — which is what
                // we want operators to see.
                tracing::warn!(
                    entity_id,
                    ability_id,
                    "useAbility: ability not in known set and not granted by active weapon"
                );
                return false;
            }
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

    // Stash the just-fired ability so `setAutoCycle(1)` can fire it
    // immediately on the next button press. Distinct from
    // `auto_cycle_ability_id` (the LOOP's committed ability, cleared
    // on stop): this field persists across auto-cycle on/off cycles
    // for the whole session. NPCs use `chooseAbility` per-fire and
    // don't need the stash.
    if entity.is_player {
        entity.abilities.last_fired_ability_id = Some(ability_id);
    }

    // ── Auto-cycle commit-time arm / deactivate classification ──
    //
    // Three cases after the cooldown has started:
    //
    //   1. `AF_DEACTIVATE_AUTO_CYCLE` flag (mask `0x400`) on the firing
    //      ability — break the loop. One-shot specials that mustn't
    //      auto-repeat.
    //   2. `auto_cycle == true` (button armed) — stash the ability id
    //      AND set `BSF_AUTO_CYCLING`. The driver tick reads
    //      `current_target_id` LIVE at re-fire time so target stash
    //      isn't needed here.
    //   3. `auto_cycle == false` — no-op.
    //
    // Mutation + broadcast run AFTER the cooldown-timer send below
    // (which would re-acquire the immutable borrow).
    let is_player = entity.is_player;
    let auto_cycle_armed = entity.abilities.auto_cycle;
    let has_deactivate_flag = ability_def
        .as_ref()
        .is_some_and(|d| d.flags & AF_DEACTIVATE_AUTO_CYCLE != 0);

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

    // ── Auto-cycle commit: arm or DEACTIVATE-flag clear ──
    //
    // Classification was captured before the mutable borrow ended.
    // Run the actual state mutation + broadcast now that the cooldown
    // timer send is past.
    if is_player && auto_cycle_armed {
        if has_deactivate_flag {
            if let Some(new_state) = combat::clear_auto_cycle(space_mgr, entity_id) {
                tracing::info!(
                    entity_id,
                    ability_id,
                    "auto-cycle: cleared by AF_DEACTIVATE_AUTO_CYCLE flag"
                );
                send_state_field(entity_id, new_state, tx, space_mgr).await;
            }
        } else if let Some(new_state) =
            combat::arm_auto_cycle(space_mgr, entity_id, ability_id, target_id)
        {
            tracing::info!(
                entity_id,
                ability_id,
                target_id,
                "auto-cycle: armed (first commit) — BSF_AUTO_CYCLING set"
            );
            send_state_field(entity_id, new_state, tx, space_mgr).await;
        }
        // Bit-already-set path: `arm_auto_cycle` updates the stash
        // unconditionally; only the `Some(new_state)` branch needs to
        // broadcast.
    }

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
        maybe_trigger_auto_reload(entity_id, needs_ammo_stat_send, ability_id, tx, space_mgr).await;
        return true;
    }

    // Phase J: cancel any channelled effects this attacker started
    // with a DIFFERENT ability. Same-ability re-fire keeps the channel
    // alive (it'll refresh via `register_active_effect`'s same-source
    // rule). Cancellation MUST happen before the new damage applies so
    // the wire ordering reads "old channel cleared, new ability fired".
    crate::cell::effects::cancel_channels_from_attacker(entity_id, Some(ability_id), tx, space_mgr)
        .await;

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

    // Cone AoE fan-out — once the primary takes damage,
    // sweep every effect on this ability for `TCM_AECone` and apply
    // damage to any additional hostiles caught in the cone. Returns
    // alive→dead transitions so the caller's kill-credit wrapper can
    // fire entity_death for each. `target_id > 0` already enforced
    // because we'd have early-returned with no-target above.
    let cone_deaths = super::cone_aoe::fan_out_cone_effects(
        entity_id,
        target_id as u32,
        ability_id,
        &ability_def,
        tx,
        space_mgr,
    )
    .await;
    // Stash the cone deaths on the attacker so
    // `handle_use_ability_with_kill_credit` can pick them up after
    // we return. Persisting via a per-attacker scratchpad keeps the
    // function signature stable for non-kill-credit callers.
    if !cone_deaths.is_empty() {
        if let Some(att) = space_mgr.get_entity_mut(entity_id) {
            att.last_aoe_deaths.extend(cone_deaths);
        }
    }

    maybe_trigger_auto_reload(entity_id, needs_ammo_stat_send, ability_id, tx, space_mgr).await;
    true
}

/// If this fire decremented player ammo to zero AND the player's
/// `autoReload` client option is set, automatically request a reload —
/// the same path the manual `R` keypress takes.
///
/// Routes through [`crate::cell::cell_methods::player::world::handle_reload`]
/// so the Phase A draw-window logic and Phase B warmup/cooldown are both
/// honoured uniformly with manual reloads. Gated on `is_player` because
/// NPCs do not consume ammo (their `set_slot_ammo` branch never runs).
///
/// Called after `apply_damage_to_target` releases the entity borrow so
/// `handle_reload` can re-acquire `&mut SpaceManager` for its own state
/// mutations. Re-entrancy with the per-tick `pending_reload_tick` is fine:
/// `handle_reload`'s phase-A guard rejects a second invocation while the
/// draw window is mid-flight (`pending_reload_at` is in the future).
/// Test-only re-export of [`maybe_trigger_auto_reload`] so cross-module
/// tests can pin the gate decisions directly without driving the full
/// fire pipeline. Production paths inside this module call the
/// non-public version.
#[cfg(test)]
pub(crate) async fn maybe_trigger_auto_reload_for_test(
    entity_id: u32,
    ammo_was_consumed: bool,
    ability_id: i32,
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
) {
    maybe_trigger_auto_reload(entity_id, ammo_was_consumed, ability_id, tx, space_mgr).await;
}

async fn maybe_trigger_auto_reload(
    entity_id: u32,
    ammo_was_consumed: bool,
    ability_id: i32,
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
) {
    if !ammo_was_consumed {
        return;
    }
    // Snapshot the gate inside an immutable borrow; release before
    // `handle_reload` re-acquires the mutable borrow.
    let should_reload = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            e.is_player
                && e.system_options.auto_reload
                // Active slot must actually carry a clip — melee
                // weapons report `active_clip_size() == 0` and
                // `active_ammo() == 0` would otherwise look like
                // "out of bullets" and queue a no-op reload that the
                // `handle_reload` "already at max ammo" early-return
                // would catch. Skipping here saves one wasted call.
                && e.active_clip_size() > 0
                && e.active_ammo() == 0
                // Don't queue auto-reload on top of an in-flight reload —
                // `handle_reload` would early-return at "already at max
                // ammo" or push another deferred phase A on top of the
                // existing one.
                && e.reload_complete_at.is_none()
                && e.pending_reload_at.is_none()
        }
        None => false,
    };
    if !should_reload {
        return;
    }
    tracing::info!(
        entity_id,
        ability_id,
        "useAbility: auto-reload triggered (autoReload + clip empty)"
    );
    crate::cell::cell_methods::player::world::handle_reload(entity_id, tx, space_mgr).await;
}

/// `handle_use_ability` + content-engine kill-credit hook.
///
/// **Use this from every single-target player-driven path that calls
/// [`handle_use_ability`] directly.** Calls `handle_use_ability` to
/// resolve the ability, then — if the attacker is a player who just
/// transitioned a tagged NPC from alive→dead — fires the `EntityDeath`
/// content event so mission KillCount chains (e.g., "kill 5
/// Hallway_Guards") progress.
///
/// **Not** for AoE / ground-target callers: those go through
/// [`super::handle_use_ability_on_ground`], which returns the set of
/// every NPC that died during the cast and fires per-death
/// `fire_entity_death` at the caller layer. The AoE path is the only
/// other single canonical kill-credit fan-out today; collapsing them
/// would require returning a Vec<entity_id> from this helper too.
///
/// Why this isn't baked into `handle_use_ability` itself: NPC AI also
/// calls `handle_use_ability`, and NPC kills shouldn't fire
/// `EntityDeath` (the killer has no `player_id` — there's no mission to
/// credit). Tests that exercise `handle_use_ability` mechanics also
/// don't need to thread a `ChainEngine` through. Keeping the bare
/// function callable from those sites preserves both invariants.
///
/// Mirrors the python `useAbility` → `attemptDeath` → `_doDeath` chain
/// where the cell-side death callback was the canonical credit point.
pub async fn handle_use_ability_with_kill_credit(
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // Snapshot whether the target was a live NPC *before* the ability
    // resolves. Without this, hitting an already-dead corpse would
    // re-fire `fire_entity_death` and double-count mission progress on
    // every post-death swing. Player targets are excluded because PvP
    // kills don't drive mission progression today.
    let was_alive_before = if target_id > 0 {
        space_mgr
            .get_entity(target_id as u32)
            .is_some_and(|t| !t.is_player && t.stats.get(HEALTH).is_some_and(|s| s.cur > 0))
    } else {
        false
    };

    let committed = handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;

    // Skip the death check when the ability was rejected pre-consume —
    // nothing was damaged, so nothing died. Also short-circuits the
    // common no-target paths (target_id == 0).
    if !committed || !was_alive_before {
        return committed;
    }

    let target_eid = target_id as u32;
    let just_died = space_mgr
        .get_entity(target_eid)
        .is_some_and(|t| t.stats.get(HEALTH).is_some_and(|s| s.cur <= 0));
    if !just_died {
        return committed;
    }

    // Resolve the target's content-engine tag (the chain trigger key,
    // e.g. "Hallway01_Guard") and the killer's `player_id` (the
    // mission-context key). Either being absent is benign — a tagless
    // NPC just doesn't progress any chain; a player_id-less killer
    // (NPC AI shouldn't reach this helper, but be defensive) skips
    // with a warn so the unexpected case stays visible.
    let tag = match space_mgr.get_entity(target_eid).and_then(|t| t.tag.clone()) {
        Some(t) => t,
        None => return committed,
    };
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(pid) => pid,
        None => {
            tracing::warn!(
                entity_id, npc_tag = %tag,
                "handle_use_ability_with_kill_credit: killer has no player_id — skipping EntityDeath event"
            );
            return committed;
        }
    };

    crate::cell::content::fire_entity_death(entity_id, player_id, &tag, engine, tx, space_mgr)
        .await;

    // Cone AoE kill credit: drain the per-attacker scratchpad that
    // `handle_use_ability` populated with cone-secondary deaths and
    // fire `entity_death` for each tagged kill. Matches the same
    // discipline as `handle_use_ability_on_ground`.
    let cone_dead_ids: Vec<u32> = space_mgr
        .get_entity_mut(entity_id)
        .map(|att| std::mem::take(&mut att.last_aoe_deaths))
        .unwrap_or_default();
    for dead_eid in cone_dead_ids {
        let dead_tag = space_mgr.get_entity(dead_eid).and_then(|t| t.tag.clone());
        if let Some(t) = dead_tag {
            crate::cell::content::fire_entity_death(
                entity_id, player_id, &t, engine, tx, space_mgr,
            )
            .await;
        }
    }
    committed
}

#[cfg(test)]
mod tests;
