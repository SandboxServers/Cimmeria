//! Channel cancellation primitives.
//!
//! Channelled effects (`pulse_count == 0`) cancel proactively:
//! - [`cancel_channels_from_attacker`] — when the channeller fires a
//!   different ability or dies.
//! - [`channel_interrupt_on_movement_tick`] — when the channeller drifts
//!   past [`CHANNEL_INTERRUPT_DISTANCE`] from their anchor (per-tick
//!   sweep), delegating per-(invoker, ability) cancellation to
//!   [`cancel_channels_for_invoker_ability`].
//!
//! All three share the same cleanup discipline: dispatch the script's
//! `on_remove` hook, flush stat dirty bits, and fire the timer-clear
//! wire packet so the client removes the buff icon.

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{
    serialize_timer_update, AF_CHANNEL_ALLOWS_MOVEMENT, TIMER_DURATION_EFFECT,
};

use crate::cell::abilities::send_entity_method;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::CHANNEL_INTERRUPT_DISTANCE;

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
    // Snapshot (target_id, effect_id, ability_id, invoker_id) for every
    // channel to cancel — we can't hold a mutable borrow across awaits.
    // `ability_id` is part of the key (not just effect_id + invoker_id)
    // because a single invoker could have two channel instances sharing
    // the same effect_id under different abilities (rare but possible if
    // two abilities reference the same effect template). Without the
    // ability_id pin, cancelling one would over-delete the other.
    let to_cancel: Vec<(u32, i32, i32, u32)> = {
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
                out.push((target_eid, inst.effect_id, inst.ability_id, inst.invoker_id));
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

    for (target_eid, effect_id, ability_id, invoker_id) in to_cancel {
        // Remove the instance from the target — three-key match
        // (effect_id + ability_id + invoker_id) so we don't over-delete.
        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
            target.active_effects.retain(|inst| {
                !(inst.effect_id == effect_id
                    && inst.ability_id == ability_id
                    && inst.invoker_id == invoker_id)
            });
        }
        // Dispatch on_remove for script-driven cleanup.
        if let Some(effect_def) = space_mgr.effect_defs.get(&effect_id).cloned() {
            if let Some(script_name) = effect_def.script_name.clone() {
                let mut ctx = crate::cell::effects::EffectContext {
                    source_id: invoker_id,
                    target_id: target_eid,
                    effect: &effect_def,
                    space_mgr,
                };
                crate::cell::effects::dispatch_on_remove(&script_name, &mut ctx);
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
            // 2D planar (X/Z) distance to match the cone-collection
            // convention in `cone_aoe::collect_cone_targets`. Jumping
            // in place or being lifted by terrain shouldn't interrupt
            // a channel — only horizontal movement counts. Mixing 3D
            // here with 2D in cone collection would be confusing.
            let dx = invoker.position.x - anchor.x;
            let dz = invoker.position.z - anchor.z;
            let dist = (dx * dx + dz * dz).sqrt();
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
            target.active_effects.retain(|inst| {
                !(inst.effect_id == effect_id
                    && inst.ability_id == ability_id
                    && inst.invoker_id == inv_id)
            });
        }
        if let Some(effect_def) = space_mgr.effect_defs.get(&effect_id).cloned() {
            if let Some(script_name) = effect_def.script_name.clone() {
                let mut ctx = crate::cell::effects::EffectContext {
                    source_id: inv_id,
                    target_id: target_eid,
                    effect: &effect_def,
                    space_mgr,
                };
                crate::cell::effects::dispatch_on_remove(&script_name, &mut ctx);
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
