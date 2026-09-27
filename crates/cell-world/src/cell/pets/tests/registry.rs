//! `PetRegistry`, the ownership guard and the credit seam.

use super::*;
use crate::cell::pets::{PetRegistry, PetReject};
use cimmeria_entity::cell_entity::PlayerIdentity;

const NOBODY: PlayerIdentity = PlayerIdentity::UNKNOWN;

#[test]
fn owned_pet_accepts_only_the_owner() {
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 100_001, NOBODY);
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
    reg.register(OWNER, 100_001, NOBODY);
    reg.register(OWNER, 100_002, NOBODY);
    assert_eq!(reg.pets_of(OWNER), vec![100_001, 100_002]);
    assert_eq!(reg.forget_pet(100_001), Some(OWNER));
    assert_eq!(reg.pets_of(OWNER), vec![100_002]);
    assert_eq!(reg.forget_pet(100_002), Some(OWNER));
    assert!(reg.pets_of(OWNER).is_empty());
    assert!(reg.is_empty());
    assert_eq!(reg.forget_pet(100_002), None);
    assert!(!reg.summoner_identity(100_001).is_known());
}

#[test]
fn reregistering_a_pet_moves_it_to_the_new_owner() {
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 100_001, NOBODY);
    reg.register(OTHER, 100_001, NOBODY);
    assert!(reg.pets_of(OWNER).is_empty());
    assert_eq!(reg.pets_of(OTHER), vec![100_001]);
    assert_eq!(reg.len(), 1);
}

#[test]
fn space_manager_owned_pet_refuses_a_registry_entry_without_an_entity() {
    let (mut mgr, pet) = world_with_pet();
    assert_eq!(mgr.owned_pet(OWNER, pet), Ok(pet));
    // Simulate a torn-down pet the registry still names (the sweep's gap).
    mgr.pets.register(OWNER, 999_999, NOBODY);
    assert_eq!(mgr.owned_pet(OWNER, 999_999), Err(PetReject::PetGone));
    // An ordinary NPC id is not a pet.
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    assert_eq!(mgr.owned_pet(OWNER, npc), Err(PetReject::NotAPet));
}

/// A different player given the owner's entity id cannot command the pet:
/// the id matches the registry, the summon-time identity does not.
#[test]
fn owned_pet_refuses_a_player_that_reused_the_owner_id() {
    let (mut mgr, pet) = world_with_pet();
    assert_eq!(mgr.owned_pet(OWNER, pet), Ok(pet), "control: the summoner");
    reuse_owner_id_by_another_player(&mut mgr);
    assert_eq!(
        mgr.owned_pet(OWNER, pet),
        Err(PetReject::OwnerIdentityMismatch)
    );
}

#[test]
fn identity_match_uses_character_then_account_and_never_trusts_unknown() {
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 1, PlayerIdentity::new(Some(10), Some(11)));
    assert!(reg.summoner_matches(1, PlayerIdentity::new(Some(10), Some(11))));
    assert!(!reg.summoner_matches(1, PlayerIdentity::new(Some(10), Some(12))));
    assert!(!reg.summoner_matches(1, PlayerIdentity::UNKNOWN));
    reg.register(OWNER, 2, PlayerIdentity::new(Some(20), None));
    assert!(reg.summoner_matches(2, PlayerIdentity::new(Some(20), Some(5))));
    assert!(!reg.summoner_matches(2, PlayerIdentity::new(Some(21), None)));
    // Nothing captured: cannot vouch for anyone.
    reg.register(OWNER, 3, NOBODY);
    assert!(!reg.summoner_matches(3, PlayerIdentity::new(Some(30), Some(31))));
    assert!(!reg.summoner_matches(4, PlayerIdentity::new(Some(30), Some(31))));
}

/// Copilot, #870: the capture is per pet. A second summon under the same
/// (reused) owner id records its own summoner and does not overwrite the
/// first pet's.
#[test]
fn a_second_summon_under_the_same_owner_id_keeps_each_pets_summoner() {
    let a = PlayerIdentity::new(Some(10), Some(11));
    let b = PlayerIdentity::new(Some(20), Some(21));
    let mut reg = PetRegistry::default();
    reg.register(OWNER, 1, a);
    reg.register(OWNER, 2, b);
    assert!(reg.summoner_matches(1, a) && !reg.summoner_matches(1, b));
    assert!(reg.summoner_matches(2, b) && !reg.summoner_matches(2, a));
    reg.forget_pet(2);
    assert!(
        reg.summoner_matches(1, a),
        "forgetting one pet keeps the other's"
    );
}

/// Copilot, #870: the owner's id is reused by another player, who then
/// summons a pet of its own before the sweep. The OLD pet stays refused to
/// that player (commands, credit); its own new pet is accepted.
#[test]
fn a_new_summon_by_the_id_holder_does_not_adopt_the_old_pet() {
    let (mut mgr, old_pet) = world_with_pet();
    let new_pet = super::reuse_owner_id_then_resummon(&mut mgr);
    assert_eq!(
        mgr.owned_pet(OWNER, old_pet),
        Err(PetReject::OwnerIdentityMismatch),
        "the old pet is still not the new holder's"
    );
    assert_eq!(mgr.owned_pet(OWNER, new_pet), Ok(new_pet));
    assert_eq!(mgr.credit_recipient(old_pet), None);
    assert_eq!(mgr.credit_recipient(new_pet), Some(OWNER));
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

/// Copilot, #870: a pet's kill credit must not reach another player who was
/// given the owner's entity id before the sweep, nor a destroyed owner.
#[test]
fn credit_recipient_refuses_a_reused_or_gone_owner_id() {
    let (mut mgr, pet) = world_with_pet();
    super::reuse_owner_id_by_another_player(&mut mgr);
    assert_eq!(mgr.credit_recipient(pet), None, "reused id");
    mgr.destroy_entity(OWNER);
    assert_eq!(mgr.credit_recipient(pet), None, "owner gone");
}
