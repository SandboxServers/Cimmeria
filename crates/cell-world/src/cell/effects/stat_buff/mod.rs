//! The timed effect ledger's `SpaceManager` face (ability-mechanics AB-04,
//! D-AB08; decision 28 of `docs/architecture/abilities-and-effects-system.md`):
//! stat changes with a duration, from the consumable stimpacks to ability
//! buffs and debuffs, one entry per `(entity, effect_id, invoker)`.
//!
//! The scripts that apply entries (`StatBuff` for the stimpacks, `TimedStat`
//! for abilities) are in `cimmeria-cell-effect-scripts`
//! (`cell::effects::stat_buff`, which re-exports this module). What stays
//! here is the logged ledger API every caller uses, synchronous so an effect
//! script can call it:
//!
//! - `SpaceManager::apply_timed_effect(target, TimedEffectSpec, now)`;
//! - `SpaceManager::remove_timed_effects(target, StatBuffRemoval, pred)` and
//!   `remove_timed_effects_by_moniker`;
//! - [`StatBuffRemoval`], the reason an entry came off.
//!
//! The async half (expiry, the client's duration timers, the death strip and
//! the other clear hooks) is `cimmeria-cell-combat`'s `effects::stat_buffs`.
//!
//! Log target `abilities`.

mod ledger;

pub use ledger::StatBuffRemoval;
