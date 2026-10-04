//! Effect scripts and active-effect pulsing, at their old path.
//!
//! The synchronous effect-script layer (`EffectScript`, `EffectContext`,
//! `dispatch_by_name` / `dispatch_on_remove` and the `registry` type) is in
//! `cimmeria-cell-world` (wave C1 of
//! `docs/architecture/services-crate-split.md`), because the spawn-time cover
//! hold runs Cover Stance through it. This module re-exports all of it, so
//! every `crate::cell::effects::X` path compiles unchanged. The scripts
//! themselves are in `cimmeria-cell-effect-scripts` (#962 step 4), which only
//! the composition root and test code depend on; dispatch finds them in the
//! registry the cell installs on its `SpaceManager`.
//!
//! [`pulsing`] — the async DoT/HoT/channel scheduler — is combat and stays
//! here, beside [`stat_buffs`], the async half of the stat-buff ledger
//! (expiry, duration timers, the death strip, the state-field flush), and
//! [`interrupt`], which resolves the interrupts effect scripts queue.

pub use cimmeria_cell_world::cell::effects::*;

pub mod interrupt;
pub mod pulsing;
pub mod stat_buffs;

pub use pulsing::{
    cancel_channels_for_invoker_ability, cancel_channels_from_attacker,
    channel_interrupt_on_movement_tick, effect_pulse_tick, register_active_effect,
};
pub use stat_buffs::{
    clear_stat_buffs_on_death, flush_stat_buff_timers, stat_buff_tick, stat_buff_tick_at,
    strip_timed_effects, HELD_ICON_SECS,
};
