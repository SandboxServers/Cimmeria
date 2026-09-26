//! Active-effect pulsing for DoT, HoT, and timed-buff effects.
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
//! ## Scope notes
//!
//! - **Stacking**: same-source = refresh, multi-source = stack.
//!   See [`register_active_effect`] for the dedup logic.
//! - **Channelled effects**: `pulse_count == 0` is treated as
//!   "channel until cancelled" by registering with a safety cap
//!   ([`MAX_CHANNEL_DURATION_SECS`]) and cancelling proactively via
//!   [`cancel_channels_from_attacker`] when the channeller fires a
//!   different ability, dies, or the safety cap elapses. Movement-based
//!   interrupt fires via [`channel_interrupt_on_movement_tick`] — any
//!   channel whose invoker drifts more than [`CHANNEL_INTERRUPT_DISTANCE`]
//!   from their channel-start anchor cancels, unless the owning ability
//!   carries the `AF_CHANNEL_ALLOWS_MOVEMENT` flag.
//! - **Dead-target pulse** is a no-op (the per-pulse guard returns
//!   without applying), but the instance still ages out via the schedule.
//!
//! ## Module layout
//!
//! This is split along the pulsing lifecycle:
//!
//! - [`register`] — [`register_active_effect`]: register/refresh a
//!   pulsing instance on a target right after the initial pulse fires.
//! - [`channel_cancel`] — the channel cancellation primitives
//!   ([`cancel_channels_from_attacker`], [`cancel_channels_for_invoker_ability`])
//!   and the per-tick movement-interrupt sweep
//!   ([`channel_interrupt_on_movement_tick`]).
//! - [`tick`] — [`effect_pulse_tick`] (the per-cell scheduler) and the
//!   single-pulse apply path.

mod channel_cancel;
mod register;
mod tick;

#[cfg(test)]
mod tests;

pub use channel_cancel::{
    cancel_channels_for_invoker_ability, cancel_channels_from_attacker,
    channel_interrupt_on_movement_tick,
};
pub use register::register_active_effect;
pub use tick::effect_pulse_tick;

/// Hard time cap for a `pulse_count == 0` channelled effect. Acts as
/// a safety backstop so a missed cancellation event can't leave a
/// channel running forever. Well past any reasonable in-game channel
/// duration — Sustained Sweep is 10s / 20 pulses (the longest authored
/// channel in seed).
///
/// Stored as seconds (not pulse count) so a channel with a 0.1s
/// `pulse_duration` still caps at 30 s of real time rather than the
/// 3 s a fixed pulse-count would give it.
pub(crate) const MAX_CHANNEL_DURATION_SECS: f32 = 30.0;

/// Distance (world units / metres) the channeller can drift from their
/// channel-start position before the channel interrupts. Tuned for
/// typical movement: walking is ~3 m/s, so a 0.5m budget catches step-
/// off-the-spot intent within ~150ms of a player input without firing
/// on the tiny position jitter from server-side smoothing.
///
/// Per-ability override: set `AF_CHANNEL_ALLOWS_MOVEMENT` on the
/// `AbilityDef.flags` and the interrupt sweep skips that channel.
pub(crate) const CHANNEL_INTERRUPT_DISTANCE: f32 = 0.5;
