//! `PetRegistry`, the ownership guard and the credit seam.

use super::*;
use crate::cell::pets::{PetRegistry, PetReject};

#[test]
fn owned_pet_accepts_only_the_owner() {
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 100_001);
    assert_eq!(reg.owned_pet(OWNER, 100_001), Ok(100_001));
    assert_eq!(
        reg.owned_pet(OTHER, 100_001),
        Err(PetReject::NotOwner { owner_id: OWNER })
    );
    assert_eq!(reg.owned_pet(OWNER, 100_002), Err(PetReject::NotAPet));
    assert_eq!(reg.owned_pet(OWNER, OWNER), Err(PetReject::NotAPet));
}

#[test]
fn forget_pet_clears_both_maps() {
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 100_001);
    reg.register(OWNER, 100_002);
    assert_eq!(reg.pets_of(OWNER), vec![100_001, 100_002]);
    assert_eq!(reg.forget_pet(100_001), Some(OWNER));
    assert_eq!(reg.pets_of(OWNER), vec![100_002]);
    assert_eq!(reg.forget_pet(100_002), Some(OWNER));
    assert!(reg.pets_of(OWNER).is_empty());
    assert!(reg.is_empty());
    assert_eq!(reg.forget_pet(100_002), None);
}

#[test]
fn reregistering_a_pet_moves_it_to_the_new_owner() {
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 100_001);
    reg.register(OTHER, 100_001);
    assert!(reg.pets_of(OWNER).is_empty());
    assert_eq!(reg.pets_of(OTHER), vec![100_001]);
    assert_eq!(reg.len(), 1);
}

#[test]
fn space_manager_owned_pet_refuses_a_registry_entry_without_an_entity() {
    let (mut mgr, pet) = world_with_pet();
    assert_eq!(mgr.owned_pet(OWNER, pet), Ok(pet));
    // Simulate a torn-down pet the registry still names (the sweep's gap).
    mgr.pets.register(OWNER, 999_999);
    assert_eq!(mgr.owned_pet(OWNER, 999_999), Err(PetReject::PetGone));
    // An ordinary NPC id is not a pet.
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    assert_eq!(mgr.owned_pet(OWNER, npc), Err(PetReject::NotAPet));
}

#[test]
fn credit_recipient_maps_pet_to_owner_player_to_self_npc_to_none() {
    let (mut mgr, pet) = world_with_pet();
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    assert_eq!(mgr.credit_recipient(pet), Some(OWNER));
    assert_eq!(mgr.credit_recipient(OWNER), Some(OWNER));
    assert_eq!(mgr.credit_recipient(npc), None);
    assert_eq!(mgr.credit_recipient(424_242), None);
}
