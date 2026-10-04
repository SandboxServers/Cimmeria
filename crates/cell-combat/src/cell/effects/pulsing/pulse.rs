//! One pulse: [`pulse_one`] wraps the apply ([`fire_pulse`]) and the
//! content hooks a pulse owes ([`dot_kill_credit`], the health-below drain)
//! in the `combat.effect_tick` span and its cast's scope (AB-T1).
//! [`super::tick::effect_pulse_tick`] calls it for every due pulse.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{EffectDef, DT_PHYSICAL};
use cimmeria_entity::cell_entity::{ActiveEffectInstance, AiState};
use cimmeria_entity::stats::{FOCUS, HEALTH};

use cimmeria_cell_world::cell::combat_debug::{self, Note, Pools, PulseNote};

use crate::cell::abilities::wire_ledger::{self, WireCtx};
use crate::cell::abilities::WireRoute;
use crate::cell::content_events::ContentEvents;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// One due pulse, and the content hooks it owes, under the
/// `combat.effect_tick` span and its cast's scope (AB-T1): every row the
/// pulse causes (the pulse, a ledger entry its script applies, the kill)
/// carries the `cast_id` of the cast that registered the effect.
///
/// The span opens per fired pulse, not per tick, so an idle tick opens
/// nothing (instrumentation-discipline rule 3). It is DEBUG: a 0.1 s channel
/// pulses ten times a second, and the `pulse_ticked` row already
/// carries the same fields.
#[tracing::instrument(
    name = "combat.effect_tick",
    target = "abilities",
    level = "debug",
    skip_all,
    fields(
        target_id = target_id,
        invoker_id = inst.invoker_id,
        cast_id = inst.cast_id,
        ability_id = inst.ability_id,
        effect_id = inst.effect_id
    )
)]
pub(super) async fn pulse_one(
    target_id: u32,
    inst: &ActiveEffectInstance,
    effect_def: &EffectDef,
    events: &dyn ContentEvents,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let outer_cast = space_mgr.enter_cast_scope(inst.cast_id);
    fire_pulse(target_id, inst, effect_def, tx, space_mgr).await;
    // Death first, threshold second: `dot_kill_credit` stamps `BSF_DEAD` on
    // a mob the pulse finished, which is exactly what
    // `fire_health_below_for_hit` reads to suppress the threshold chain.
    // Running the drain first would let a lethal pulse fire
    // `entity_health_below` on its way past the band, breaking the "exactly
    // one of the two per hit" contract that the single-target path already
    // honours.
    dot_kill_credit(target_id, inst.invoker_id, events, tx, space_mgr).await;
    events.pending_health_below(tx, space_mgr).await;
    space_mgr.exit_cast_scope(outer_cast);
    // The pulse's debug lines (AB-N1), now its scope is closed.
    combat_debug::flush(tx, space_mgr).await;
}

