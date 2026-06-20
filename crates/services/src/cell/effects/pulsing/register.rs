//! Register/refresh a pulsing effect on a target.
//!
//! Called once per effect right after the initial pulse fires in
//! `damage_apply`. Implements same-source-refresh / multi-source-stack
//! semantics and the channelled-effect safety cap.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{serialize_timer_update, EffectDef, TIMER_DURATION_EFFECT};
use cimmeria_entity::cell_entity::ActiveEffectInstance;

use crate::cell::abilities::send_entity_method;
use crate::cell::client_methods::being::ON_TIMER_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::MAX_CHANNEL_DURATION_SECS;

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
    // pulse_count == 0 → channelled. Compute the cap as
    // `ceil(MAX_CHANNEL_DURATION_SECS / pulse_duration)` so a fast-
    // pulsing channel (0.1 s) still gets 30 s of safety runway rather
    // than the ~3 s a fixed pulse-count cap would give it.
    // Cancellation comes from `cancel_channels_from_attacker` (different
    // ability fire, death) or `channel_interrupt_on_movement_tick`;
    // the safety cap only triggers when ALL of those fail.
    let total_pulses = if effect.pulse_count == 0 {
        let interval = effect.pulse_duration.max(0.1);
        (MAX_CHANNEL_DURATION_SECS / interval).ceil() as i32
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
            // Preserve the higher remaining pulse count — a 10-pulse DoT
            // at 8 remaining, refreshed by a 5-pulse cast of the same
            // effect, should NOT drop to 4. Matches the Python
            // reference's "refresh extends, never shortens" rule.
            existing.remaining_pulses = existing.remaining_pulses.max(remaining);
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
