//! Active-effect pulsing for DoT, HoT, and timed-buff effects (#47, #419).
//!
//! When an effect's `pulse_count > 1`, the initial apply in
//! `damage_apply` fires the first pulse and then registers an
//! [`ActiveEffectInstance`] on the target. A per-cell tick walks every
//! entity's active-effect list, fires due pulses, decrements
//! `remaining_pulses`, and removes the instance when it hits zero.
//!
//! ## What a "pulse" does
//!
//! - If the effect has a `script_name`, re-dispatches the script (the
//!   script decides whether to heal again, do more damage, refresh a
//!   buff, etc.).
//! - Else, applies the legacy `HealthDamage` / `FocusDamage` NVPs as
//!   raw stat mutations (no QR roll on subsequent pulses — DoT/HoT use
//!   the QR result from the initial cast for replay determinism).
//! - Sends a follow-up `onStatUpdate` to the target so the client
//!   updates their stat bars.
//!
//! ## v1 scope notes
//!
//! - **Stacking** (Phase E): same-source = refresh, multi-source = stack.
//!   See `register_active_effect` for the dedup logic.
//! - **Channelled effects** (Phase J): `pulse_count == 0` is treated as
//!   "channel until cancelled" by registering with a safety cap
//!   ([`MAX_CHANNEL_PULSES`]) and cancelling proactively via
//!   [`cancel_channels_from_attacker`] when the channeller fires a
//!   different ability, dies, or the safety cap elapses. Movement-based
//!   interrupt is a follow-up — needs per-ability `AF_INTERRUPT_ON_MOVE`
//!   data the canonical seed doesn't carry yet.
//! - **Dead-target pulse** is a no-op (the per-pulse guard returns
//!   without applying), but the instance still ages out via the schedule.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, EffectDef, AF_CHANNEL_ALLOWS_MOVEMENT, DT_PHYSICAL,
    TIMER_DURATION_EFFECT,
};
use cimmeria_entity::cell_entity::ActiveEffectInstance;
use cimmeria_entity::stats::{FOCUS, HEALTH};

use crate::cell::abilities::send_entity_method;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Maximum pulses for a `pulse_count == 0` channelled effect. Acts as
/// a safety cap so a missed cancellation event can't leave a channel
/// running forever. At a typical `pulse_duration = 0.5s`, this caps a
/// channel at 30 seconds — well past any reasonable in-game channel
/// duration (Sustained Sweep is 10s / 20 pulses, the longest in seed).
const MAX_CHANNEL_PULSES: i32 = 60;

/// Distance (world units / metres) the channeller can drift from their
/// channel-start position before the channel interrupts. Tuned for
/// typical movement: walking is ~3 m/s, so a 0.5m budget catches step-
/// off-the-spot intent within ~150ms of a player input without firing
/// on the tiny position jitter from server-side smoothing.
///
/// Per-ability override: set `AF_CHANNEL_ALLOWS_MOVEMENT` on the
/// `AbilityDef.flags` and the interrupt sweep skips that channel.
const CHANNEL_INTERRUPT_DISTANCE: f32 = 0.5;

