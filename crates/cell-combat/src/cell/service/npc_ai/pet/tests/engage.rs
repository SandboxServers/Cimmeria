//! `engage_pet_target`, the one pet-engagement entry (stance picks and an
//! owner's attack order): both sides of the fight are seeded, and a refusal
//! changes nothing.

use super::*;
use crate::cell::service::npc_ai::pet::{engage_pet_target, PetEngagement};

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

    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Ok(())
    );

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
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Err("target_not_hostile")
    );
    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB_2, PetEngagement::Automatic),
        Err("target_dead")
    );
    assert_eq!(
        engage_pet_target(&mut mgr, MOB, pet, PetEngagement::Automatic),
        Err("not_a_pet")
    );

    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB).unwrap().threat_list.is_empty());
    assert!(mgr.get_entity(MOB_2).unwrap().threat_list.is_empty());
    assert_ne!(state(&mgr, pet), AiState::Fighting);
}

/// A surrendered (`Submit`) NPC is not engageable on the pet's own
/// initiative, but an owner's order still reaches it, as a player's own
/// attack does (`handle_use_ability` does not refuse a submitted target).
#[test]
fn a_surrendered_npc_takes_an_order_but_not_an_automatic_engagement() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(MOB).unwrap(), AiState::Submit);

    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Err("target_not_engageable")
    );
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::OwnerOrder),
        Ok(())
    );
    assert!(lists(&mgr, pet, MOB));
}
