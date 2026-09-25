//! Per-target damage application — extracted from `use_ability::handle_use_ability`
//! so AoE / ground-target paths can apply damage to multiple targets without
//! re-running the consume/cooldown/timer/sequence wire packets that should
//! fire exactly once per ability invocation.
//!
//! The function takes the already-resolved `effect_seq` and `ability_def`
//! plus a `needs_ammo_stat_send` flag that the caller controls — the primary
//! target gets `true` (the consume happened upstream and the ammo stat needs
//! to be flushed after the damage commits), AoE secondary targets get
//! `false` (the primary already flushed).

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{serialize_effect_results, AbilityDef, DT_PHYSICAL};
use cimmeria_entity::stats::HEALTH;

use super::super::combat;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;

use super::messaging::{
    flush_attacker_ammo_stat, send_entity_method, send_entity_method_to_self_and_witnesses,
};
use super::rng::pseudo_random_seed;

/// Resolve damage from `entity_id` to `target_eid` for ability `ability_id`.
///
/// Performs:
///   - Snapshot attacker stats; bail if attacker missing.
///   - Read damage NVPs from the ability's effect definitions.
///   - Compute QR + roll a hit result for this attacker/target pair.
///   - Apply health/focus damage to the target.
///   - Detect death (direct damage).
///   - Send `onEffectResults` to the attacker (witnesses pick it up via
///     entity routing) and to the target if the target is a player.
///   - Send `onStatUpdate` to the target.
///   - Optionally flush the attacker's dirty ammo stat (for the primary
///     consume path; AoE-secondary calls pass `false` so they don't
///     re-flush).
///   - On death, call into [`super::death::resolve_death`] — the single
///     kill path (state mutations, wire burst, threat drain, death
///     sequence, kill XP, onBeginAidWait).
///   - After the effect scripts run, sweep for an effect-driven death
///     (a script's HEALTH bleed finishing a target the direct damage
///     left standing) and resolve it in the SAME ability resolution.
///     `resolve_death` is idempotent, so a target the direct-damage arm
///     already killed is not re-killed.
///   - On survival, call into [`combat::generate_threat`] which mirrors
///     the threat-table addition into the player's `threatened_mobs`
///     set (#92) and broadcasts the BSF_InCombat transition if needed.
///
/// `effect_seq` should be unique per (attacker, target) pair within the
/// same invocation — AoE callers should generate a fresh `effect_seq`
/// for each secondary target so the client can correlate per-target
/// effect packets independently.
pub(super) async fn apply_damage_to_target(
    entity_id: u32,
    target_eid: u32,
    ability_id: i32,
    ability_def: &Option<AbilityDef>,
    effect_seq: u32,
    needs_ammo_stat_send: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
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
            tracing::debug!(target_eid, "apply_damage_to_target: target not found");
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

    // Seed the beta-distribution sample from this ability invocation.
    // Per-(entity, ability, effect_seq) determinism — a fresh effect_seq per
    // AoE secondary target gives independent rolls without losing replay
    // reproducibility.
    let seed = pseudo_random_seed(entity_id, ability_id, effect_seq);
    let qr_result = combat::calculate_result(qr, seed);

    // Look up damage values from the ability's effect NVPs. When the
    // ability is known but exposes no positive HealthDamage (e.g. focus
    // drains, heals, buff-only abilities) we keep health_base_damage at 0
    // so the ability doesn't accidentally read as a 15-HP physical hit.
    // The 15-HP fallback is reserved for the unknown-ability case.
    //
    // Effects with a `script_name` are collected here and dispatched after
    // the legacy NVP damage path completes (so heals/buffs see the
    // post-hit state). See `cell::effects` for the dispatcher and the
    // registered scripts.
    let mut script_effect_ids: Vec<i32> = Vec::new();
    let (health_base_damage, focus_base_damage) = if let Some(def) = ability_def {
        let mut h_dmg = 0i32;
        let mut f_dmg = 0i32;
        for &eid in &def.effect_ids {
            if let Some(effect) = space_mgr.effect_defs.get(&eid) {
                let hd = effect.param_i32("HealthDamage");
                let fd = effect.param_i32("FocusDamage");
                if hd > 0 {
                    h_dmg = hd;
                }
                if fd > 0 {
                    f_dmg = fd;
                }
                if effect.script_name.is_some() {
                    script_effect_ids.push(eid);
                }
            }
        }
        (h_dmg, f_dmg)
    } else {
        (15, 0)
    };
    // Temp: 2x player damage so players can kill NPCs before dying
    let is_player_attacker = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let health_base_damage = if is_player_attacker {
        health_base_damage * 2
    } else {
        health_base_damage
    };

    // ── `entity_health_below` pre-hit sample ──
    //
    // This is the one seam every ability-driven health mutation passes
    // through — single target, AoE secondary, cone secondary, and the
    // effect scripts dispatched at the bottom of this function. Sampling
    // here (rather than at the single-target caller, where H04 originally
    // put it) is what makes the trigger fire for all of them; see
    // `combat::damage_credit` for why a missed sample is unrecoverable
    // rather than merely late. The content-layer drain runs at the
    // caller that owns the `ChainEngine`.
    combat::note_pre_damage_health(space_mgr, entity_id, target_eid);

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
        &qr_result,
        health_base_damage,
        DT_PHYSICAL,
        HEALTH,
        &attacker_stats,
        &mut target.stats,
    );

    // Apply focus damage if present
    if focus_base_damage > 0 {
        let _ = combat::calculate_damage(
            &qr_result,
            focus_base_damage,
            DT_PHYSICAL,
            cimmeria_entity::stats::FOCUS,
            &attacker_stats,
            &mut target.stats,
        );
    }

    // Did the *direct* damage kill? The state mutations and the whole
    // death burst are deferred to `death::resolve_death` below so the
    // effect-results / stat-update packets are computed against
    // pre-death state exactly as they were before the extraction. Only
    // the threat gate needs the answer this early.
    let target_died = target.stats.get(HEALTH).is_some_and(|s| s.cur <= 0);

    // Serialize dirty stats for the target
    let target_stat_update = target.stats.serialize_dirty();
    let _target_public_stat_update = target.stats.serialize_dirty_public();
    target.stats.clear_dirty();

    // ── Send effect results ──

    // onEffectResults — send to both attacker and target, but avoid double-sending.
    // If attacker is a player and target is an NPC, the witness routing on the NPC
    // already reaches the player. So only send to the attacker directly + NPC witnesses.
    let effect_args = serialize_effect_results(
        entity_id as i32, // source
        ability_id,
        effect_seq as i32, // effect ID (using sequence as stub)
        target_eid as i32, // target
        qr_result.result_code,
        &effect_results,
    );

    let attacker_is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let target_is_player = space_mgr
        .get_entity(target_eid)
        .is_some_and(|e| e.is_player);

    // Fan out the attacker's effect results to self + all AoI witnesses so a
    // spectator sees the ability fire. For NPC attackers the self send is a
    // no-op (NPCs have no client) and this collapses to witness-only.
    send_entity_method_to_self_and_witnesses(
        entity_id,
        crate::mercury::method_idx::ON_EFFECT_RESULTS,
        effect_args.clone(),
        tx,
        space_mgr,
    )
    .await;

    // On-target effect results — fan out for any player target so witnesses
    // see the hit land on them. For NPC targets the attacker's self+witness
    // send above already carries the result (entity_id = attacker, target_eid
    // in the payload). Player targets are tracked by a different entity_id, so
    // they need a separate fanout keyed on target_eid.
    if target_is_player {
        send_entity_method_to_self_and_witnesses(
            target_eid,
            crate::mercury::method_idx::ON_EFFECT_RESULTS,
            effect_args,
            tx,
            space_mgr,
        )
        .await;
    }

    // ── Send stat updates ──

    // onStatUpdate to target — health bar changes must reach witnesses so the
    // spectator sees the health drain. Fan out to self+witnesses of the target.
    send_entity_method_to_self_and_witnesses(
        target_eid,
        crate::mercury::method_idx::ON_STAT_UPDATE,
        target_stat_update,
        tx,
        space_mgr,
    )
    .await;

    // onStatUpdate to attacker — drains AmmoSlot{N} dirty bits set by
    // `set_slot_ammo` on the consume path so the bandolier UI updates on every
    // shot. Skipped if the consume path didn't run (no ammo cost, NPC
    // attacker, or AoE secondary target where the primary already flushed).
    if needs_ammo_stat_send {
        flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
    }

    // ── Death resolution (direct damage) ──
    //
    // Everything a death entails — the kill-site state mutations, the
    // ordered wire burst, the threat drain, the death animation, kill XP,
    // and the player Defeat Window — lives in `death::resolve_death` so
    // every kill path produces the same corpse. It runs here, after the
    // effect-results / stat-update packets, exactly where the burst used
    // to be inlined.
    if target_died {
        super::death::resolve_death(
            target_eid,
            entity_id,
            Some(ability_id),
            attacker_is_player,
            // Combat kills pay XP; `resolve_death` no-ops the grant for
            // player targets.
            true,
            tx,
            space_mgr,
        )
        .await;
    }

    // Generate threat on surviving NPCs so they aggro back. If this hit
    // is what put the player into combat (their threatened_mobs went
    // from empty → {target}), broadcast the new state_field so the
    // client flips its in-combat HUD/cursor routing, and re-emit
    // BeingAppearance so the weapon visual is in the wire ComponentList
    // (Phase 2 of the holster work, and
    // `docs/architecture/state-field-bits.md`).
    //
    // **Order matters here**: send `BeingAppearance` BEFORE
    // `onStateFieldUpdate`. Both flow through `FUN_00e7b4c0`
    // (`ghidra://SGW.exe@0x00e7b4c0`) to re-key the animation blend, but
    // only the appearance path triggers `FUN_00e7b7c0` (socket
    // re-attach, `ghidra://SGW.exe@0x00e7b7c0`) and writes the
    // weapon-category byte at `+0x3d2`. If `BSF_InCombat` flips first,
    // the unholster animation starts before the weapon mesh is attached
    // — the hand reaches for the holster, grabs air, and the mesh
    // snaps in mid-animation (the "splinch" seen in playtest). Sending
    // appearance first puts the mesh at the holster socket so the draw
    // animation has something real to act on.
    if !target_died {
        if let Some(new_state) = combat::generate_threat(
            space_mgr,
            entity_id,
            target_eid,
            _total_health_damage as f32,
        ) {
            super::messaging::request_appearance_refresh(entity_id, tx, space_mgr).await;
            // BSF_InCombat flip — broadcast to self+witnesses so a spectator
            // sees the entity enter the combat stance. This is the primary
            // trigger site for the witness-fanout requirement.
            send_entity_method_to_self_and_witnesses(
                entity_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                new_state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // ── Effect scripts ──
    //
    // Dispatch any effects on this ability that have a `script_name` set.
    // Scripts run AFTER the legacy damage path so heals see the post-hit
    // state. They mutate the target via the shared `space_mgr` borrow;
    // any stat changes are flushed in a follow-up onStatUpdate so the
    // client picks up the heal/buff/debuff alongside the damage packet.
    //
    // Scripts that need wire-side fan-out (effect anims, buff icons) own
    // their own send calls — v1 only ships HealHealth / HealFocus /
    // MeleeDamage which are stat-mutation-only.
    if !script_effect_ids.is_empty() {
        for eid in &script_effect_ids {
            let effect_def = match space_mgr.effect_defs.get(eid) {
                Some(e) => e.clone(),
                None => continue,
            };
            let Some(script_name) = effect_def.script_name.clone() else {
                continue;
            };
            let mut ctx = crate::cell::effects::EffectContext {
                source_id: entity_id,
                target_id: target_eid,
                effect: &effect_def,
                space_mgr,
            };
            crate::cell::effects::dispatch_by_name(&script_name, &mut ctx);
        }
        // Flush any stat changes the scripts produced so the client sees
        // the heal/buff alongside the existing damage update.
        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
            let dirty = target.stats.serialize_dirty();
            target.stats.clear_dirty();
            if !dirty.is_empty() {
                send_entity_method(
                    target_eid,
                    crate::mercury::method_idx::ON_STAT_UPDATE,
                    dirty,
                    tx,
                    space_mgr,
                )
                .await;
            }
        }
    }

    // ── Death resolution (effect-driven) ──
    //
    // Effect scripts write HEALTH directly — `RangedPhysicalDamage`'s
    // Focus-pierce bleed, `MeleePhysicalDamage`, `RangedEnergyDamage`,
    // `Suppression` — so a shot whose *direct* damage left the target
    // standing can still take it to zero down here, long after the
    // `target_died` check above has run. Playtest 2026-09-19: pistol auto
    // attack (ability 579 / effect 641) bled MessHall_Guard1 to 0 HP, the
    // mission's `entity_dead_tag` trigger fired off the health probe in
    // `handle_use_ability_with_kill_credit`, and then nothing else
    // happened — no corpse, no loot, no XP, `ai_state` still `Fighting`.
    // The guard kept shooting back from 0 HP for 1.5 s until the player's
    // NEXT shot re-entered the direct-damage arm and finally killed it.
    //
    // Sweeping here rather than inside each script keeps the scripts pure
    // stat mutators (they hold `&mut SpaceManager` through a sync
    // `EffectContext` and cannot await the wire burst). `resolve_death` is
    // idempotent on `BSF_DEAD`, so the common case — direct damage already
    // killed — costs one entity lookup and returns.
    //
    // Unconditional rather than gated on `!script_effect_ids.is_empty()`:
    // any post-`calculate_damage` step that zeroes HEALTH should produce a
    // corpse, and a target that arrives here already at 0 HP without
    // `BSF_DEAD` is a bug we would rather fail safe on than propagate.
    if space_mgr
        .get_entity(target_eid)
        .is_some_and(|e| e.stats.get(HEALTH).is_some_and(|s| s.cur <= 0))
    {
        super::death::resolve_death(
            target_eid,
            entity_id,
            Some(ability_id),
            attacker_is_player,
            true,
            tx,
            space_mgr,
        )
        .await;
    }

    // ── Register pulsing effects ──
    //
    // Walk the ability's effects again — for any with `pulse_count > 1`
    // and a positive `pulse_duration`, register an `ActiveEffectInstance`
    // on the target. The initial pulse already fired (above, via NVP
    // damage or script dispatch); registration carries the remaining
    // pulses. See `cell::effects::pulsing::effect_pulse_tick` for the
    // per-tick fire loop.
    if let Some(def) = ability_def {
        let now = std::time::Instant::now();
        for &eid in &def.effect_ids {
            let effect_clone = match space_mgr.effect_defs.get(&eid) {
                Some(e) if e.is_pulsing() => e.clone(),
                _ => continue,
            };
            let _ = crate::cell::effects::register_active_effect(
                space_mgr,
                target_eid,
                entity_id,
                &effect_clone,
                now,
                tx,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod bleed_death_tests;
#[cfg(test)]
mod tests;
