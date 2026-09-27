//! The ability selector honours the owner's toggled-off list
//! (`SGWPet.toggledAbilities`).

use super::*;
use crate::cell::service::npc_ai::{choose_npc_ability, choose_npc_ability_within_reach};
use crate::test_support::PET_FIXTURE_ABILITIES;

fn toggle_off(mgr: &mut SpaceManager, pet: u32, ids: &[i32]) {
    mgr.get_entity_mut(pet)
        .unwrap()
        .pet
        .as_deref_mut()
        .unwrap()
        .toggled_off = ids.to_vec();
}

#[test]
fn pet_selector_skips_toggled_off_abilities() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let [first, second] = PET_FIXTURE_ABILITIES;
    assert_eq!(choose_npc_ability(pet, &mgr), Some(first), "precondition");

    toggle_off(&mut mgr, pet, &[first]);
    assert_eq!(choose_npc_ability(pet, &mgr), Some(second));
    assert_eq!(
        choose_npc_ability_within_reach(pet, &mgr, 5.0, 30.0),
        Some(second),
        "the reach-filtered production selector too"
    );

    toggle_off(&mut mgr, pet, &[first, second]);
    assert_eq!(
        choose_npc_ability(pet, &mgr),
        None,
        "everything toggled off: the pet holds fire"
    );
}

/// A pet whose template grants no ability falls back to the NPC default
/// ability; toggling that off holds fire as well, like any other ability.
#[test]
fn a_pet_with_no_abilities_honours_a_toggled_off_default() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    mgr.get_entity_mut(pet).unwrap().abilities = Default::default();
    let default = crate::cell::combat::NPC_DEFAULT_ABILITY;
    assert_eq!(choose_npc_ability(pet, &mgr), Some(default), "precondition");

    toggle_off(&mut mgr, pet, &[default]);
    assert_eq!(choose_npc_ability(pet, &mgr), None);
    assert_eq!(choose_npc_ability_within_reach(pet, &mgr, 5.0, 30.0), None);
}
