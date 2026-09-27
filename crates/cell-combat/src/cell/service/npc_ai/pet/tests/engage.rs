//! `engage_pet_target`, the one pet-engagement entry (stance picks and an
//! owner's attack order): both sides of the fight are seeded, and a refusal
//! changes nothing.

use super::*;
use crate::cell::service::npc_ai::pet::engage_pet_target;

fn lists(mgr: &SpaceManager, who: u32, whom: u32) -> bool {
    mgr.get_entity(who).unwrap().threat_list.contains_key(&whom)
}

/// Both sides: the mob lists the pet and fights back, the pet lists the mob
/// and fights. An explicit engagement ignores the stance, so an owner's
/// order is obeyed even by a Passive pet.
#[test]
fn an_engagement_is_two_sided_whatever_the_stance() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Passive);

    assert_eq!(engage_pet_target(&mut mgr, pet, MOB), Ok(()));

    assert_eq!(state(&mgr, MOB), AiState::Fighting, "the mob fights back");
    assert!(lists(&mgr, MOB, pet), "the mob lists the pet");
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert!(lists(&mgr, pet, MOB), "the pet lists the mob");
}

/// A refusal says why and touches neither side.
#[test]
fn a_refused_engagement_changes_nothing() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], 0);
    add_mob(&mut mgr, MOB_2, [16.0, 0.0, 10.0], HOSTILE);
    mgr.get_entity_mut(MOB_2)
        .unwrap()
        .stats
        .get_mut(cimmeria_entity::stats::HEALTH)
        .unwrap()
        .update(0, 0, 100);

    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB),
        Err("target_not_hostile")
    );
    assert_eq!(engage_pet_target(&mut mgr, pet, MOB_2), Err("target_dead"));
    assert_eq!(engage_pet_target(&mut mgr, MOB, pet), Err("not_a_pet"));

    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB_2).unwrap().threat_list.is_empty());
    assert_ne!(state(&mgr, pet), AiState::Fighting);
}
