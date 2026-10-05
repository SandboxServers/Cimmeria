//! Which effect scripts act on pets (pets PT-08, issue #570).
//!
//! The pet scripts themselves (`PetStatBuff`, `PetDeathTimer`,
//! `HealPetHealth`, `PetSummonSpeed`) are in `cimmeria-cell-effect-scripts`
//! (`cell::effects::pet_scripts`, which re-exports this module), with the
//! ability table that maps each owner ability to its script. What stays here
//! are the name predicates the layers below that crate ask:
//!
//! - [`acts_on_owner_pet`]: the cast redirect in `cimmeria-cell-combat`
//!   (`use_ability::owner_pet`) recognises an ability that acts on the
//!   caster's pet by the scripts on its effects, which the seed names;
//! - [`is_passive_script`]: the passive pass ([`super::passives`]) applies
//!   only these while the ability is known.

/// Whether an effect with `script` acts on the caster's pet, so its
/// ability's cast must be redirected to the owner's pet.
pub fn acts_on_owner_pet(script: &str) -> bool {
    matches!(script, "PetStatBuff" | "PetDeathTimer" | "HealPetHealth")
}

/// Whether `script` is the effect of a passive ability, applied while the
/// ability is known (`EF_AlwaysPersist`, see `super::passives`): the pet
/// summon speed, and the timed effect ledger's `TimedStat`, which holds an
/// `EF_AlwaysPersist` stat entry until the respec takes it off (ability
/// mechanics AB-08). A heal is never one: a login must not fire it.
pub fn is_passive_script(script: &str) -> bool {
    matches!(script, "PetSummonSpeed" | "TimedStat")
}
