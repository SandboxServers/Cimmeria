//! The owner's side of a pet's fight (D-PT06): pet threat puts the owner in
//! combat, and the owner leaves combat when that fight ends.

use super::*;
use crate::cell::combat::{clear_dead_npc_from_all_player_threat, BSF_IN_COMBAT};
use crate::test_support::LogCapture;

fn in_combat(mgr: &SpaceManager, id: u32) -> bool {
    mgr.get_entity(id).unwrap().state_field & BSF_IN_COMBAT != 0
}

/// A mob with the pet on its threat list puts the owner in combat, and the
/// owner is told (`onStateFieldUpdate` to the owner's own client).
#[tokio::test]
async fn pet_threat_puts_the_owner_in_combat() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, pet);
    assert!(!in_combat(&mgr, OWNER));

    let logs = LogCapture::install();
    let msgs = tick(&mut mgr).await;

    assert!(mgr
        .get_entity(OWNER)
        .unwrap()
        .threatened_mobs
        .contains(&MOB));
    assert!(in_combat(&mgr, OWNER));
    let sent = state_update_to(&msgs, OWNER).expect("onStateFieldUpdate to the owner");
    assert_ne!(sent & BSF_IN_COMBAT, 0);
    assert!(pets_ai_row(&logs, "owner_combat_entered").is_some());
}

/// The pet's mob dies: the owner leaves combat through the ordinary
/// dead-NPC sweep, although the owner is not on the mob's threat list.
#[tokio::test]
async fn owner_leaves_combat_when_the_pets_mob_dies() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, pet);
    tick(&mut mgr).await;
    assert!(in_combat(&mgr, OWNER), "precondition");

    let exits = clear_dead_npc_from_all_player_threat(&mut mgr, MOB);
    assert_eq!(exits.len(), 1, "{exits:?}");
    assert_eq!(exits[0].0, OWNER);
    assert!(!in_combat(&mgr, OWNER));
}

/// The mob dropped the pet (lost it, reset) without dying or leashing: the
/// next pet turn takes the owner out of combat.
#[tokio::test]
async fn owner_leaves_combat_when_nothing_explains_the_entry() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, pet);
    tick(&mut mgr).await;
    assert!(in_combat(&mgr, OWNER), "precondition");

    // Idle with an empty list: a Fighting mob with nobody left would leash,
    // and the leash drain would take the owner out first (the tick visits
    // NPCs in hash order), hiding whether the pet's reconcile works.
    let mob = mgr.get_entity_mut(MOB).unwrap();
    mob.threat_list.clear();
    crate::cell::service::npc_ai::force_ai_state(mob, AiState::Idle);
    let logs = LogCapture::install();
    let msgs = tick(&mut mgr).await;
    assert!(!in_combat(&mgr, OWNER));
    let sent = state_update_to(&msgs, OWNER).expect("onStateFieldUpdate to the owner");
    assert_eq!(sent & BSF_IN_COMBAT, 0);
    assert!(pets_ai_row(&logs, "owner_combat_left").is_some());
}

/// The negative for that reconcile: the owner's own fight is not the pet's to
/// end. A mob with the owner on its threat list keeps the owner in combat.
#[tokio::test]
async fn owners_own_fight_is_left_alone() {
    let (mut mgr, _pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [30.0, 0.0, 10.0], HOSTILE);
    let _ = crate::cell::combat::generate_threat(
        &mut mgr,
        OWNER,
        MOB,
        10.0,
        crate::cell::combat::AggroCause::Damage,
    );
    assert!(
        in_combat(&mgr, OWNER),
        "precondition: the owner hit the mob"
    );

    tick(&mut mgr).await;
    assert!(in_combat(&mgr, OWNER));
    assert!(mgr
        .get_entity(OWNER)
        .unwrap()
        .threatened_mobs
        .contains(&MOB));
}