/// Death credit for a pulse that finished the target.
///
/// A DoT kill has no other credit path: `fire_pulse` writes the HEALTH
/// stat directly, so none of `apply_damage_to_target`'s death machinery
/// runs. Before the PR #662 review a mob killed by a DoT sat at zero
/// health, never flipped to `BSF_DEAD`, dropped no loot, and never fired
/// `entity_dead_tag` — so a kill-count mission stalled if the killing
/// blow happened to be a tick rather than a shot.
///
/// Routes through the canonical
/// [`crate::cell::abilities::kill_npc_out_of_band`] so loot, threat fanout
/// and the dead-state flip land in the same protocol order a shot produces,
/// then fires `EntityDeath` for the invoker exactly as the single-target
/// wrapper does. No-ops unless the pulse actually finished a tagged,
/// live, non-player target for a player invoker.
async fn dot_kill_credit(
    target_id: u32,
    invoker_id: u32,
    events: &dyn ContentEvents,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = space_mgr.get_entity(target_id) else {
        return;
    };
    if target.is_player || crate::cell::combat::is_dead_state(target.state_field) {
        return;
    }
    // No HEALTH stat, or still above zero — the pulse wounded but did not
    // finish, so the threshold drain owns this hit, not the death path.
    //
    // This is also where a surrendered NPC exits: `fire_pulse` floors an
    // `AiState::Submit` target at 1 HP before returning, so the `cur > 0`
    // probe is the surrender guard as well as the wounded-not-killed one.
    // Deliberately not restated as a second `ai_state` check — one guard
    // that can drift is better than two.
    if target.stats.get(HEALTH).is_none_or(|s| s.cur > 0) {
        return;
    }
    let tag = target.tag.clone();

    let invoker_is_player = space_mgr
        .get_entity(invoker_id)
        .is_some_and(|e| e.is_player);
    // The kill itself is unconditional once the health check passes: an
    // NPC's DoT still has to produce a corpse, loot and a threat drain
    // even though there is no mission to credit.
    if !crate::cell::abilities::kill_npc_out_of_band(
        target_id,
        invoker_id,
        invoker_is_player,
        // A DoT finishing a mob is a combat kill like any other — the
        // invoker earns the same XP their direct shot would have.
        true,
        tx,
        space_mgr,
    )
    .await
    {
        return;
    }

    // Mission credit is player-only and tag-only, matching
    // `handle_use_ability_with_kill_credit`: a player's DoT credits the
    // player, a pet's credits its owner (pets PT-06). A tagless mob or an
    // NPC-owned DoT still died above; there is just no chain to advance.
    let Some(tag) = tag else {
        return;
    };
    let Some((credited, player_id)) =
        crate::cell::abilities::credited_player(space_mgr, invoker_id)
    else {
        return;
    };
    events
        .entity_death(credited, player_id, &tag, tx, space_mgr)
        .await;
}

