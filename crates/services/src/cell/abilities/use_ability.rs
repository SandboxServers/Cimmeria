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
    serialize_timer_update, AF_DEACTIVATE_AUTO_CYCLE, TIMER_ABILITY_COOLDOWN,
};

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

use super::messaging::{flush_attacker_ammo_stat, send_entity_method};

/// Broadcast `onStateFieldUpdate` to the player's own client after a
/// `BSF_AUTO_CYCLING` transition. Used at every arm/clear site in this file —
/// kept as a single helper so the routing rule (self-only, like BSF_InCombat
/// changes) lives in one place.
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
/// secondary-target damage on this — see PR #122 / Copilot review.
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
    // fires a different ability, cancel the loop before validation. This
    // matches python `AbilityManager.useAbility` (line 1019: `self.autoCycle
    // = False`) — the manual click was the player's intent to break the
    // cycle. The loop's own re-fire path always invokes with the stashed
    // ability_id, so this gate never trips for tick-driven re-fires.
    //
    // Same-ability manual fire is NOT a cancel: clicking the same weapon
    // on a different target should update the loop's target stash (handled
    // in the arm step after commit), not break the loop entirely.
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

    // ── Auto-cycle commit-time arm / deactivate ──────────────────────
    //
    // Now that the cast has committed (cooldown started), reconcile the
    // auto-cycle loop state. Three cases:
    //
    //   1. `AF_DEACTIVATE_AUTO_CYCLE` flag (mask `0x400`) on the firing
    //      ability — break the loop. Used for one-shot abilities the
    //      original design wanted to interrupt sustained auto-fire.
    //   2. `auto_cycle == true` (button armed) — stash the ability id and
    //      target id on the entity AND set `BSF_AUTO_CYCLING` so the
    //      client highlights the gun-icon button. The driver tick uses
    //      the stash to re-invoke this function every cooldown expiry.
    //   3. `auto_cycle == false` — no-op, ordinary one-off shot.
    //
    // The stash captures the SERVER-side target id, not the player's
    // current cursor. Subsequent same-ability manual fires update the
    // stash to follow the new target; the loop never reads the cursor.
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
    // The classification was captured before the mutable borrow ended.
    // Run the actual state mutation + broadcast now that we've dropped
    // back to passing `&mut space_mgr` (the cooldown timer send above
    // re-acquires the immutable borrow inside `send_entity_method`).
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
        // broadcast. Subsequent same-ability fires fall here.
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
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use crate::mercury::method_idx;
    use cimmeria_entity::abilities::AbilityDef;

    fn make_ability(id: i32, required_ammo: i32, max_range: i32) -> AbilityDef {
        AbilityDef {
            ability_id: id,
            name: "test".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo,
            event_set_id: None,
            velocity: 0.0,
        }
    }

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr
    }

    fn make_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
        mgr.create_entity(id, "Castle_CellBlock", pos, [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(id) {
            p.is_player = true;
            p.player_id = Some(100 + id as i32);
        }
    }

    fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
        let mut out = Vec::new();
        while let Ok(m) = rx.try_recv() {
            out.push(m);
        }
        out
    }

    #[tokio::test]
    async fn missing_entity_returns_false_and_emits_no_packets() {
        let mut mgr = make_mgr();
        let (tx, mut rx) = mpsc::channel(8);
        let committed = handle_use_ability(999, 1, 0, &tx, &mut mgr).await;
        assert!(!committed);
        assert!(drain(&mut rx).is_empty());
    }

    #[tokio::test]
    async fn entity_without_ability_returns_false() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        // No ability added to the entity.
        let (tx, mut rx) = mpsc::channel(8);
        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(!committed);
        assert!(drain(&mut rx).is_empty());
    }

    #[tokio::test]
    async fn cooldown_blocks_fire_and_emits_no_packets() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
            p.abilities
                .start_ability_cooldown(7, std::time::Duration::from_secs(60));
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, mut rx) = mpsc::channel(8);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(!committed);
        assert!(
            drain(&mut rx).is_empty(),
            "cooldown rejection must not emit any wire packets"
        );
    }

    /// Out-of-range hits the dedicated error-code branch — emits exactly
    /// one onErrorCode (method 121) carrying CONDITION_FEEDBACK_OutsideWeaponRange=42.
    /// Pin the byte layout (SystemID:u8 + InstanceID:i32 + ErrorCodeID:u16).
    #[tokio::test]
    async fn out_of_range_emits_on_error_code_with_condition_42() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        // Target far away beyond max_range=10.
        mgr.create_entity(2, "Castle_CellBlock", [100.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 10));
        let (tx, mut rx) = mpsc::channel(8);

        let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
        assert!(!committed);

        let msgs = drain(&mut rx);
        let err = msgs
            .iter()
            .find_map(|m| match m {
                CellToBaseMsg::EntityMethodCall {
                    entity_id: 1,
                    method_index,
                    args,
                } if *method_index == method_idx::ON_ERROR_CODE => Some(args.clone()),
                _ => None,
            })
            .expect("out-of-range must emit onErrorCode");
        // Layout: u8 SystemID + i32 InstanceID + u16 ErrorCodeID = 7 bytes.
        assert_eq!(err.len(), 7);
        assert_eq!(err[0], 0, "SystemID should be ERRORCODE_SYSTEM_Ability=0");
        assert_eq!(
            i32::from_le_bytes([err[1], err[2], err[3], err[4]]),
            7,
            "InstanceID should echo the ability_id"
        );
        assert_eq!(
            u16::from_le_bytes([err[5], err[6]]),
            42,
            "ErrorCodeID should be CONDITION_FEEDBACK_OutsideWeaponRange=42"
        );
    }

    /// Reload-in-flight blocks fire even when the deadline elapsed by clock.
    /// Regression guard: the gate is `is_some()`, not `now < deadline`.
    /// Setting `reload_complete_at` to a past instant must still block,
    /// because only the 100ms reload-completion tick clears it.
    #[tokio::test]
    async fn reload_in_flight_blocks_fire_even_with_past_deadline() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            // Weapon drawn so the attack-while-holstered queue doesn't
            // intercept — this test is about the reload-in-flight gate.
            p.weapon_holstered = false;
            p.abilities.add_ability(7);
            p.reload_complete_at =
                Some(std::time::Instant::now() - std::time::Duration::from_secs(1));
        }
        mgr.ability_defs.insert(7, make_ability(7, 1, 30));
        let (tx, mut rx) = mpsc::channel(8);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(
            !committed,
            "fire must be blocked while reload_complete_at is_some(), regardless of wall-clock"
        );
        assert!(drain(&mut rx).is_empty());
    }

    #[tokio::test]
    async fn no_ammo_for_player_blocks_fire() {
        use cimmeria_entity::cell_entity::BandolierItem;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            // Weapon drawn so the attack-while-holstered queue doesn't
            // intercept — this test is about the no-ammo gate.
            p.weapon_holstered = false;
            p.abilities.add_ability(7);
            // Active slot 0, ammo 0 of 30.
            p.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 0,
                    cur_ammo_type: 2,
                },
            );
        }
        mgr.ability_defs.insert(7, make_ability(7, 1, 30));
        let (tx, mut rx) = mpsc::channel(8);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(!committed);
        assert!(drain(&mut rx).is_empty());
    }

    /// Cast against a dead target rejects without consuming ammo or starting
    /// the cooldown. Regression guard for the dead-target branch.
    #[tokio::test]
    async fn dead_target_blocks_fire_without_consuming_resources() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        mgr.create_entity(2, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
        }
        if let Some(t) = mgr.get_entity_mut(2) {
            t.set_state_flag(crate::cell::combat::BSF_DEAD);
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(8);

        let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
        assert!(!committed);
        // Cooldown must not have been started — a follow-up cast with a
        // live target should be allowed.
        assert!(
            !mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
            "rejecting against a dead target must not start the cooldown"
        );
    }

    /// `use_ability` does not touch BSF_InCombat (bit 3). The bit is
    /// derived from `threatened_mobs` and flips on via
    /// `combat::generate_threat` → `enter_player_combat` from
    /// `damage_apply::apply_damage_to_target` when this attack actually
    /// hits a surviving NPC. A self-cast (target_id == 0) commits
    /// cooldown + ammo but produces no threat, so the bit stays
    /// unchanged — pinned here so a regression that re-introduces a raw
    /// `state_field |= BSF_IN_COMBAT` setter on this path doesn't slip
    /// through (stuck-bit hazard for target-less casts).
    #[tokio::test]
    async fn commit_leaves_bsf_in_combat_alone_on_self_cast() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(committed);

        let s = mgr.get_entity(1).unwrap().state_field;
        assert_eq!(
            s & (1 << 3),
            0,
            "BSF_InCombat must NOT be set by use_ability — \
             it's now derived from threatened_mobs via enter_player_combat"
        );
    }

    /// Target-less / no-target cast (target_id == 0) must not set
    /// BSF_InCombat on the attacker. Stuck-bit regression guard: the
    /// previous raw `state_field |= BSF_IN_COMBAT` here ran before the
    /// `if target_id <= 0` early-return downstream, so a self-cast
    /// would flip the in-combat HUD forever (no NPC death ever runs
    /// the clear path).
    #[tokio::test]
    async fn no_target_cast_does_not_set_bsf_in_combat() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(committed, "self-cast still commits (cooldown + ammo)");

        let s = mgr.get_entity(1).unwrap().state_field;
        assert_eq!(
            s & (1 << 3),
            0,
            "no-target cast must not strand BSF_InCombat — no NPC death \
             would ever run the clear path"
        );
        // threatened_mobs must also stay empty so the regen tick (which
        // gates on the set) is free to fire.
        assert!(
            mgr.get_entity(1).unwrap().threatened_mobs.is_empty(),
            "no-target cast must leave threatened_mobs empty"
        );
    }

    /// Attack-while-holstered: pressing fire on a weapon
    /// attack while OOC + holstered must defer the ability — draw
    /// weapon, fire `Item_Equip`, stash the call on
    /// `pending_attack_*`, and return false WITHOUT committing
    /// cooldown or consuming ammo. The `pending_attack_tick`
    /// re-invokes after `UNHOLSTER_DRAW_DURATION` to fire for real.
    ///
    /// Bug shape this catches: a refactor removes the queue and the
    /// first attack on a holstered weapon fires with no animation
    /// (the playtest symptom that drove this fix).
    #[tokio::test]
    async fn attack_while_holstered_queues_and_draws_without_committing() {
        use cimmeria_entity::cell_entity::BandolierItem;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
            p.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
            p.weapon_holstered = true; // OOC + holstered
            p.abilities.add_ability(7);
            p.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 30,
                    cur_ammo_type: 2,
                },
            );
        }
        // required_ammo=1 → triggers the weapon-attack queue.
        mgr.ability_defs.insert(7, make_ability(7, 1, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(
            !committed,
            "attack-while-holstered must NOT commit on the first press — \
             the queue defers the ability until the draw animation finishes",
        );

        let e = mgr.get_entity(1).unwrap();
        assert!(!e.weapon_holstered, "Phase A draws the weapon");
        assert!(
            e.combat_exit_at.is_some(),
            "OOC re-holster timer must arm so the weapon goes away post-fight",
        );
        assert!(
            e.pending_attack_at.is_some(),
            "pending_attack_at must stamp so pending_attack_tick can fire the queued ability",
        );
        assert_eq!(
            e.pending_attack_ability_id,
            Some(7),
            "ability_id must be stashed for Phase B dispatch",
        );
        assert!(
            !e.abilities.is_on_cooldown(7),
            "Phase A must NOT start the cooldown — cooldown commits in Phase B",
        );
        assert_eq!(
            e.bandolier_items[&0].current_ammo, 30,
            "Phase A must NOT consume ammo — ammo check happens in Phase B",
        );
    }

    /// Attack inputs DURING the draw window are rejected so the first
    /// press locks in the queue. Spamming clicks must not change the
    /// queued ability/target or restart the draw timer.
    #[tokio::test]
    async fn attack_while_queued_is_rejected_input() {
        use cimmeria_entity::cell_entity::BandolierItem;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        let queued_stamp = std::time::Instant::now();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
            p.weapon_holstered = true;
            p.abilities.add_ability(7);
            p.bandolier_items.insert(
                0,
                BandolierItem {
                    item_id: 1,
                    clip_size: 30,
                    default_ammo_type: 2,
                    current_ammo: 30,
                    cur_ammo_type: 2,
                },
            );
            // Already queued from a previous press.
            p.pending_attack_at = Some(queued_stamp);
            p.pending_attack_ability_id = Some(99);
            p.pending_attack_target_id = Some(42);
        }
        mgr.ability_defs.insert(7, make_ability(7, 1, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(
            !committed,
            "second press during draw window must be rejected"
        );

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.pending_attack_ability_id,
            Some(99),
            "the existing queued ability must NOT be overwritten by the second press",
        );
        assert_eq!(
            e.pending_attack_target_id,
            Some(42),
            "the existing queued target must NOT be overwritten",
        );
        assert_eq!(
            e.pending_attack_at,
            Some(queued_stamp),
            "the draw timer must NOT be restarted by spamming clicks",
        );
    }

    /// Non-weapon abilities (required_ammo == 0 — heals, buffs,
    /// self-casts) must NOT trigger the unholster queue. Pin the gate
    /// so a refactor that drops the `required_ammo > 0` check doesn't
    /// turn every self-cast on a holstered player into a 1s-delayed
    /// queued cast.
    #[tokio::test]
    async fn non_weapon_ability_skips_unholster_queue_when_holstered() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.weapon_holstered = true;
            p.abilities.add_ability(7);
        }
        // required_ammo=0 → non-weapon ability (heal, buff, self-cast).
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(
            committed,
            "non-weapon ability on a holstered player must fire immediately, \
             not queue — the weapon isn't being used",
        );
        assert!(
            mgr.get_entity(1).unwrap().pending_attack_at.is_none(),
            "non-weapon ability must NOT set pending_attack_at",
        );
    }

    /// Non-weapon abilities (heals, buffs, self-casts) must STILL fire
    /// even when a weapon attack is queued behind the unholster
    /// animation. The queue is about the unholster choreography, not
    /// a global ability lockout — a player mid-draw should still be
    /// able to heal themselves.
    ///
    /// Bug shape: a refactor that gates the `queued_attack_already_pending`
    /// early reject without also checking `is_weapon_attack` regresses
    /// to "queue blocks ALL abilities" — heals get silently dropped
    /// the moment a weapon attack is queued.
    #[tokio::test]
    async fn non_weapon_ability_fires_even_when_weapon_attack_queued() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        let queued_stamp = std::time::Instant::now() + std::time::Duration::from_secs(1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.weapon_holstered = false; // weapon drawn (just queued an attack)
            p.abilities.add_ability(7);
            // Simulate a queued weapon attack from a prior press.
            p.pending_attack_at = Some(queued_stamp);
            p.pending_attack_ability_id = Some(99);
            p.pending_attack_target_id = Some(42);
        }
        // Ability 7 is non-weapon (required_ammo=0 — heal/buff).
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(
            committed,
            "non-weapon ability must commit even while a weapon attack \
             is queued — the queue is animation-state, not a global \
             ability lockout",
        );

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.pending_attack_at,
            Some(queued_stamp),
            "queued weapon attack must NOT be cleared by a non-weapon \
             ability firing through the queue",
        );
        assert_eq!(
            e.pending_attack_ability_id,
            Some(99),
            "queued ability id must be untouched",
        );
    }

    /// Self-target (target_id == 0) commits cooldown and ammo consume but
    /// skips combat resolution — the function returns true. Pin so a
    /// regression that re-routes self-cast through damage_apply (and would
    /// then bail on missing target) doesn't go silently.
    #[tokio::test]
    async fn self_target_commits_returns_true() {
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(committed);
        assert!(mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7));
    }

    // ── Auto-cycle commit-time arm and clear paths ─────────────────────

    /// First commit while `auto_cycle == true` arms the loop:
    /// stashes `(ability_id, target_id)`, sets `BSF_AUTO_CYCLING`,
    /// and broadcasts `onStateFieldUpdate` so the client highlights
    /// the gun-icon button.
    ///
    /// Bug shape this catches: a refactor that calls `arm_auto_cycle`
    /// but skips the broadcast leaves the server thinking the loop
    /// is running while the client never lights the button — the
    /// exact missing-feedback hazard #341 closes.
    #[tokio::test]
    async fn auto_cycle_first_commit_arms_loop_and_broadcasts_state_field() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
            p.abilities.auto_cycle = true; // button pressed earlier
            p.weapon_holstered = false; // skip the unholster queue
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, mut rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(committed);

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.abilities.auto_cycle_ability_id,
            Some(7),
            "ability id must be stashed for the driver tick to re-fire",
        );
        assert_eq!(
            e.abilities.auto_cycle_target_id,
            Some(0),
            "target id must be stashed even for self-cast (target_id=0)",
        );
        assert_ne!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "BSF_AUTO_CYCLING must be set so the client highlights the gun-icon button",
        );

        // The state-field broadcast is what fires
        // `EmitAutoCycleStateChanged` on the client — without it the
        // button stays dark even though the server is now looping.
        let msgs = drain(&mut rx);
        let saw_state_field_update = msgs.iter().any(|m| {
            matches!(
                m,
                CellToBaseMsg::EntityMethodCall {
                    entity_id: 1,
                    method_index,
                    ..
                } if *method_index == method_idx::ON_STATE_FIELD_UPDATE
            )
        });
        assert!(
            saw_state_field_update,
            "first commit must emit onStateFieldUpdate so the client lights the button"
        );
    }

    /// AF_DEACTIVATE_AUTO_CYCLE-flagged abilities firing while
    /// auto-cycle is armed CANCEL the loop after commit. The cast
    /// still goes through (cooldown started, ammo consumed) — the
    /// flag just stops the re-fire chain so one-shot abilities don't
    /// get repeated by the driver. Mirrors python `AbilityManager`
    /// behavior for flag mask `0x400`.
    ///
    /// Bug shape: a refactor that arms unconditionally after commit
    /// makes one-shot specials (signature moves) auto-repeat forever.
    #[tokio::test]
    async fn af_deactivate_auto_cycle_clears_loop_on_commit() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        use cimmeria_entity::abilities::AF_DEACTIVATE_AUTO_CYCLE;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
            // Pre-armed loop with a different ability stashed (so the
            // manual-override gate doesn't trip on entry — this test
            // is about the AF_DEACTIVATE-flag exit path, not the
            // manual-override exit path).
            p.abilities.auto_cycle = true;
            p.abilities.auto_cycle_ability_id = Some(7);
            p.abilities.auto_cycle_target_id = Some(0);
            p.set_state_flag(BSF_AUTO_CYCLING);
            p.weapon_holstered = false;
        }
        // Ability 7 has the deactivate-auto-cycle flag set.
        let mut deactivating_ability = make_ability(7, 0, 30);
        deactivating_ability.flags = AF_DEACTIVATE_AUTO_CYCLE;
        mgr.ability_defs.insert(7, deactivating_ability);
        let (tx, _rx) = mpsc::channel(64);

        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(committed, "the cast itself must still commit");

        let e = mgr.get_entity(1).unwrap();
        assert!(
            !e.abilities.auto_cycle,
            "AF_DEACTIVATE_AUTO_CYCLE must clear the auto_cycle flag",
        );
        assert!(e.abilities.auto_cycle_ability_id.is_none());
        assert!(e.abilities.auto_cycle_target_id.is_none());
        assert_eq!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "BSF_AUTO_CYCLING must be cleared so the client un-highlights the button",
        );
    }

    /// Manual fire of a different ability while auto-cycle is armed
    /// cancels the loop on entry — the player's intent was to break
    /// the cycle and switch abilities. The new manual cast still
    /// commits normally.
    ///
    /// Mirrors python `AbilityManager.useAbility` (line 1019:
    /// `self.autoCycle = False` on every direct call). The loop's
    /// own tick-driven re-fires invoke with the stashed ability id,
    /// so this gate never trips for loop-driven shots — only manual
    /// override clicks.
    ///
    /// Bug shape: a refactor that drops the manual-override gate
    /// turns ability swaps into "fire the new ability AND keep
    /// looping the old one," with no way to actually break the loop
    /// without pressing the gun-icon button.
    #[tokio::test]
    async fn manual_fire_of_different_ability_cancels_auto_cycle() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
            p.abilities.add_ability(8); // different ability
            p.abilities.auto_cycle = true;
            p.abilities.auto_cycle_ability_id = Some(7); // armed on ability 7
            p.abilities.auto_cycle_target_id = Some(99);
            p.set_state_flag(BSF_AUTO_CYCLING);
            p.weapon_holstered = false;
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        mgr.ability_defs.insert(8, make_ability(8, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        // Manual fire of ability 8 (different from stashed 7).
        let committed = handle_use_ability(1, 8, 0, &tx, &mut mgr).await;
        assert!(committed, "the new ability still fires");

        let e = mgr.get_entity(1).unwrap();
        assert!(
            !e.abilities.auto_cycle,
            "different-ability manual fire must clear the auto_cycle flag",
        );
        assert!(e.abilities.auto_cycle_ability_id.is_none());
        assert!(e.abilities.auto_cycle_target_id.is_none());
        assert_eq!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "BSF must clear so the client un-highlights the button",
        );
    }

    /// Same-ability manual fire while auto-cycle is armed does NOT
    /// cancel the loop — it just refreshes the target stash. Right-
    /// clicking the same weapon on a new enemy should redirect the
    /// loop, not break it.
    ///
    /// Bug shape: a refactor that conflates "any manual fire" with
    /// "different-ability fire" turns every right-click into a loop
    /// reset, which defeats the purpose of auto-cycle (the player
    /// would have to re-press the button every time they switch
    /// targets).
    #[tokio::test]
    async fn same_ability_manual_fire_redirects_target_without_breaking_loop() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_mgr();
        make_player(&mut mgr, 1, [0.0; 3]);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.add_ability(7);
            p.abilities.auto_cycle = true;
            p.abilities.auto_cycle_ability_id = Some(7);
            p.abilities.auto_cycle_target_id = Some(99); // old target
            p.set_state_flag(BSF_AUTO_CYCLING);
            p.weapon_holstered = false;
        }
        mgr.ability_defs.insert(7, make_ability(7, 0, 30));
        let (tx, _rx) = mpsc::channel(64);

        // Same-ability fire but at a different target (0 = self-cast).
        let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
        assert!(committed);

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.abilities.auto_cycle,
            "same-ability fire must NOT break the loop",
        );
        assert_eq!(
            e.abilities.auto_cycle_ability_id,
            Some(7),
            "ability id must be stable",
        );
        assert_eq!(
            e.abilities.auto_cycle_target_id,
            Some(0),
            "target stash must update to the new target",
        );
        assert_ne!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "BSF must remain set — the loop continues",
        );
    }
}
