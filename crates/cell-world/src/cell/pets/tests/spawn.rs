//! `spawn_pet_from_template`: the pet comes out as a pet, not as the
//! placed mob its template describes.

use super::*;
use crate::cell::pets::{stance_mask_from_flags, PetSpawnError};
use crate::mercury::SGWPET_CLASS_ID;
use cimmeria_entity::cell_entity::PetState;
use cimmeria_entity::cell_entity::{PetStance, ALL_STANCES_MASK};
use cimmeria_wire::cell::client_methods::pet::{
    ENTITYFLAG_NO_DEFENSIVE, ENTITYFLAG_NO_PASSIVE, ENTITYFLAG_NO_PET_LEVELING, ENTITYFLAG_PET,
};

#[test]
fn spawned_pet_is_an_owned_sgwpet_with_the_owners_faction_and_level() {
    let (mgr, pet) = world_with_pet();
    let e = mgr.get_entity(pet).expect("pet entity exists");

    assert_eq!(e.class_id, SGWPET_CLASS_ID, "wire class must be SGWPet");
    assert!(!e.is_player);
    assert_ne!(e.entity_flags & ENTITYFLAG_PET, 0, "ENTITYFLAG_Pet set");
    // D-PT06: the owner's faction (players carry 0), never the template's 10.
    assert_eq!(e.faction, 0);
    assert!(!crate::cell::combat::aggression::is_hostile_to_players(e));
    // D-PT02: the owner's level, and health follows it.
    assert_eq!(e.level, 12);
    assert_eq!(
        e.stats.get(cimmeria_entity::stats::HEALTH).map(|s| s.max),
        Some(200 + 12 * 50)
    );
    // Not a placed NPC.
    assert_eq!(e.loot_table_id, None);
    assert_eq!(e.respawn_secs, None);
    assert!(e.patrol_path.is_empty());
    assert_eq!(e.wander_radius, 0.0);
    assert!(!e.use_cover);
    assert_eq!(e.tag, None);
    assert_eq!(e.spawn_id, None);
    // In the owner's space, near the owner.
    assert_eq!(mgr.get_entity_space_id(pet), mgr.get_entity_space_id(OWNER));
    let owner_pos = mgr.get_entity(OWNER).unwrap().position;
    assert!(e.position.distance_squared_to(&owner_pos) <= 2.5 * 2.5);

    let state = e.extensions.get::<PetState>().expect("pet state attached");
    assert_eq!(state.owner_id, OWNER);
    assert_eq!(state.stance, PetStance::Defensive);
    assert_eq!(
        state.ability_list,
        crate::test_fixtures::PET_FIXTURE_ABILITIES
    );
    assert_eq!(state.stance_mask, ALL_STANCES_MASK);
    assert_eq!(state.summon_ability_id, 1643);
    assert_eq!(state.transfer_xp, 1.0);

    assert_eq!(mgr.pets.pets_of(OWNER), vec![pet]);
    assert_eq!(mgr.pets.owner_of(pet), Some(OWNER));
}

#[test]
fn a_mob_class_template_still_spawns_as_sgwpet() {
    let mut mgr = make_world();
    let mut record = crate::test_fixtures::pet_template_record(351);
    record.class = "mob".to_string();
    mgr.spawn_templates.insert(351, record);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [0.0; 3], 5);
    let pet = mgr.spawn_pet_from_template(OWNER, 351, 0).unwrap();
    assert_eq!(mgr.get_entity(pet).unwrap().class_id, SGWPET_CLASS_ID);
}

#[test]
fn no_pet_leveling_keeps_the_template_level() {
    let mut mgr = make_world();
    let mut record = crate::test_fixtures::pet_template_record(352);
    record.flags = ENTITYFLAG_NO_PET_LEVELING as i64;
    mgr.spawn_templates.insert(352, record);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [0.0; 3], 20);
    let pet = mgr.spawn_pet_from_template(OWNER, 352, 0).unwrap();
    assert_eq!(mgr.get_entity(pet).unwrap().level, 3);
}

#[test]
fn stance_flags_narrow_the_stance_mask_and_the_start_stance() {
    assert_eq!(stance_mask_from_flags(0), ALL_STANCES_MASK);
    let mask = stance_mask_from_flags(ENTITYFLAG_NO_PASSIVE | ENTITYFLAG_NO_DEFENSIVE);
    assert_eq!(mask, PetStance::Aggressive.mask_bit());

    let mut mgr = make_world();
    let mut record = crate::test_fixtures::pet_template_record(353);
    record.flags = (ENTITYFLAG_NO_PASSIVE | ENTITYFLAG_NO_DEFENSIVE) as i64;
    mgr.spawn_templates.insert(353, record);
    add_pet_owner(&mut mgr, OWNER, "Agnos", [0.0; 3], 5);
    let pet = mgr.spawn_pet_from_template(OWNER, 353, 0).unwrap();
    let state = mgr
        .get_entity(pet)
        .unwrap()
        .extensions
        .get::<PetState>()
        .unwrap()
        .clone();
    assert_eq!(state.stance, PetStance::Aggressive);
    assert_eq!(state.allowed_stances(), vec![PetStance::Aggressive]);
}

#[test]
fn spawn_refuses_unknown_templates_and_non_player_owners() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [0.0; 3], 5);
    assert_eq!(
        mgr.spawn_pet_from_template(OWNER, 9999, 0),
        Err(PetSpawnError::UnknownTemplate(9999))
    );
    assert_eq!(
        mgr.spawn_pet_from_template(424_242, PET_FIXTURE_TEMPLATE_ID, 0),
        Err(PetSpawnError::OwnerNotFound(424_242))
    );
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc(npc, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    assert_eq!(
        mgr.spawn_pet_from_template(npc, PET_FIXTURE_TEMPLATE_ID, 0),
        Err(PetSpawnError::OwnerNotPlayer(npc))
    );
    assert!(mgr.pets.is_empty(), "a refused summon registers nothing");
}
