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

use cimmeria_entity::abilities::{serialize_effect_results, AbilityDef, DT_PHYSICAL, RC_MISS};
use cimmeria_entity::stats::HEALTH;

use super::super::combat;
use super::super::combat::god_mode::{DamageSource, GodModeGuard};
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use cimmeria_cell_world::cell::duel;
use cimmeria_cell_world::cell::effects::{ammo_damage, ammo_explosive};

use super::messaging::{
    flush_attacker_ammo_stat, send_entity_method, send_entity_method_to_self_and_witnesses,
};
use super::rng::pseudo_random_seed;
use ammo_splash::HitKind;
use duel_gate::{clamp_source, player_hit_refusal};
pub(in crate::cell::abilities) use effect_scripts::is_damage_script;
use hit_ids::HitIds;

/// Resolve damage from `entity_id` to `target_eid` for ability `ability_id`.
///
/// Performs:
///   - Snapshot attacker stats; bail if attacker missing.
///   - Compute QR + roll a hit result for this attacker/target pair, with
///     the attacker's cover QR (no roll when every effect carries
///     `EF_DontUseQR`, [`qr_gate`]), and scale the damage by the
///     defender's cover reduction (NA32).
///   - Sort the effects into damage paths ([`effect_scripts`]): a damage
///     script is its effect's only damage; NVP damage for the rest; a
///     missed QR-rolled effect lands nothing (AB-06, D-AB07).
///   - Apply each effect's health/focus damage to the target on its own,
///     a `DontUseQR` effect at its base ([`nvp_damage`], AB-03).
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
    apply_hit(
        entity_id,
        target_eid,
        ability_id,
        ability_def,
        effect_seq,
        needs_ammo_stat_send,
        HitKind::Direct,
        tx,
        space_mgr,
    )
    .await;
}

