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
    //
    // Note on the holster bit: BSF_Holster (bit 8) was previously cleared
    // here as a "clear-on-fire" write. Removed per issue #333 — the SGW
    // client does not test bit 8 of `bStateField` anywhere
    // (`GameBeing_OnStateFieldUpdate` at `ghidra://SGW.exe@0x00e01c90`
    // dispatches only on bits 0-7). The visible "draw weapon on fire"
    // behavior is now driven by `CellEntity::weapon_holstered` plus a
    // `BeingAppearance` rebroadcast (Phase 2 of the holster fix).

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
    ///
    /// (This test previously also pinned a `state_field &= !BSF_HOLSTER`
    /// clear-on-fire write. That write was removed per issue #333 — the
    /// SGW client doesn't read bit 8 of `bStateField`, so the write was a
    /// no-op. Visible "draw weapon on fire" behavior is now driven by
    /// `CellEntity::weapon_holstered` + `BeingAppearance` rebroadcast,
    /// Phase 2 of the holster fix.)
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
}
