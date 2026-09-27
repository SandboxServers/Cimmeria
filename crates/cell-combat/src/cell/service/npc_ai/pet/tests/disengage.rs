//! What a pet leaving a fight does to the mob's entry for it (pets PT-05):
//! released when the target stopped being fightable or the owner called the
//! pet off, kept when the pet is only pulled back by distance.

use super::*;
use crate::cell::combat::BSF_IN_COMBAT;
use crate::cell::service::npc_ai::pet::engage_pet_target;
use crate::test_support::LogCapture;

/// A two-sided fight between the pet and `MOB`, the owner mirrored into it.
async fn engaged(owner: [f32; 3], pet_at: [f32; 3], mob_at: [f32; 3]) -> (SpaceManager, u32) {
    let (mut mgr, pet) = world_with_pet(owner);
    move_to(&mut mgr, pet, pet_at);
    add_mob(&mut mgr, MOB, mob_at, HOSTILE);
    assert_eq!(engage_pet_target(&mut mgr, pet, MOB), Ok(()));
    (mgr, pet)
}

fn mob_lists_pet(mgr: &SpaceManager, pet: u32) -> bool {
    mgr.get_entity(MOB).unwrap().threat_list.contains_key(&pet)
}

fn owner_in_fight_with_mob(mgr: &SpaceManager) -> bool {
    let o = mgr.get_entity(OWNER).unwrap();
    o.threatened_mobs.contains(&MOB) && o.state_field & BSF_IN_COMBAT != 0
}

/// The target turns friendly mid-fight: the pet drops it and the mob forgets
/// the pet, so the owner leaves combat with it this same turn.
#[tokio::test]
async fn a_target_that_turns_friendly_forgets_the_pet() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    tick(&mut mgr).await;
    assert!(owner_in_fight_with_mob(&mgr), "precondition");
    mgr.get_entity_mut(MOB).unwrap().faction = 0;

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert!(!mob_lists_pet(&mgr, pet), "the mob forgot the pet");
    assert!(
        !owner_in_fight_with_mob(&mgr),
        "and the owner left the fight"
    );
    let row = pets_ai_row(&logs, "pet_attackers_released").expect("released row");
    assert!(row.has_field("reason", "target_invalid"), "{row:?}");
    assert!(row.has_field("released", "1"), "{row:?}");
}

/// The owner switches the pet to Passive (a recall): every mob that listed
/// the pet forgets it, and the owner leaves combat with them.
#[tokio::test]
async fn calling_the_pet_off_releases_every_attacker() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    tick(&mut mgr).await;
    assert!(owner_in_fight_with_mob(&mgr), "precondition");
    set_stance(&mut mgr, pet, PetStance::Passive);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(!mob_lists_pet(&mgr, pet), "the mob forgot the pet");
    assert!(
        !owner_in_fight_with_mob(&mgr),
        "and the owner left the fight"
    );
    let row = pets_ai_row(&logs, "pet_attackers_released").expect("released row");
    assert!(row.has_field("reason", "called_off"), "{row:?}");
}

/// Pulled back by the owner-anchored leash: the pet follows again, but the
/// mob is still fighting it, the way it keeps chasing a fleeing player until
/// its own leash resets it.
#[tokio::test]
async fn a_leash_pull_back_leaves_the_mob_on_the_pet() {
    let (mut mgr, pet) = engaged([100.0, 0.0, 10.0], [0.0, 0.0, 0.0], [70.0, 0.0, 10.0]).await;

    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Follow, "the pet leashed back");
    assert!(mob_lists_pet(&mgr, pet), "the mob still fights the pet");
}

/// Left behind by an owner teleport (`target_far_from_owner`): the pet lets
/// the target go, the target keeps chasing the pet.
#[tokio::test]
async fn a_target_left_behind_by_distance_keeps_the_pet() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    move_to(&mut mgr, OWNER, [300.0, 0.0, 10.0]);
    let (tx, _rx) = mpsc::channel(64);
    crate::cell::pets::on_owner_teleported(
        OWNER,
        crate::cell::pets::OwnerPath::GmTravel,
        &tx,
        &mut mgr,
    )
    .await;

    tick(&mut mgr).await;

    assert!(!mgr.get_entity(pet).unwrap().threat_list.contains_key(&MOB));
    assert!(mob_lists_pet(&mgr, pet), "the mob still fights the pet");
}

/// With another target still in the fight, the pet does not re-arm; the
/// owner's entry for the mob that forgot the pet still goes this turn.
#[tokio::test]
async fn a_released_target_leaves_the_owner_while_the_fight_goes_on() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    add_mob(&mut mgr, MOB_2, [12.0, 0.0, 6.0], HOSTILE);
    assert_eq!(engage_pet_target(&mut mgr, pet, MOB_2), Ok(()));
    tick(&mut mgr).await;
    assert!(owner_in_fight_with_mob(&mgr), "precondition");
    mgr.get_entity_mut(MOB).unwrap().faction = 0;

    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Fighting, "still fighting MOB_2");
    let o = mgr.get_entity(OWNER).unwrap();
    assert!(!o.threatened_mobs.contains(&MOB), "{:?}", o.threatened_mobs);
    assert!(o.threatened_mobs.contains(&MOB_2));
}