/// [`apply_damage_to_target`] for one [`HitKind`]. An explosive round's
/// splash targets (AM-10, [`ammo_splash`]) come back through here as
/// `HitKind::Splash`, which scales the damage and never splashes again.
async fn apply_hit(
    entity_id: u32,
    target_eid: u32,
    ability_id: i32,
    ability_def: &Option<AbilityDef>,
    effect_seq: u32,
    needs_ammo_stat_send: bool,
    kind: HitKind,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // The harm gate again, at apply time (SS-D3 review). The launch and the
    // collectors check it, but a multi-hit ability collects every cone's
    // targets up front and then applies them in turn: the first cone's hit
    // can end a duel (the non-lethal clamp) before the second lands, and
    // nothing else would stop the second from killing the ex-partner as a
    // normal death. Re-checking here closes the class (cones, AoE
    // secondaries, any future multi-hit). Player on player only: a self-cast
    // and every NPC path are unchanged.
    if let Some(reason) = player_hit_refusal(space_mgr, entity_id, target_eid) {
        let (a, t) = (
            space_mgr.player_identity(entity_id),
            space_mgr.player_identity(target_eid),
        );
        tracing::debug!(
            target: "duel",
            event = "duel.hit_refused",
            account_id = a.account_id,
            player_id = a.player_id,
            entity_id,
            target_player_id = t.player_id,
            target_entity_id = target_eid,
            ability_id,
            reason,
            "player-on-player damage refused at apply time: not an engaged duel pair"
        );
        if needs_ammo_stat_send {
            flush_attacker_ammo_stat(entity_id, tx, space_mgr).await;
        }
        return;
    }

    // We need both attacker and target stats. Since we can't borrow two
    // entities mutably at once, we snapshot the attacker stats first.
    let attacker_stats = match space_mgr.get_entity(entity_id) {
        Some(e) => e.stats.clone(),
        None => {
            // Defensive: entity vanished after the consume mutation. Flush
            // before exiting so the client still sees the ammo decrement.
            silent_rows::hit_gone(space_mgr, entity_id, target_eid, ability_id, "caster_gone");
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
            silent_rows::hit_gone(space_mgr, entity_id, target_eid, ability_id, "target_gone");
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
    // Cover (NA32, D-NA15a, `cover_roll`): a damage reduction rated by the
    // defender's node, only when the node faces the attacker; the
    // attacker's own `coverQRModifier` behind cover is a QR term.
    let cover = cover_roll::resolve_cover(
        space_mgr,
        entity_id,
        target_eid,
        &attacker_stats,
        &target.stats,
    );
    let qr =
        combat::calculate_qr(&attacker_stats, &target.stats, ability_is_ranged) + cover.attacker_qr;
    let cover_scale = cover.reduction.damage_scale();

    // Seed the beta-distribution sample from this ability invocation.
    // Per-(entity, ability, effect_seq) determinism — a fresh effect_seq per
    // AoE secondary target gives independent rolls without losing replay
    // reproducibility.
    // An ability whose every effect carries `EF_DontUseQR` takes no roll
    // (AB-06, `qr_gate`).
    let seed = pseudo_random_seed(entity_id, ability_id, effect_seq);
    let ids = HitIds {
        entity_id,
        target_eid,
        ability_id,
        actor: space_mgr.player_identity(entity_id),
        target: space_mgr.player_identity(target_eid),
        cast_id: space_mgr.current_cast_id(),
        god_mode: space_mgr.get_entity(target_eid).is_some_and(|e| e.god_mode),
    };
    let qr_result = qr_gate::roll_hit(ability_def.as_ref(), space_mgr, qr, seed, cover, ids);

    // Special ammo (AM-04, D-AM07): a player's weapon shot with a modified
    // ammo type loaded scales its damage, divides the armour by its
    // penetration, may change its damage type, and runs an on-hit effect.
    // `None` (flag off, NPC, default ammo, not a shot) is the old pipeline.
    let shot = ammo_damage::shot_ammo(
        space_mgr,
        entity_id,
        ability_def.as_ref(),
        cimmeria_entity::ammo_feature::finite_special(),
    );
    let (ammo_scale, penetration_mult, damage_type) = shot.map_or((1.0, 1.0, DT_PHYSICAL), |s| {
        (
            s.damage_scale(),
            s.penetration_mult(),
            s.damage_type(DT_PHYSICAL),
        )
    });
    // A splash target (AM-10) runs no on-hit effect: that is what stops a
    // splash from splashing again. A beneficial row's heal or cleanse never
    // lands through this pipeline (AM-11d): the single-target fire sends a
    // support shot down `use_ability::support_shot`, so a beneficial shot
    // that reaches this function (an AoE or cone secondary) is at a hostile.
    let on_hit_effect_id = shot
        .filter(|s| !s.modifier.beneficial)
        .filter(|_| qr_result.result_code != RC_MISS && kind.is_direct())
        .and_then(|s| s.on_hit_effect_id(space_mgr));
    let splash = on_hit_effect_id
        .and_then(|id| space_mgr.effect_defs.get(&id))
        .and_then(ammo_explosive::splash_of);
    // Scales both damage components below: cover, then the ammo row, then
    // a splash target's share.
    let damage_scale = cover_scale * ammo_scale * kind.damage_scale();

    // Sort the effects into the hit's damage paths (AB-06, D-AB07): NVP
    // damage per effect for effects with no damage script (AB-03,
    // `nvp_damage`; 15 HP for an unknown ability), damage scripts as the
    // only damage of their effects, and every other script for after the
    // hit. A missed QR-rolled effect lands in none of them.
    let plan = effect_scripts::plan_hit_effects(
        space_mgr,
        ability_def.as_ref(),
        on_hit_effect_id,
        kind.is_direct(),
        qr_result.result_code,
        ids,
    );
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
    // Player targets only: the pools before the hit, for the `vitals`
    // `damage_taken` row logged once the hit (and any duel clamp) landed.
    let vitals_before = combat::vitals::player_snapshot(space_mgr, target_eid);
    // GM god mode (142): snapshot the pools, put back any loss below.
    let god_mode = GodModeGuard::arm(space_mgr, target_eid);

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

    // Each effect's NVP damage on its own, at its own QR (AB-03).
    let (mut effect_results, mut total_health_damage) = nvp_damage::apply_nvp_damage(
        &plan.nvp,
        &qr_result,
        damage_scale,
        penetration_mult,
        damage_type,
        &attacker_stats,
        &mut target.stats,
        ids,
    );

    // The damage scripts' damage, at the same point and the same scale as
    // the NVP damage, so the death check, `onEffectResults` and the threat
    // below all see it.
    let (script_results, script_health_damage) = effect_scripts::apply_damage_scripts(
        space_mgr,
        ids,
        &plan.damage_scripts,
        damage_scale,
        damage_type,
    );
    effect_results.extend(script_results);
    total_health_damage += script_health_damage;
    // AB-10: charge what the hit drained from the absorb stats to the
    // shields on the ledger; an emptied one comes off (icon clear below).
    space_mgr.settle_absorb_shields(target_eid);
    if let Some(guard) = &god_mode {
        let source = DamageSource::hit(entity_id, ability_id, "ability_hit");
        if guard.restore_hit(space_mgr, source, &mut effect_results) {
            total_health_damage = 0;
        }
    }

    if let Some(shot) = &shot {
        ammo_damage::log_applied(
            space_mgr,
            shot,
            entity_id,
            target_eid,
            ability_id,
            damage_type,
            total_health_damage,
            on_hit_effect_id,
        );
    }

    // D-SS20: a duel partner's hit never kills. HEALTH at or below 0 is
    // held at 1 here, before `target_died` reads it and before the stat
    // flush, so the client sees 1 and nothing below reaches the death path.
    // The duel ends at the bottom of this function, after every other
    // step of this resolution (a script bleed, a registered DoT) has been
    // clamped or registered, so the end strips them too.
    let mut duel_clamp = duel::clamp_partner_lethal(
        space_mgr,
        entity_id,
        target_eid,
        clamp_source("ability", ability_id),
    );
    if let Some(before) = vitals_before {
        combat::vitals::log_damage_taken(
            space_mgr,
            target_eid,
            entity_id,
            ability_id,
            qr_result.result_code,
            before,
        );
    }
    let Some(target) = space_mgr.get_entity_mut(target_eid) else {
        // Cannot happen (the target was just written), but a held hit must
        // still end its duel.
        if let Some(hit) = duel_clamp {
            duel::finish_clamped(tx, space_mgr, hit).await;
        }
        return;
    };

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
            total_health_damage as f32,
            crate::cell::combat::AggroCause::Damage,
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

    // ── Effect scripts (after the hit) ──
    //
    // Every landing script that is not a damage script (those ran with the
    // direct damage above) runs here, so a heal sees the post-hit state.
    // Their stat changes are flushed in a follow-up onStatUpdate. Scripts
    // that need wire-side fan-out own their own sends.
    if !plan.after_scripts.is_empty() {
        effect_scripts::run_scripts(space_mgr, ids, &plan.after_scripts, damage_type);
        space_mgr.settle_absorb_shields(target_eid);
        if let Some(guard) = &god_mode {
            guard.restore(
                space_mgr,
                DamageSource::hit(entity_id, ability_id, "ability_script"),
            );
        }
        // D-SS20 again: a script's own HEALTH write (a bleed) from the
        // partner is held at 1 too, before the flush below and before the
        // effect-driven death sweep reads it.
        duel_clamp = duel_clamp.or(duel::clamp_partner_lethal(
            space_mgr,
            entity_id,
            target_eid,
            clamp_source("ability_script", ability_id),
        ));
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
    // An after-hit script can write HEALTH directly (`Suppression`, a
    // special round's on-hit burn), so a shot whose direct damage left the
    // target standing can still take it to zero down here, after the
    // `target_died` check above has run. Before AB-06 the damage scripts
    // (`RangedPhysicalDamage`'s Focus-pierce bleed and its siblings) ran
    // here too; they now run with the direct damage. Playtest 2026-09-19: pistol auto
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
    // Unconditional rather than gated on `!plan.after_scripts.is_empty()`:
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
    // per-tick fire loop. Not on a splash target (blast damage only), and
    // not for a QR-rolled effect the roll missed (AB-06: a missed DoT
    // must not tick; `plan_hit_effects` logged the skip).
    if let Some(def) = ability_def.as_ref().filter(|_| kind.is_direct()) {
        let now = std::time::Instant::now();
        for eid in def.effect_ids.iter().copied().chain(on_hit_effect_id) {
            let effect_clone = match space_mgr.effect_defs.get(&eid) {
                Some(e) if e.is_pulsing() && qr_gate::effect_lands(e, qr_result.result_code) => {
                    e.clone()
                }
                // A missing def was logged by `plan_hit_effects`.
                _ => continue,
            };
            // `register_active_effect` logs each refusal itself (AB-T2).
            crate::cell::effects::register_active_effect(
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
    // AB-04: a landed single-pulse debuff ran its ledger script (`TimedStat`)
    // with the miss-gated after-hit scripts; its icon goes out with these.
    let now = std::time::Instant::now();
    crate::cell::effects::flush_stat_buff_timers(target_eid, now, tx, space_mgr).await;

    // ── Duel end (non-lethal, D-SS20) ──
    //
    // Last, so the end strips every effect this resolution registered on
    // the loser: a DoT registered above must not outlive the duel.
    if let Some(hit) = duel_clamp {
        duel::finish_clamped(tx, space_mgr, hit).await;
    }

    // ── Explosive splash (AM-10) ──
    //
    // After the target's own hit has fully resolved (its death, threat and
    // effects), so the splash reads the settled world. `splash` is `None`
    // for a splash target, which is the no-chaining guarantee.
    if let Some(splash) = splash {
        ammo_splash::apply_splash(
            entity_id,
            target_eid,
            ability_id,
            ability_def,
            splash,
            tx,
            space_mgr,
        )
        .await;
    }
}

mod ammo_splash;
mod cover_roll;
mod duel_gate;
mod effect_scripts;
mod hit_ids;
mod nvp_damage;
mod qr_gate;
mod silent_rows;

#[cfg(test)]
mod aggro_cause_tests;
#[cfg(test)]
mod ammo_dart_cc_tests;
#[cfg(test)]
mod ammo_dart_tech_tests;
#[cfg(test)]
mod ammo_emp_tests;
#[cfg(test)]
mod ammo_incendiary_tests;
#[cfg(test)]
mod ammo_splash_tests;
#[cfg(test)]
mod ammo_support_tests;
#[cfg(test)]
mod ammo_tests;
#[cfg(test)]
mod bleed_death_tests;
#[cfg(test)]
mod cover_tests;
#[cfg(test)]
mod damage_seed_live_db_tests;
#[cfg(test)]
mod decision_rows_tests;
#[cfg(test)]
mod god_mode_tests;
#[cfg(test)]
mod per_effect_damage_tests;
#[cfg(test)]
mod shield_tests;
#[cfg(test)]
mod single_damage_path_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timed_effect_tests;
#[cfg(test)]
mod vitals_tests;
