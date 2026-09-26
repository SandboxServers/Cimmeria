//! Effect scripts and active-effect pulsing, at their old path.
//!
//! The synchronous effect-script layer (`EffectScript`, `EffectContext`,
//! `dispatch_by_name` / `dispatch_on_remove`, the `registry` and the
//! `scripts`, `cover_stance` included) is in `cimmeria-cell-world` (wave C1 of
//! `docs/architecture/services-crate-split.md`), because the spawn-time cover
//! hold runs Cover Stance through it. This module re-exports all of it, so
//! every `crate::cell::effects::X` path compiles unchanged.
//!
//! [`pulsing`] — the async DoT/HoT/channel scheduler — is combat and stays
//! here.

pub use cimmeria_cell_world::cell::effects::*;

pub mod pulsing;

pub use pulsing::{
    cancel_channels_for_invoker_ability, cancel_channels_from_attacker,
    channel_interrupt_on_movement_tick, effect_pulse_tick, register_active_effect,
};