/// Register a pulsing effect on a target. Called once per effect right
/// after the initial pulse fires in `damage_apply`. Returns `true` if
/// an instance was registered (effect was actually pulsing), `false`
/// for single-shot effects.
///
/// Implements **same-source refresh** stacking semantics (Phase E):
/// if `target_id` already has an active instance with the same
/// `effect_id` AND `invoker_id`, the existing instance is refreshed
/// (remaining_pulses + next_pulse_at restored) instead of stacking a
/// duplicate. Different invokers each get their own instance (multi-
/// source stack). Matches the Python `AbilityManager.addEffect`
/// behavior for DoT/HoT — re-applying the same caster's bleed
/// refreshes the duration, two different casters' bleeds both tick.
///
/// `now` is passed in so tests can use a fixed instant without driving
/// the wall clock. Production callers pass `Instant::now()`.
///
/// When the effect is registered (or refreshed), sends an
/// `onTimerUpdate(TIMER_DURATION_EFFECT)` so the client renders the
/// buff/debuff icon with a duration countdown. The clear-on-expiry
/// timer comes from `effect_pulse_tick` when remaining_pulses hits 0.
pub async fn register_active_effect(
    space_mgr: &mut SpaceManager,
    target_id: u32,
    invoker_id: u32,
    effect: &EffectDef,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> bool {
    if !effect.is_pulsing() {
        return false;
    }
    // pulse_count == 0 → channelled. Register with a safety-cap of
    // MAX_CHANNEL_PULSES; cancellation comes from
    // `cancel_channels_from_attacker` (different ability fire, death).
    // Without a cancellation event, the safety cap still terminates
    // the channel after ~30s at a 0.5s pulse cadence.
    let total_pulses = if effect.pulse_count == 0 {
        MAX_CHANNEL_PULSES
    } else {
        effect.pulse_count
    };
    // remaining_pulses = total - 1 (the initial pulse already fired).
    let remaining = total_pulses - 1;
    if remaining <= 0 {
        return false;
    }
    let pulse_secs = effect.pulse_duration.max(0.1);
    let next_at = now + std::time::Duration::from_secs_f32(pulse_secs);

    // Stash invoker's current position for channels so the per-tick
    // interrupt sweep can diff against it. Finite DoTs leave this `None`
    // because they don't cancel on caster movement. Snapshot taken BEFORE
    // we acquire the mutable target borrow to avoid double-borrow.
    let invoker_position_for_channel = if effect.pulse_count == 0 {
        space_mgr.get_entity(invoker_id).map(|e| e.position)
    } else {
        None
    };

    let was_refresh = {
        let Some(target) = space_mgr.get_entity_mut(target_id) else {
            return false;
        };

        // Same-source refresh: re-applying the SAME effect from the SAME
        // invoker refreshes the existing instance instead of stacking.
        // Different invoker = stack (separate instance). This matches the
        // Python reference and prevents trivial DoT stacking from a single
        // attacker spam-casting the same bleed.
        if let Some(existing) = target
            .active_effects
            .iter_mut()
            .find(|i| i.effect_id == effect.effect_id && i.invoker_id == invoker_id)
        {
            existing.remaining_pulses = remaining;
            existing.next_pulse_at = next_at;
            existing.pulse_interval_secs = pulse_secs;
            // Refresh also re-anchors the channel-start position so
            // standing still after a re-channel doesn't trigger interrupt.
            existing.invoker_position_at_register = invoker_position_for_channel;
            true
        } else {
            target.active_effects.push(ActiveEffectInstance {
                effect_id: effect.effect_id,
                ability_id: effect.ability_id,
                invoker_id,
                remaining_pulses: remaining,
                total_pulses,
                next_pulse_at: next_at,
                pulse_interval_secs: pulse_secs,
                invoker_position_at_register: invoker_position_for_channel,
            });
            false
        }
    };

    tracing::info!(
        target: "abilities",
        event = if was_refresh { "active_effect_refreshed" } else { "active_effect_registered" },
        target_id,
        invoker_id,
        effect_id = effect.effect_id,
        ability_id = effect.ability_id,
        remaining_pulses = remaining,
        total_pulses,
        pulse_interval = pulse_secs,
        is_channeled = effect.pulse_count == 0,
        "Active effect registered/refreshed"
    );

    // Send onTimerUpdate(TIMER_DURATION_EFFECT) so the client renders a
    // buff/debuff icon with the duration countdown. `effect_id` is the
    // packet's `id` field — the client uses it to correlate with the
    // clear packet when the effect expires.
    let total_time = effect.total_duration();
    let timer_bytes = serialize_timer_update(
        effect.effect_id,
        TIMER_DURATION_EFFECT,
        invoker_id as i32,
        total_time,
        total_time,
    );
    send_entity_method(target_id, ON_TIMER_UPDATE, timer_bytes, tx, space_mgr).await;

    true
}

/// Cancel every channelled effect (effect def `pulse_count == 0`) that
/// was sourced by `attacker_id`, optionally excluding effects whose
/// `ability_id` matches `keep_ability_id` (used by the same-ability
/// re-fire path to refresh rather than cancel).
///
/// Used by:
///   - `handle_use_ability` when an attacker fires a different ability
///     (any in-flight channel from this attacker stops)
///   - the death-transition path when an attacker dies (their channels
///     across all targets clear)
///
/// For each cancelled instance, dispatches the script's `on_remove`
/// hook (so Stun-on-channel-cancel clears the lock) and fires the
/// timer-clear wire packet so the client removes the buff icon.
///
/// Returns the number of instances cancelled (zero is the common case).
pub async fn cancel_channels_from_attacker(
    attacker_id: u32,
    keep_ability_id: Option<i32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    // Snapshot (target_id, effect_id, invoker_id) for every channel to
    // cancel — we can't hold a mutable borrow across awaits.
    let to_cancel: Vec<(u32, i32, u32)> = {
        let target_eids = space_mgr.all_entity_ids();
        let mut out = Vec::new();
        for target_eid in target_eids {
            let Some(entity) = space_mgr.get_entity(target_eid) else {
                continue;
            };
            for inst in &entity.active_effects {
                if inst.invoker_id != attacker_id {
                    continue;
                }
                // Only cancel channelled effects (registered from a
                // `pulse_count == 0` source) — finite DoTs from this
                // attacker keep ticking even if the attacker switches
                // weapons. The effect-def is the source of truth.
                let Some(effect_def) = space_mgr.effect_defs.get(&inst.effect_id) else {
                    continue;
                };
                if effect_def.pulse_count != 0 {
                    continue;
                }
                if keep_ability_id == Some(inst.ability_id) {
                    continue;
                }
                out.push((target_eid, inst.effect_id, inst.invoker_id));
            }
        }
        out
    };

    if to_cancel.is_empty() {
        return 0;
    }
    let cancelled_count = to_cancel.len();
    tracing::info!(
        target: "abilities",
        event = "channel_cancel_sweep",
        attacker_id,
        keep_ability_id = ?keep_ability_id,
        cancelled = cancelled_count,
        "Cancelling channels from attacker"
    );

    for (target_eid, effect_id, invoker_id) in to_cancel {
        // Remove the instance from the target.
        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
            target
                .active_effects
                .retain(|inst| !(inst.effect_id == effect_id && inst.invoker_id == invoker_id));
        }
        // Dispatch on_remove for script-driven cleanup.
        if let Some(effect_def) = space_mgr.effect_defs.get(&effect_id).cloned() {
            if let Some(script_name) = effect_def.script_name.clone() {
                let mut ctx = super::EffectContext {
                    source_id: invoker_id,
                    target_id: target_eid,
                    effect: &effect_def,
                    space_mgr,
                };
                super::dispatch_on_remove(&script_name, &mut ctx);
            }
            // Flush stat dirty bits from on_remove.
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
        // Wire-clear the buff timer.
        let zero_timer = serialize_timer_update(
            effect_id,
            TIMER_DURATION_EFFECT,
            invoker_id as i32,
            0.0,
            0.0,
        );
        send_entity_method(target_eid, ON_TIMER_UPDATE, zero_timer, tx, space_mgr).await;
    }

    cancelled_count
}

/// Sweep every active channel and cancel any whose invoker has moved
/// more than [`CHANNEL_INTERRUPT_DISTANCE`] from their anchor position.
/// Per-ability override: skip channels whose owning ability has
/// `AF_CHANNEL_ALLOWS_MOVEMENT` set.
///
/// Returns the number of channels cancelled (zero is the common case).
/// Called from `effect_pulse_tick` so the interrupt-then-pulse ordering
/// runs at the same 100ms cadence as everything else.
pub async fn channel_interrupt_on_movement_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    // Snapshot (invoker_id) for every channel that should interrupt.
    // We collect attackers, not instances, because
    // `cancel_channels_from_attacker(attacker, keep=None, ...)` is the
    // primitive — passing `None` cancels all of THIS attacker's channels
    // in one sweep. We need per-(invoker, ability) granularity because
    // a single attacker could have one cancel-on-move ability AND one
    // AF_CHANNEL_ALLOWS_MOVEMENT ability running concurrently.
    let mut to_cancel: Vec<(u32, i32)> = Vec::new();
    let entity_ids = space_mgr.all_entity_ids();
    for target_eid in entity_ids {
        let Some(target) = space_mgr.get_entity(target_eid) else {
            continue;
        };
        for inst in &target.active_effects {
            // Only channels carry an invoker anchor — finite DoTs leave
            // this `None` and are immune to caster-movement interrupt.
            let Some(anchor) = inst.invoker_position_at_register else {
                continue;
            };
            let Some(invoker) = space_mgr.get_entity(inst.invoker_id) else {
                // Invoker vanished — cancellation will happen via the
                // death-transition path if they died, or via the
                // `cancel_channels_from_attacker(None)` sweep on the
                // ability handler. Skip to avoid double-cancel.
                continue;
            };
            let dx = invoker.position.x - anchor.x;
            let dy = invoker.position.y - anchor.y;
            let dz = invoker.position.z - anchor.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            if dist < CHANNEL_INTERRUPT_DISTANCE {
                continue;
            }
            // Distance threshold crossed — but check the per-ability
            // override before scheduling cancel.
            let allows_movement = space_mgr
                .ability_defs
                .get(&inst.ability_id)
                .is_some_and(|def| def.flags & AF_CHANNEL_ALLOWS_MOVEMENT != 0);
            if allows_movement {
                continue;
            }
            // Schedule the (invoker, ability) pair for cancellation.
            // Dedup so two ticks against the same invoker+ability don't
            // double-cancel.
            if !to_cancel
                .iter()
                .any(|(i, a)| *i == inst.invoker_id && *a == inst.ability_id)
            {
                to_cancel.push((inst.invoker_id, inst.ability_id));
            }
        }
    }

    if to_cancel.is_empty() {
        return 0;
    }

    let mut total_cancelled = 0;
    for (invoker_id, ability_id) in to_cancel {
        tracing::info!(
            target: "abilities",
            event = "channel_interrupted_by_movement",
            invoker_id,
            ability_id,
            threshold = CHANNEL_INTERRUPT_DISTANCE,
            "Channel interrupted — caster moved past threshold"
        );
        // Cancel ONLY this specific (invoker, ability) so a concurrent
        // AF_CHANNEL_ALLOWS_MOVEMENT channel from the same invoker
        // survives the movement event.
        total_cancelled +=
            cancel_channels_for_invoker_ability(invoker_id, ability_id, tx, space_mgr).await;
    }
    total_cancelled
}

