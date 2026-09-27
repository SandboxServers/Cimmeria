//! What a pet leaving a fight does to the mob's entry for it (pets PT-05):
//! released when the target stopped being fightable or the owner called the
//! pet off, kept when the pet is only pulled back by distance.

use super::*;
use crate::cell::combat::BSF_IN_COMBAT;
use crate::cell::service::npc_ai::pet::{engage_pet_target, PetEngagement};
use crate::test_support::LogCapture;

/// A two-sided fight between the pet and `MOB`, the owner mirrored into it.
async fn engaged(owner: [f32; 3], pet_at: [f32; 3], mob_at: [f32; 3]) -> (SpaceManager, u32) {
    let (mut mgr, pet) = world_with_pet(owner);
    move_to(&mut mgr, pet, pet_at);
    add_mob(&mut mgr, MOB, mob_at, HOSTILE);
    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB, PetEngagement::Automatic),
        Ok(())
    );
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
    assert_eq!(
        engage_pet_target(&mut mgr, pet, MOB_2, PetEngagement::Automatic),
        Ok(())
    );
    tick(&mut mgr).await;
    assert!(owner_in_fight_with_mob(&mgr), "precondition");
    mgr.get_entity_mut(MOB).unwrap().faction = 0;

    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Fighting, "still fighting MOB_2");
    let o = mgr.get_entity(OWNER).unwrap();
    assert!(!o.threatened_mobs.contains(&MOB), "{:?}", o.threatened_mobs);
    assert!(o.threatened_mobs.contains(&MOB_2));
}

/// A surrendered NPC the owner still has selected: `npc_ai_submit` has
/// disarmed it (aggression Neutral), so the Aggressive scan skips it, but
/// the owner's-target rule does not ask for aggression. Without the state
/// rule the pet re-engaged it every turn (and `npc_ai_submit` cleared the
/// fight again each pass), flapping the owner in and out of combat.
#[tokio::test]
async fn an_aggressive_pet_leaves_a_surrendered_owner_target_alone() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [10.0, 0.0, 16.0], HOSTILE);
    let mob = mgr.get_entity_mut(MOB).unwrap();
    crate::cell::service::npc_ai::force_ai_state(mob, AiState::Submit);
    mob.aggro.override_level = Some(cimmeria_entity::cell_entity::MobAggression::Neutral);
    set_stance(&mut mgr, pet, PetStance::Aggressive);
    let owner = mgr.get_entity_mut(OWNER).unwrap();
    owner.state_field |= BSF_IN_COMBAT;
    owner.current_target_id = Some(MOB as i32);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(!mob_lists_pet(&mgr, pet));
    assert!(pets_ai_row(&logs, "pet_engaged").is_none());
    assert!(!mgr
        .get_entity(OWNER)
        .unwrap()
        .threatened_mobs
        .contains(&MOB));
}

/// The target surrenders mid-fight: the pet drops it and it forgets the pet.
#[tokio::test]
async fn a_target_that_surrenders_is_dropped_and_forgets_the_pet() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    tick(&mut mgr).await;
    crate::cell::service::npc_ai::force_ai_state(mgr.get_entity_mut(MOB).unwrap(), AiState::Submit);
    // Put the pet back on the mob's list, as a fight still in flight would.
    mgr.get_entity_mut(MOB)
        .unwrap()
        .threat_list
        .insert(pet, 5.0);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert!(!mgr.get_entity(pet).unwrap().threat_list.contains_key(&MOB));
    assert!(!mob_lists_pet(&mgr, pet));
    let row = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    assert!(row.has_field("reason", "target_not_engageable"), "{row:?}");
}

/// The target dies (killed by anyone): on the pet's next turn the corpse is
/// off its list, the pet is back in Follow, and the owner is out of combat
/// with it.
#[tokio::test]
async fn the_pets_target_dying_ends_the_fight_cleanly() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    tick(&mut mgr).await;
    assert!(owner_in_fight_with_mob(&mgr), "precondition");
    let (tx, _rx) = mpsc::channel(256);
    assert!(
        crate::cell::abilities::kill_npc_out_of_band(MOB, OWNER, true, false, &tx, &mut mgr).await
    );

    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(!mgr.get_entity(pet).unwrap().threat_list.contains_key(&MOB));
    assert!(!owner_in_fight_with_mob(&mgr));
    assert!(!mgr
        .get_entity(OWNER)
        .unwrap()
        .threatened_mobs
        .contains(&MOB));
}

/// A dismissed (despawned) pet: nothing in the pet AI releases it, the mob's
/// fight handler prunes the vanished target, and the mob's own reset takes
/// the owner out of combat with it.
#[tokio::test]
async fn a_dismissed_pets_attacker_lets_it_go_and_the_owner_leaves_combat() {
    let (mut mgr, pet) = engaged([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]).await;
    tick(&mut mgr).await;
    assert!(owner_in_fight_with_mob(&mgr), "precondition");
    let (tx, _rx) = mpsc::channel(256);
    let _ = crate::cell::pets::despawn_pet(
        &mut mgr,
        pet,
        crate::cell::pets::PetDespawnReason::Dismissed,
        &tx,
    )
    .await;
    assert!(
        mgr.get_entity(pet).is_none(),
        "precondition: the pet is gone"
    );

    tick(&mut mgr).await;

    assert!(!mob_lists_pet(&mgr, pet), "the mob pruned the vanished pet");
    assert!(
        !owner_in_fight_with_mob(&mgr),
        "and the owner left the fight"
    );
}

/// A target in another space on the pet's list (however it got there) is
/// dropped as `target_other_space` and forgets the pet.
#[tokio::test]
async fn a_target_in_another_space_is_dropped() {
    let (mut mgr, pet) = two_space_world_with_pet();
    add_mob_in(&mut mgr, MOB, "Castle", [10.0, 0.0, 8.0], HOSTILE);
    let p = mgr.get_entity_mut(pet).unwrap();
    crate::cell::service::npc_ai::force_ai_state(p, AiState::Fighting);
    p.threat_list.insert(MOB, 10.0);
    mgr.get_entity_mut(MOB)
        .unwrap()
        .threat_list
        .insert(pet, 10.0);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert!(!mgr.get_entity(pet).unwrap().threat_list.contains_key(&MOB));
    assert!(!mob_lists_pet(&mgr, pet));
    let row = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    assert!(row.has_field("reason", "target_other_space"), "{row:?}");
}

/// Nor does a pet take threat from an attacker in another space.
#[tokio::test]
async fn a_pet_refuses_threat_from_another_space() {
    let (mut mgr, pet) = two_space_world_with_pet();
    add_mob_in(&mut mgr, MOB, "Castle", [10.0, 0.0, 8.0], HOSTILE);

    let logs = LogCapture::install();
    let _ = crate::cell::combat::generate_threat(
        &mut mgr,
        MOB,
        pet,
        50.0,
        crate::cell::combat::AggroCause::Damage,
    );

    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    assert_ne!(state(&mgr, pet), AiState::Fighting);
    let row = pets_ai_row(&logs, "pet_threat_refused").expect("refusal row");
    assert!(row.has_field("reason", "attacker_other_space"), "{row:?}");
}
