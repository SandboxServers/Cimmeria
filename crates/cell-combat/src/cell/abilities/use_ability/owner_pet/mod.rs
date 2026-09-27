//! Owner abilities that act on the owner's pet (pets PT-08, issue #570).
//!
//! An ability whose effects carry a pet script
//! (`effects::pet_scripts::acts_on_owner_pet`: `PetStatBuff`,
//! `PetDeathTimer`, `HealPetHealth`) acts on the caster's pet, not on the
//! client's target. It rides the ordinary cast with three diversions, the
//! same shape as the summon (`super::summon`):
//!
//! 1. **Launch** ([`launch::refuse_owner_pet_launch`]). The client's
//!    `target_id` is discarded, so the #444 gate (a player may only aim at a
//!    hostile NPC) never sees the cast and stays exactly as strict for every
//!    other ability. The owner's pet is resolved through the registry with
//!    the summon-time identity (`SpaceManager::owner_pet_targets`); no pet,
//!    a pet in another space, a dead pet or a reused owner id refuses the
//!    press with `onErrorCode` plus a `CHAN_FEEDBACK` line, before the
//!    cooldown is charged. To The Death is refused while it is already
//!    running on the pet.
//! 2. **Warmup.** The ability's own warmup, with every decision-21
//!    interrupt.
//! 3. **Fire** ([`fire::fire_owner_pet`], ahead of the damage pipeline).
//!    The pet is resolved again (it may have died or left during the
//!    warmup), `Ability_End` plays, each pet effect script runs with the
//!    owner as source and the pet as target, the pet's stats go to its
//!    witnesses, and a pulsing heal is registered on the pet.
//!
//! [`tick::owner_pet_tick`] expires the timed buffs and carries out To The
//! Death. NPC casters never divert: [`player_owner_pet_ability`] only
//! answers for players.
//!
//! Log target `pets.buff`.

mod feedback;
mod fire;
mod launch;
mod tick;

#[cfg(test)]
mod tests;

pub(super) use fire::fire_owner_pet;
pub(super) use launch::refuse_owner_pet_launch;
pub use tick::{owner_pet_tick, owner_pet_tick_at};

use cimmeria_cell_world::cell::effects::pet_scripts::acts_on_owner_pet;

use super::super::super::space_manager::SpaceManager;

/// Whether `ability_id` acts on the owner's pet: one of its effects runs a
/// pet script. Read from the seed's `script_name`s, so a new pet ability is
/// wired by its data.
pub fn is_owner_pet_ability(space_mgr: &SpaceManager, ability_id: i32) -> bool {
    space_mgr.ability_defs.get(&ability_id).is_some_and(|def| {
        def.effect_ids.iter().any(|eid| {
            space_mgr
                .effect_defs
                .get(eid)
                .and_then(|e| e.script_name.as_deref())
                .is_some_and(acts_on_owner_pet)
        })
    })
}

/// [`is_owner_pet_ability`] for a player caster; `false` for anyone else.
/// The one question every diversion asks.
pub(super) fn player_owner_pet_ability(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
) -> bool {
    space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player)
        && is_owner_pet_ability(space_mgr, ability_id)
}

/// The `TargetID` an owner-pet cast's phase sequences carry: the owner's
/// first live pet, so the cast visibly lands on it. Python sent `targetId
/// or ent.entityId` (`AbilityManager.py:888-892`); the pet is the real
/// target here. Anything else keeps the target it was given.
pub(super) fn phase_sequence_target(
    space_mgr: &SpaceManager,
    entity_id: u32,
    ability_id: i32,
    target_id: i32,
) -> i32 {
    if target_id > 0 || !player_owner_pet_ability(space_mgr, entity_id, ability_id) {
        return target_id;
    }
    match space_mgr.owner_pet_targets(entity_id) {
        Ok(pets) => pets.first().map_or(target_id, |&p| p as i32),
        Err(_) => target_id,
    }
}