/// Cancel every channelled effect with the given `(invoker_id, ability_id)`
/// pair, regardless of which entity is hosting it. Used by the
/// channel-interrupt sweep so movement on one ability doesn't kill
/// a concurrent movement-tolerant channel from the same caster.
///
/// Mirrors `cancel_channels_from_attacker`'s cleanup discipline
/// (on_remove dispatch + stat-dirty flush + timer-clear wire packet).
pub async fn cancel_channels_for_invoker_ability(
    invoker_id: u32,
    ability_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let to_cancel: Vec<(u32, i32, u32)> = {
        let target_eids = space_mgr.all_entity_ids();
        let mut out = Vec::new();
        for target_eid in target_eids {
            let Some(entity) = space_mgr.get_entity(target_eid) else {
                continue;
            };
            for inst in &entity.active_effects {
                if inst.invoker_id != invoker_id || inst.ability_id != ability_id {
                    continue;
                }
                // Channels only — finite DoTs from this invoker keep ticking.
                let Some(effect_def) = space_mgr.effect_defs.get(&inst.effect_id) else {
                    continue;
                };
                if effect_def.pulse_count != 0 {
                    continue;
                }
                out.push((target_eid, inst.effect_id, inst.invoker_id));
            }
        }
        out
    };

    if to_cancel.is_empty() {
        return 0;
    }

    let cancelled_count = to_cancel.len();
    for (target_eid, effect_id, inv_id) in to_cancel {
        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
            target
                .active_effects
                .retain(|inst| !(inst.effect_id == effect_id && inst.invoker_id == inv_id));
        }
        if let Some(effect_def) = space_mgr.effect_defs.get(&effect_id).cloned() {
            if let Some(script_name) = effect_def.script_name.clone() {
                let mut ctx = super::EffectContext {
                    source_id: inv_id,
                    target_id: target_eid,
                    effect: &effect_def,
                    space_mgr,
                };
                super::dispatch_on_remove(&script_name, &mut ctx);
            }
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
        let zero_timer =
            serialize_timer_update(effect_id, TIMER_DURATION_EFFECT, inv_id as i32, 0.0, 0.0);
        send_entity_method(target_eid, ON_TIMER_UPDATE, zero_timer, tx, space_mgr).await;
    }
    cancelled_count
}

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
        // Identify which effects are due and clone enough to fire without
        // holding the borrow across awaits. Each due fire is followed by
        // a re-acquire to commit the schedule update.
        let due: Vec<(usize, ActiveEffectInstance, EffectDef)> = {
            let Some(entity) = space_mgr.get_entity(entity_id) else {
                continue;
            };
            entity
                .active_effects
                .iter()
                .enumerate()
                .filter(|(_, inst)| inst.next_pulse_at <= now && inst.remaining_pulses > 0)
                .filter_map(|(idx, inst)| {
                    space_mgr
                        .effect_defs
                        .get(&inst.effect_id)
                        .cloned()
                        .map(|def| (idx, inst.clone(), def))
                })
                .collect()
        };

        if due.is_empty() {
            continue;
        }

        // Fire each due pulse. We re-look up the entity every iteration
        // because between awaits another tick could mutate.
        for (idx, inst, effect_def) in &due {
            fire_pulse(entity_id, inst, effect_def, tx, space_mgr).await;
            // Update schedule + decrement on the matching instance.
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                if let Some(active) = entity.active_effects.get_mut(*idx) {
                    // Defensive: someone else may have already cleaned it.
                    if active.effect_id == inst.effect_id && active.invoker_id == inst.invoker_id {
                        active.remaining_pulses = active.remaining_pulses.saturating_sub(1);
                        if active.remaining_pulses > 0 {
                            active.next_pulse_at = now
                                + std::time::Duration::from_secs_f32(active.pulse_interval_secs);
                        }
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
                    let mut ctx = super::EffectContext {
                        source_id: invoker,
                        target_id: entity_id,
                        effect: &effect_def,
                        space_mgr,
                    };
                    super::dispatch_on_remove(&script_name, &mut ctx);
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
        let mut ctx = super::EffectContext {
            source_id: inst.invoker_id,
            target_id,
            effect,
            space_mgr,
        };
        super::dispatch_by_name(&script_name, &mut ctx);
    } else {
        // Legacy path — read HealthDamage / FocusDamage NVPs and
        // mutate the target's stats directly. No QR roll per pulse
        // (the initial cast's QR result already determined hit type;
        // re-rolling per pulse would make DoT unpredictable).
        let h_dmg = effect.param_i32("HealthDamage");
        let f_dmg = effect.param_i32("FocusDamage");
        if let Some(target) = space_mgr.get_entity_mut(target_id) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::EffectDef;
    use std::collections::HashMap;
    use std::time::Duration;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" Instanced="false" MinX="-100" MaxX="100" MinY="-100" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="W" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "W", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "W", [5.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 100, 100);
            }
        }
        if let Some(e) = mgr.get_entity_mut(2) {
            if let Some(s) = e.stats.get_mut(HEALTH) {
                s.update(0, 100, 100);
            }
        }
        mgr
    }

    fn make_dot_effect(pulse_count: i32, pulse_secs: f32, dmg: i32) -> EffectDef {
        let mut params = HashMap::new();
        params.insert("HealthDamage".to_string(), dmg.to_string());
        EffectDef {
            effect_id: 7777,
            ability_id: 1234,
            pulse_count,
            pulse_duration: pulse_secs,
            params,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn register_pulsing_effect_stores_remaining_pulse_count_minus_one() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        let now = Instant::now();
        let registered = register_active_effect(&mut mgr, 2, 1, &effect, now, &tx).await;
        assert!(registered);
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].remaining_pulses, 4);
        assert_eq!(active[0].total_pulses, 5);
    }

    #[tokio::test]
    async fn register_single_shot_effect_does_not_register() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(1, 0.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        let registered = register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        assert!(!registered);
        assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
    }

    #[tokio::test]
    async fn register_channelled_effect_uses_safety_cap_total_pulses() {
        // Phase J: pulse_count=0 → register with MAX_CHANNEL_PULSES cap.
        let mut mgr = make_mgr();
        let effect = make_dot_effect(0, 0.5, 10);
        let (tx, _rx) = mpsc::channel(64);
        let registered = register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        assert!(
            registered,
            "channelled effect now registers with safety cap"
        );
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].total_pulses, MAX_CHANNEL_PULSES);
        assert_eq!(active[0].remaining_pulses, MAX_CHANNEL_PULSES - 1);
    }

    #[tokio::test]
    async fn cancel_channels_from_attacker_drops_channeled_keeps_finite_dot() {
        // Phase J: when an attacker fires a different ability, channels
        // get cancelled but finite DoTs from the same attacker keep ticking.
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let now = Instant::now();
        // Effect 1: pulse_count=0 → channelled
        let mut channel_effect = make_dot_effect(0, 0.5, 10);
        channel_effect.effect_id = 9001;
        channel_effect.ability_id = 100;
        mgr.effect_defs.insert(9001, channel_effect.clone());
        // Effect 2: pulse_count=5 → finite DoT
        let mut dot_effect = make_dot_effect(5, 1.0, 10);
        dot_effect.effect_id = 9002;
        dot_effect.ability_id = 200;
        mgr.effect_defs.insert(9002, dot_effect.clone());

        register_active_effect(&mut mgr, 2, 1, &channel_effect, now, &tx).await;
        register_active_effect(&mut mgr, 2, 1, &dot_effect, now, &tx).await;
        assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 2);

        // Attacker fires a totally different ability (id 300). Channel
        // should die; DoT should survive.
        let cancelled = cancel_channels_from_attacker(1, Some(300), &tx, &mut mgr).await;
        assert_eq!(cancelled, 1, "exactly one channel cancelled");
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 1, "DoT survived");
        assert_eq!(active[0].effect_id, 9002, "finite DoT kept");
    }

    #[tokio::test]
    async fn cancel_channels_from_attacker_with_keep_ability_id_skips_matching() {
        // Phase J: same-ability re-fire keeps the channel alive (refresh path).
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let mut channel_effect = make_dot_effect(0, 0.5, 10);
        channel_effect.effect_id = 9003;
        channel_effect.ability_id = 100;
        mgr.effect_defs.insert(9003, channel_effect.clone());
        register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;

        // Re-fire the SAME ability — cancel sweep should keep this channel.
        let cancelled = cancel_channels_from_attacker(1, Some(100), &tx, &mut mgr).await;
        assert_eq!(cancelled, 0, "same-ability re-fire keeps channel");
        assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);
    }

    #[tokio::test]
    async fn channel_interrupt_fires_when_invoker_moves_past_threshold() {
        // Phase K: invoker at origin starts a channel; tick fires while
        // they're still at origin → no interrupt. Move them past 0.5m →
        // tick interrupts.
        use cimmeria_entity::abilities::AbilityDef;
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let mut channel_effect = make_dot_effect(0, 0.5, 5);
        channel_effect.effect_id = 9100;
        channel_effect.ability_id = 500;
        mgr.effect_defs.insert(9100, channel_effect.clone());
        mgr.ability_defs.insert(
            500,
            AbilityDef {
                ability_id: 500,
                name: "TestChannel".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0, // default = cancel-on-move
                is_ranged: true,
                min_range: 0,
                max_range: 50,
                target_type_id: 0,
                effect_ids: vec![9100],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;
        assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);

        // Invoker hasn't moved → no interrupt.
        let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
        assert_eq!(cancelled, 0, "stationary invoker keeps channel");
        assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);

        // Move invoker past threshold.
        if let Some(inv) = mgr.get_entity_mut(1) {
            inv.position.x = 1.5; // > 0.5m
        }
        let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
        assert_eq!(cancelled, 1, "moved invoker interrupted");
        assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
    }

    #[tokio::test]
    async fn channel_interrupt_respects_af_channel_allows_movement_flag() {
        // Phase K: ability with AF_CHANNEL_ALLOWS_MOVEMENT survives a
        // movement event that would otherwise interrupt.
        use cimmeria_entity::abilities::{AbilityDef, AF_CHANNEL_ALLOWS_MOVEMENT};
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let mut channel_effect = make_dot_effect(0, 0.5, 5);
        channel_effect.effect_id = 9101;
        channel_effect.ability_id = 501;
        mgr.effect_defs.insert(9101, channel_effect.clone());
        mgr.ability_defs.insert(
            501,
            AbilityDef {
                ability_id: 501,
                name: "MovingChannel".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: AF_CHANNEL_ALLOWS_MOVEMENT,
                is_ranged: true,
                min_range: 0,
                max_range: 50,
                target_type_id: 0,
                effect_ids: vec![9101],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;
        // Move invoker far past threshold.
        if let Some(inv) = mgr.get_entity_mut(1) {
            inv.position.x = 50.0;
        }
        let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
        assert_eq!(cancelled, 0, "AF_CHANNEL_ALLOWS_MOVEMENT exempts channel");
        assert_eq!(
            mgr.get_entity(2).unwrap().active_effects.len(),
            1,
            "channel survives movement"
        );
    }

    #[tokio::test]
    async fn channel_interrupt_below_threshold_does_not_fire() {
        // Phase K: small position jitter (< 0.5m) doesn't interrupt.
        use cimmeria_entity::abilities::AbilityDef;
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let mut channel_effect = make_dot_effect(0, 0.5, 5);
        channel_effect.effect_id = 9102;
        channel_effect.ability_id = 502;
        mgr.effect_defs.insert(9102, channel_effect.clone());
        mgr.ability_defs.insert(
            502,
            AbilityDef {
                ability_id: 502,
                name: "TestChannel2".to_string(),
                cooldown: 0.0,
                warmup: 0.0,
                flags: 0,
                is_ranged: true,
                min_range: 0,
                max_range: 50,
                target_type_id: 0,
                effect_ids: vec![9102],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;
        // Move invoker BUT only 0.3m (under threshold of 0.5m)
        if let Some(inv) = mgr.get_entity_mut(1) {
            inv.position.x = 0.3;
        }
        let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
        assert_eq!(cancelled, 0, "below-threshold jitter must not interrupt");
        assert_eq!(mgr.get_entity(2).unwrap().active_effects.len(), 1);
    }

    #[tokio::test]
    async fn finite_dot_does_not_interrupt_on_invoker_movement() {
        // Phase K: only channels (pulse_count == 0) carry the position
        // anchor. Finite DoTs from the same invoker keep ticking when
        // the invoker moves.
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let mut dot_effect = make_dot_effect(5, 1.0, 10);
        dot_effect.effect_id = 9103;
        dot_effect.ability_id = 503;
        mgr.effect_defs.insert(9103, dot_effect.clone());
        register_active_effect(&mut mgr, 2, 1, &dot_effect, Instant::now(), &tx).await;
        if let Some(inv) = mgr.get_entity_mut(1) {
            inv.position.x = 50.0;
        }
        let cancelled = channel_interrupt_on_movement_tick(&tx, &mut mgr).await;
        assert_eq!(cancelled, 0, "finite DoTs are immune to movement interrupt");
    }

    #[tokio::test]
    async fn cancel_channels_from_attacker_no_keep_cancels_all() {
        // Phase J: cancel-all path (called from death-transition) drops
        // every channel regardless of ability.
        let mut mgr = make_mgr();
        let (tx, _rx) = mpsc::channel(64);
        let mut channel_effect = make_dot_effect(0, 0.5, 10);
        channel_effect.effect_id = 9004;
        channel_effect.ability_id = 100;
        mgr.effect_defs.insert(9004, channel_effect.clone());
        register_active_effect(&mut mgr, 2, 1, &channel_effect, Instant::now(), &tx).await;

        let cancelled = cancel_channels_from_attacker(1, None, &tx, &mut mgr).await;
        assert_eq!(cancelled, 1);
        assert!(mgr.get_entity(2).unwrap().active_effects.is_empty());
    }

    #[tokio::test]
    async fn register_same_invoker_refreshes_existing_instance() {
        // Phase E stacking semantics: same source re-cast = refresh duration.
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        // Burn a few pulses so we can detect a refresh.
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(inst) = t.active_effects.first_mut() {
                inst.remaining_pulses = 1;
            }
        }
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 1, "same-source re-cast must NOT stack");
        assert_eq!(
            active[0].remaining_pulses, 4,
            "remaining_pulses refreshed to 4 (5 total - 1 just fired)"
        );
    }

    #[tokio::test]
    async fn register_different_invoker_stacks_separate_instance() {
        // Phase E stacking semantics: different source = stack.
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        // A different invoker (id 99) lands the same effect_id.
        register_active_effect(&mut mgr, 2, 99, &effect, Instant::now(), &tx).await;
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active.len(), 2, "different invokers must stack");
        let invokers: Vec<u32> = active.iter().map(|i| i.invoker_id).collect();
        assert!(invokers.contains(&1));
        assert!(invokers.contains(&99));
    }

    #[tokio::test]
    async fn pulse_tick_fires_due_pulse_and_decrements_remaining() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        mgr.effect_defs.insert(effect.effect_id, effect.clone());
        let past = Instant::now() - Duration::from_secs(2);
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, past, &tx).await;
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(inst) = t.active_effects.first_mut() {
                inst.next_pulse_at = past;
            }
        }
        let hp_before = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
        effect_pulse_tick(&tx, &mut mgr).await;
        let hp_after = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(hp_after, hp_before - 10, "DoT pulse should subtract 10");
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert_eq!(active[0].remaining_pulses, 3);
    }

    #[tokio::test]
    async fn pulse_tick_removes_instance_when_remaining_hits_zero() {
        let mut mgr = make_mgr();
        let mut effect = make_dot_effect(2, 1.0, 10);
        effect.effect_id = 8888;
        mgr.effect_defs.insert(effect.effect_id, effect.clone());
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(inst) = t.active_effects.first_mut() {
                inst.next_pulse_at = Instant::now() - Duration::from_secs(2);
                inst.remaining_pulses = 1;
            }
        }
        effect_pulse_tick(&tx, &mut mgr).await;
        let active = &mgr.get_entity(2).unwrap().active_effects;
        assert!(
            active.is_empty(),
            "instance removed after its last pulse fires"
        );
    }

    #[tokio::test]
    async fn pulse_tick_skips_pulse_on_dead_target() {
        let mut mgr = make_mgr();
        let effect = make_dot_effect(5, 1.0, 10);
        mgr.effect_defs.insert(effect.effect_id, effect.clone());
        let (tx, _rx) = mpsc::channel(64);
        register_active_effect(&mut mgr, 2, 1, &effect, Instant::now(), &tx).await;
        if let Some(t) = mgr.get_entity_mut(2) {
            if let Some(s) = t.stats.get_mut(HEALTH) {
                s.update(0, 0, 100);
            }
            if let Some(inst) = t.active_effects.first_mut() {
                inst.next_pulse_at = Instant::now() - Duration::from_secs(2);
            }
        }
        effect_pulse_tick(&tx, &mut mgr).await;
        let hp = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
        assert_eq!(hp, 0);
        let remaining = mgr.get_entity(2).unwrap().active_effects[0].remaining_pulses;
        assert_eq!(remaining, 3);
    }
}
