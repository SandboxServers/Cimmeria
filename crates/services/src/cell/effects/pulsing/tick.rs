//! The per-cell pulse scheduler and the single-pulse apply path.
//!
//! [`effect_pulse_tick`] runs at the cell's AoI cadence (100ms), walks
//! every entity's active-effect list, fires due pulses via [`fire_pulse`],
//! decrements `remaining_pulses`, and sweeps expired instances.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, EffectDef, DT_PHYSICAL, TIMER_DURATION_EFFECT,
};
use cimmeria_entity::cell_entity::ActiveEffectInstance;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::abilities::send_entity_method;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Per-cell tick — fire any due pulses on any entity's active-effect
/// list. Runs at the cell's AoI cadence (100ms) so pulse intervals
/// down to 0.1s resolve correctly.
///
/// For each entity with active effects:
///   1. Take a snapshot of indices of effects whose `next_pulse_at`
///      has elapsed.
///   2. For each due effect, clone enough state to fire (effect def,
///      invoker), drop the borrow, fire the pulse, re-acquire to
///      decrement `remaining_pulses` / reschedule `next_pulse_at`.
///   3. Sweep removed instances (remaining_pulses == 0) after the
///      pulse-fire loop completes.
pub async fn effect_pulse_tick(tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &mut SpaceManager) {
    let now = Instant::now();

    // Snapshot entities with at least one active effect so we don't
    // hold a borrow on `space_mgr` across the await. Walks every
    // entity (not just players + NPCs) so DoTs on turrets, destructibles,
    // and any future entity types fire correctly.
    let entities_with_effects: Vec<u32> = space_mgr
        .all_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| !e.active_effects.is_empty())
        })
        .collect();

    for entity_id in entities_with_effects {
        // Identify which effects are due and clone enough to fire
        // without holding the borrow across awaits.
        //
        // **Key by (effect_id, invoker_id) rather than Vec index.** The
        // earlier version re-acquired the entity by Vec index after the
        // await — but `cancel_channels_from_attacker` / `_for_invoker_ability`
        // can `retain()` between the await and the re-acquire, which
        // shifts indices and lands the post-pulse decrement on the wrong
        // effect. Keying by the instance's stable identity tuple avoids
        // the race.
        let due: Vec<(ActiveEffectInstance, EffectDef)> = {
            let Some(entity) = space_mgr.get_entity(entity_id) else {
                continue;
            };
            entity
                .active_effects
                .iter()
                .filter(|inst| inst.next_pulse_at <= now && inst.remaining_pulses > 0)
                .filter_map(|inst| {
                    space_mgr
                        .effect_defs
                        .get(&inst.effect_id)
                        .cloned()
                        .map(|def| (inst.clone(), def))
                })
                .collect()
        };

        if due.is_empty() {
            continue;
        }

        // Fire each due pulse. We re-look up the entity every iteration
        // because between awaits another tick could mutate.
        for (inst, effect_def) in &due {
            fire_pulse(entity_id, inst, effect_def, tx, space_mgr).await;
            // Update schedule + decrement on the matching instance,
            // located by (effect_id, invoker_id) — index would be unsafe.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(active) = entity
                    .active_effects
                    .iter_mut()
                    .find(|a| a.effect_id == inst.effect_id && a.invoker_id == inst.invoker_id)
                {
                    active.remaining_pulses = active.remaining_pulses.saturating_sub(1);
                    if active.remaining_pulses > 0 {
                        active.next_pulse_at =
                            now + std::time::Duration::from_secs_f32(active.pulse_interval_secs);
                    }
                }
            }
        }

        // Sweep completed instances after the fire loop, sending
        // `onTimerUpdate` with `total_time = 0` so the client clears
        // the buff/debuff icon. Without the clear, the icon would
        // stick at "expiring in 0s" forever.
        let cleared_ids: Vec<(i32, u32)> = {
            let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
                continue;
            };
            let mut cleared = Vec::new();
            entity.active_effects.retain(|inst| {
                if inst.remaining_pulses <= 0 {
                    cleared.push((inst.effect_id, inst.invoker_id));
                    false
                } else {
                    true
                }
            });
            cleared
        };
        for (cleared_effect, invoker) in cleared_ids {
            // Phase I: script on_remove first so stateful effects (Stun
            // clearing BSF_MOVEMENT_LOCK, AbsorbShield draining residual
            // pool) get their cleanup. The effect-def lookup needs to
            // happen here because the active-effect Vec only stores
            // effect_id, not the full def.
            if let Some(effect_def) = space_mgr.effect_defs.get(&cleared_effect).cloned() {
                if let Some(script_name) = effect_def.script_name.clone() {
                    let mut ctx = crate::cell::effects::EffectContext {
                        source_id: invoker,
                        target_id: entity_id,
                        effect: &effect_def,
                        space_mgr,
                    };
                    crate::cell::effects::dispatch_on_remove(&script_name, &mut ctx);
                }
                // Flush any stat dirty bits the on_remove produced (e.g.
                // ABSORB_PHYSICAL drained by AbsorbShield::on_remove) so
                // the client picks up the change.
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    let dirty = entity.stats.serialize_dirty();
                    entity.stats.clear_dirty();
                    if !dirty.is_empty() {
                        send_entity_method(
                            entity_id,
                            crate::mercury::method_idx::ON_STAT_UPDATE,
                            dirty,
                            tx,
                            space_mgr,
                        )
                        .await;
                    }
                }
            }
            let zero_timer = serialize_timer_update(
                cleared_effect,
                TIMER_DURATION_EFFECT,
                invoker as i32,
                0.0,
                0.0,
            );
            send_entity_method(entity_id, ON_TIMER_UPDATE, zero_timer, tx, space_mgr).await;
        }
    }
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
            target: "abilities",
            event = "pulse_skipped_dead_target",
            target_id,
            effect_id = inst.effect_id,
            "Skipping pulse — target is dead"
        );
        return;
    }

    // Script path takes precedence over NVP path so a registered
    // script can fully decide what happens on each pulse.
    if let Some(script_name) = effect.script_name.clone() {
        let mut ctx = crate::cell::effects::EffectContext {
            source_id: inst.invoker_id,
            target_id,
            effect,
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
        let dmg_type = cimmeria_entity::abilities::DT_PHYSICAL;
        if let (Some(attacker), Some(target)) =
            (attacker_stats, space_mgr.get_entity_mut(target_id))
        {
            if h_dmg > 0 {
                let _ = crate::cell::combat::calculate_damage(
                    &neutral_qr,
                    h_dmg,
                    dmg_type,
                    HEALTH,
                    &attacker,
                    &mut target.stats,
                );
            }
            if f_dmg > 0 {
                let _ = crate::cell::combat::calculate_damage(
                    &neutral_qr,
                    f_dmg,
                    dmg_type,
                    FOCUS,
                    &attacker,
                    &mut target.stats,
                );
            }
        } else if let Some(target) = space_mgr.get_entity_mut(target_id) {
            // Invoker vanished mid-DoT (NPC despawned, etc.). Apply
            // raw damage as a degraded fallback — better than dropping
            // the pulse entirely, which would let DoT victims survive
            // forever after their attacker died.
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
            send_entity_method(
                target_id,
                crate::mercury::method_idx::ON_STAT_UPDATE,
                bytes,
                tx,
                space_mgr,
            )
            .await;
        }
    }

    tracing::debug!(
        target: "abilities",
        event = "effect_pulse_fired",
        target_id,
        invoker_id = inst.invoker_id,
        effect_id = inst.effect_id,
        ability_id = inst.ability_id,
        damage_type = DT_PHYSICAL,
        remaining_before_decrement = inst.remaining_pulses,
        "Effect pulse fired"
    );
}
