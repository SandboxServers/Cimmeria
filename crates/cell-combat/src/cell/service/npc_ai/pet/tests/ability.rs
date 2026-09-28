//! The ability selector honours the owner's toggled-off list
//! (`SGWPet.toggledAbilities`), and never picks an ability with no visible
//! result for a pet (PT-11).

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

/// Insert a def for `id` with `event_set_id` and no effects: with `None` it
/// has no visible result (`ability_is_unimplemented`).
fn def_with_event_set(mgr: &mut SpaceManager, id: i32, event_set_id: Option<i32>) {
    mgr.ability_defs.insert(
        id,
        cimmeria_entity::abilities::AbilityDef {
            ability_id: id,
            name: format!("Test{id}"),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0.0,
            max_range: 0.0,
            target_type_id: 2,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id,
            velocity: 100.0,
        },
    );
}

/// PT-11: a pet never picks an ability that does nothing (no damage, no
/// effect script, no event set), so the Lo'taur holds fire instead of
/// "attacking" with empty hits. Fails with the pet AI's skip removed: the
/// lowest id, the unimplemented one, is picked.
#[test]
fn pet_selector_skips_abilities_that_do_nothing() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let [first, second] = PET_FIXTURE_ABILITIES;
    def_with_event_set(&mut mgr, first, None);
    def_with_event_set(&mut mgr, second, Some(3));

    assert_eq!(choose_npc_ability(pet, &mgr), Some(second));
    assert_eq!(
        choose_npc_ability_within_reach(pet, &mgr, 5.0, 30.0),
        Some(second),
        "the reach-filtered production selector too"
    );

    def_with_event_set(&mut mgr, second, None);
    assert_eq!(
        choose_npc_ability(pet, &mgr),
        None,
        "a kit of nothing but no-ops: the pet holds fire"
    );
    assert_eq!(choose_npc_ability_within_reach(pet, &mgr, 5.0, 30.0), None);
}

/// The skip is pet-only: a mob keeps its seeded behaviour (the NA43 linter
/// already keeps silent abilities out of mob kits).
#[test]
fn a_mob_still_picks_an_ability_that_does_nothing() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let [first, _] = PET_FIXTURE_ABILITIES;
    def_with_event_set(&mut mgr, first, None);
    mgr.get_entity_mut(pet).unwrap().pet = None;
    assert_eq!(choose_npc_ability(pet, &mgr), Some(first));
}
