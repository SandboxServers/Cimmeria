//! Special-ammo effect scripts, packet AM-11c (ammo campaign, issue #1026):
//! the buff and heal darts Stim, Coagulant, Nanites, Antidote and
//! Adrenaline.
//!
//! # How a beneficial dart fits the damage pipeline
//!
//! Each dart is an ordinary `ammo_modifiers` row
//! (`db/resources/Abilities/Seed/ammo_modifiers_dart_support.sql`). The
//! pipeline (`damage_apply`, AM-04) scales the shot by `damage_mult` and runs
//! `on_hit_effect_id` on the target after a hit. A support row sets
//! [`DART_SUPPORT_DAMAGE_MULT`], small enough that the shot deals no damage,
//! and an on-hit effect that helps the target:
//!
//! | Ammo | On-hit effect | Script |
//! |---|---|---|
//! | Stim | 9160, +10% Focus (effect 5008's text) | `HealFocus` |
//! | Antidote | 9161, removes Poison, Disease, Contagion, Wound, Burning | [`RemoveEffects`] |
//! | Coagulant | 9162, removes Wound | [`RemoveEffects`] |
//! | Adrenaline | 9163, +10% Health (RECONSTRUCTION) | `HealHealth` |
//! | Nanites | none: no evidence, so it has no row and fires as a plain dart | |
//!
//! Every on-hit effect here is server-only: it changes stats (flushed by
//! `damage_apply` as `onStatUpdate`) and sends no per-effect message. The
//! 91xx effect ids are not in the client's cooked data, and nobody has
//! checked what the client does with an unknown effect id in an
//! `onTimerUpdate` (an unknown cooked id crashed it before, #938). That is
//! why Adrenaline is a heal and not a `StatBuff`, which would send one.
//!
//! **Targeting (AM-11d).** Every row here is `beneficial = true`, so a
//! shot with one loaded lands on an ally player or the shooter and is
//! refused at a hostile target (`cimmeria-cell-combat`'s
//! `use_ability::support_shot`). It runs only the on-hit effect below, with
//! no damage, threat or combat state. `damage_apply` never runs a
//! beneficial row's on-hit effect, so these scripts only ever see an ally.
//!
//! # Effect categories
//!
//! `RemoveEffects` and the category NVPs moved to `cleanse/` when the
//! ability cleanses (ability-mechanics AB-10) started using them; they are
//! re-exported here so the dart rows' paths keep working.

pub use super::cleanse::{
    effect_category, remove_categories, RemoveEffects, EFFECT_CATEGORY_NVP, REMOVE_CATEGORIES_NVP,
};

/// `damage_mult` of every support dart row. The table's CHECK requires
/// `damage_mult > 0`, so the multiplier cannot be exactly 0. At this value
/// any shot under 5000 points of pre-armour damage rounds to 0 in
/// `combat::calculate_damage_penetrating`, and every seeded ability deals
/// far less than that.
pub const DART_SUPPORT_DAMAGE_MULT: f32 = 0.0001;

#[cfg(test)]
mod tests;
