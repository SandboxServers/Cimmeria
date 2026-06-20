//! Ability system for entity combat.
//!
//! Each being has an `AbilityManager` that tracks known abilities, cooldowns,
//! and the currently active ability. Abilities are defined in the database
//! (`resources.abilities`) and organized into archetype-specific ability trees.
//!
//! Reference: `python/cell/AbilityManager.py`, `python/common/defs/Ability.py`
//!
//! Module layout:
//! - [`defs`] — flag/code constants plus [`AbilityDef`], [`EffectDef`],
//!   [`AbilityTreeData`].
//! - [`manager`] — [`AbilityManager`] and its [`CooldownEntry`].
//! - [`wire`] — client-message serializers ([`ClientEffectResult`],
//!   [`serialize_timer_update`], [`serialize_effect_results`]).
//!
//! All public items are re-exported here so external callers keep using the
//! flat `crate::abilities::Item` paths.

mod defs;
mod manager;
mod wire;

pub use defs::*;
pub use manager::{AbilityManager, CooldownEntry};
pub use wire::{serialize_effect_results, serialize_timer_update, ClientEffectResult};