/// Apply a single pulse to `target_id`. Re-dispatches the effect's
/// script if it has one, otherwise applies the legacy NVP damage path
/// (HealthDamage / FocusDamage as raw stat mutations).
async fn fire_pulse(
    target_id: u32,
    inst: &ActiveEffectInstance,
    effect: &EffectDef,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Bail on dead target — pulses on corpses shouldn't fire.
    if space_mgr
        .get_entity(target_id)
        .and_then(|e| e.stats.get(HEALTH))
        .is_some_and(|s| s.cur <= 0)
    {
        tracing::debug!(
            target: "abilities.pulse",
            event = "pulse_skipped_dead_target",
            stage = "pulse",
            account_id = inst.invoker_identity.account_id,
            player_id = inst.invoker_identity.player_id,
            entity_id = inst.invoker_id,
            target_id,
            target_player_id = space_mgr.player_identity(target_id).player_id,
            invoker_id = inst.invoker_id,
            cast_id = inst.cast_id,
            ability_id = inst.ability_id,
            effect_id = inst.effect_id,
            "Skipping pulse — target is dead"
        );
        return;
    }

    // `entity_health_below` pre-pulse sample. The second of the two
    // health-application seams (the other is `apply_damage_to_target`);
    // without it a DoT that dragged a tagged mob through its threshold
    // lost the crossing permanently, because the band predicate needs
    // `pct_before > threshold` and every later hit arrives below it. The
    // attacker is the effect's invoker, not whoever is shooting the target
    // this tick. See `combat::damage_credit`.
    crate::cell::combat::note_pre_damage_health(space_mgr, inst.invoker_id, target_id);
    // GM god mode (142): snapshot the pools, put back any loss below.
    let god_mode = crate::cell::combat::god_mode::GodModeGuard::arm(space_mgr, target_id);
    // AB-T3: the pools this pulse starts from, and how it acts, for
    // `pulse_ticked`.
    let (health_before, focus_before) = pools(space_mgr, target_id);
    let mut path = "script";
    let mut damage_type = DT_PHYSICAL;
    let mut absorbed = 0;

    // Script path takes precedence over NVP path so a registered
    // script can fully decide what happens on each pulse.
    if let Some(script_name) = effect.script_name.clone() {
        // A scripted DoT pulse passes the target's shields first (AB-10), as
        // the NVP branch does inside `calculate_damage`.
        let mut effect = effect.clone();
        if let Some(target) = space_mgr.get_entity_mut(target_id) {
            damage_type = crate::cell::combat::script_damage_type(Some(&script_name));
            absorbed = crate::cell::combat::absorb_damage_nvps(
                &mut target.stats,
                &mut effect,
                damage_type,
            );
        }
        let mut ctx = crate::cell::effects::EffectContext {
            source_id: inst.invoker_id,
            target_id,
            effect: &effect,
            space_mgr,
        };
        crate::cell::effects::dispatch_by_name(&script_name, &mut ctx);
    } else {
        // Legacy NVP path — read HealthDamage / FocusDamage and route
        // through `calculate_damage` so armor + absorption + stat
        // resistance apply per pulse. Without this routing, a target
        // with full ABSORB_PHYSICAL would still take DoT physical
        // damage as raw stat mutation — bypassing the shield mechanic.
        //
        // QR is held fixed at neutral (qr=0, qr_rand=1.0, RC_HIT) so
        // DoT pulses don't re-roll hit/crit per tick — the initial
        // cast's QR is the authoritative roll; subsequent pulses
        // deliver consistent base damage with full mitigation applied.
        let h_dmg = effect.param_i32("HealthDamage");
        let f_dmg = effect.param_i32("FocusDamage");
        let attacker_stats = space_mgr
            .get_entity(inst.invoker_id)
            .map(|e| e.stats.clone());
        // qr_rand = 0.5 cancels the internal `QR_DAMAGE_MULTIPLIER = 2.0`
        // in calculate_damage so `HealthDamage = 10` actually delivers
        // ~10 base damage per pulse (matching what content authors wrote
        // in the NVP). Without this cancellation, every DoT tick would
        // double the intended damage. qr = 0.0 zeroes the (1 + qr)
        // multiplier so the pipeline is `base × damage_bonus × (1 - resist)
        // - armor - absorption` for DoT — same shape, no QR amplification.
        let neutral_qr = crate::cell::combat::QrResult {
            qr_rand: 0.5,
            result_code: cimmeria_entity::abilities::RC_HIT,
            qr: 0.0,
        };
        // Default damage type to PHYSICAL when not otherwise specified.
        // Per-effect damage type (DT_ENERGY for staff weapons, etc.)
        // requires plumbing a damage_type column onto effects — flagged
        // as a follow-up; today's content authoring relies on the
        // attacker's weapon type rather than a per-effect override.
        let dmg_type = DT_PHYSICAL;
        path = "nvp";
        if let (Some(attacker), Some(target)) =
            (attacker_stats, space_mgr.get_entity_mut(target_id))
        {
            // Focus first, as on a hit: a partial shield spends itself on
            // the Focus half before the Health half (AB-10).
            for (amount, stat) in [(f_dmg, FOCUS), (h_dmg, HEALTH)] {
                if amount > 0 {
                    absorbed += crate::cell::combat::resolve_damage(
                        &neutral_qr,
                        amount,
                        1.0,
                        1.0,
                        dmg_type,
                        stat,
                        &attacker,
                        &mut target.stats,
                    )
                    .absorbed;
                }
            }
        } else if let Some(target) = space_mgr.get_entity_mut(target_id) {
            path = "nvp_invoker_gone";
            // Invoker vanished mid-DoT (NPC despawned, etc.). Apply
            // raw damage as a degraded fallback — better than dropping
            // the pulse entirely, which would let DoT victims survive
            // forever after their attacker died. It still passes the
            // target's shields, Focus first (AB-10).
            let (f_dmg, f_absorbed) =
                crate::cell::combat::drain_absorption_pools(&mut target.stats, dmg_type, f_dmg);
            let (h_dmg, h_absorbed) =
                crate::cell::combat::drain_absorption_pools(&mut target.stats, dmg_type, h_dmg);
            absorbed = f_absorbed + h_absorbed;
            if h_dmg > 0 {
                if let Some(stat) = target.stats.get_mut(HEALTH) {
                    let cur = stat.cur;
                    let new_cur = (cur - h_dmg).max(0);
                    stat.update(stat.min, new_cur, stat.max);
                }
            }
            if f_dmg > 0 {
                if let Some(stat) = target.stats.get_mut(FOCUS) {
                    let cur = stat.cur;
                    let new_cur = (cur - f_dmg).max(0);
                    stat.update(stat.min, new_cur, stat.max);
                }
            }
        }
    }

    // AB-10: charge what this pulse drained from the absorb stats to the
    // shields on the ledger; an emptied one comes off, and the stat-buff
    // tick sends its icon clear.
    space_mgr.settle_absorb_shields(target_id);
    let mut god_mode_restored = crate::cell::combat::god_mode::Absorbed::default();
    if let Some(guard) = &god_mode {
        let source = crate::cell::combat::god_mode::DamageSource {
            source_id: inst.invoker_id,
            ability_id: Some(inst.ability_id),
            effect_id: Some(inst.effect_id),
            seam: "effect_pulse",
        };
        god_mode_restored = guard.restore(space_mgr, source);
    }

    // Surrender floor: an automatic damage source may wound a
    // surrendered NPC but may never finish it. See
    // `docs/architecture/abilities-and-effects-system.md` decision 18.
    //
    // A pulse is an *automatic* damage path by the same definition
    // `is_auto_cycle_target_valid` uses: the deliberate act was applying
    // the effect, and everything after it is the server re-delivering
    // damage on its own cadence. Harset H08 stops the auto-cycle loop
    // from killing a surrendered NPC; without this, a DoT the player
    // applied *before* the surrender walks the NPC to zero seconds
    // later and `dot_kill_credit` produces a corpse — exactly the
    // outcome the surrender exists to prevent, just on a different
    // clock. (Before PR #662's R1 fix a lethal pulse left the mob alive
    // at 0 HP, so this hazard did not exist when H08 was specified.)
    //
    // Clamped here, after both the script and NVP branches, rather than
    // in `dot_kill_credit`: the dirty flush below is the same pulse's
    // stat broadcast, so the client is told `1` and never sees a `0`
    // frame. The clamp also makes `dot_kill_credit`'s `cur > 0` probe
    // early-out on its own, so there is one guard, not two.
    //
    // Deliberately NOT a full immunity: the pulse still lands its
    // damage, and a single deliberate shot still kills a surrendered
    // NPC (H08 records explicit-attack behaviour rather than changing
    // it). Only the killing blow from a self-repeating source is
    // refused.
    // Read before the mutable borrow below (rule 5: the subject's id).
    let target_player_id = space_mgr.player_identity(target_id).player_id;
    if let Some(target) = space_mgr.get_entity_mut(target_id) {
        if !target.is_player && target.ai_state() == AiState::Submit {
            if let Some(stat) = target.stats.get_mut(HEALTH) {
                if stat.cur <= 0 {
                    stat.update(stat.min, 1, stat.max);
                    tracing::debug!(
                        target: "abilities.pulse",
                        event = "pulse_surrender_floor",
                        stage = "pulse",
                        account_id = inst.invoker_identity.account_id,
                        player_id = inst.invoker_identity.player_id,
                        entity_id = inst.invoker_id,
                        target_id,
                        target_player_id,
                        cast_id = inst.cast_id,
                        ability_id = inst.ability_id,
                        effect_id = inst.effect_id,
                        invoker_id = inst.invoker_id,
                        "Pulse would have killed a surrendered NPC -- health floored at 1"
                    );
                }
            }
        }
    }

    // D-SS20: a pulse from the duel partner never kills a duelist. It runs
    // after both branches (script and NVP, and the invoker-gone fallback,
    // which cannot match: the partner leaving ends the duel) and before the
    // flush, so the client is told 1, never 0. `pulse_ticked` below
    // still logs the pulse; the duel ends after the flush.
    let duel_clamp = cimmeria_cell_world::cell::duel::clamp_partner_lethal(
        space_mgr,
        inst.invoker_id,
        target_id,
        cimmeria_cell_world::cell::duel::ClampSource {
            path: "effect_pulse",
            ability_id: Some(inst.ability_id),
            effect_id: Some(inst.effect_id),
        },
    );

    // Flush any stat changes the pulse produced so the client renders
    // the bar update. Pulses don't generate effect-results packets in
    // v1 — that's a wire-format addition tracked alongside per-pulse
    // tick observability in a follow-up.
    let dirty = space_mgr.get_entity_mut(target_id).map(|t| {
        let d = t.stats.serialize_dirty();
        t.stats.clear_dirty();
        d
    });
    if let Some(bytes) = dirty {
        if !bytes.is_empty() {
            wire_ledger::send(
                target_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                bytes,
                WireRoute::EntityDefault,
                WireCtx::new("pulse")
                    .cast(inst.cast_id)
                    .ability(inst.ability_id),
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // The registration's snapshot: the invoker may be gone, or its entity id
    // reused by another player, by now (rule 5). AB-T3 renamed this row
    // from `effect_pulse_fired` and added the amounts and pools.
    let who = inst.invoker_identity;
    let (health_after, focus_after) = pools(space_mgr, target_id);
    tracing::debug!(
        target: "abilities.pulse",
        event = "pulse_ticked",
        stage = "pulse",
        account_id = who.account_id,
        player_id = who.player_id,
        entity_id = inst.invoker_id,
        target_id,
        target_player_id = space_mgr.player_identity(target_id).player_id,
        invoker_id = inst.invoker_id,
        cast_id = inst.cast_id,
        effect_id = inst.effect_id,
        ability_id = inst.ability_id,
        path,
        script = effect.script_name.as_deref(),
        health_amount = effect.param_i32("HealthDamage"),
        focus_amount = effect.param_i32("FocusDamage"),
        damage_type,
        absorbed,
        health_before,
        health_after,
        focus_before,
        focus_after,
        // `health_after` / `focus_after` are read after the god-mode restore,
        // so they are what the target kept; these say what was put back.
        god_mode = god_mode.is_some(),
        god_mode_restored_health = god_mode_restored.health,
        god_mode_restored_focus = god_mode_restored.focus,
        remaining_before_decrement = inst.remaining_pulses,
        "effect pulse ticked"
    );
    // The same values for the in-game combat debug (AB-N1).
    space_mgr.combat_debug.note(
        inst.invoker_id,
        inst.invoker_identity,
        inst.cast_id,
        inst.ability_id,
        Note::Pulse(PulseNote {
            target_id,
            effect_id: inst.effect_id,
            path,
            before: Pools {
                health: health_before,
                focus: focus_before,
            },
            after: Pools {
                health: health_after,
                focus: focus_after,
            },
            remaining: inst.remaining_pulses,
        }),
    );

    // The end strips this effect and every other one the partner applied,
    // so the tick's loop skips any of them still due (`still_active`).
    if let Some(hit) = duel_clamp {
        cimmeria_cell_world::cell::duel::finish_clamped(tx, space_mgr, hit).await;
    }
}

/// The target's HEALTH and FOCUS (0 when it is gone).
fn pools(space_mgr: &SpaceManager, target_id: u32) -> (i32, i32) {
    space_mgr.get_entity(target_id).map_or((0, 0), |e| {
        let cur = |id| e.stats.get(id).map_or(0, |s| s.cur);
        (cur(HEALTH), cur(FOCUS))
    })
}
