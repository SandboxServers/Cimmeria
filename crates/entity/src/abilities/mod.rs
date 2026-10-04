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
//! - [`implemented`] — [`ability_is_unimplemented`]: whether a cast has any
//!   visible result (damage, an effect script or an event set).
//! - [`beneficial`] — [`ability_is_beneficial`]: a heal or buff, resolved on
//!   the caster or an ally (ability-mechanics D-AB02).
//! - [`ability_type`] — [`AbilityType`], the `type_id` column.
//! - [`manager`] — [`AbilityManager`] and its [`CooldownEntry`].
//! - [`wire`] — client-message serializers ([`ClientEffectResult`],
//!   [`serialize_timer_update`], [`serialize_effect_results`]).
//!
//! All public items are re-exported here so external callers keep using the
//! flat `crate::abilities::Item` paths.

mod ability_type;
mod beneficial;
mod defs;
mod implemented;
mod manager;
mod range;
mod wire;

pub use ability_type::AbilityType;
pub use beneficial::ability_is_beneficial;
pub use defs::*;
pub use implemented::{ability_is_unimplemented, effect_is_implemented};
pub use manager::{AbilityManager, CooldownEntry};
pub use range::{
    ability_max_range, ability_range_bounds, ability_range_to_metres, active_weapon_ranges,
    ae_radius_metres, caster_range_bounds, RangeBounds, RangeRefusal, RangeSource, WeaponRanges,
    ABILITY_RANGE_UNITS_PER_METRE, DEFAULT_ABILITY_MAX_RANGE,
};
pub use wire::{serialize_effect_results, serialize_timer_update, ClientEffectResult};
