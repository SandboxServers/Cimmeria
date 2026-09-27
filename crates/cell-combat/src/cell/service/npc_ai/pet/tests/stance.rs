//! Stances (D-PT09): Passive never engages, Defensive defends the owner and
//! itself, Aggressive also takes the owner's target and nearby hostiles.

use super::*;
use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
use crate::test_support::LogCapture;

fn threat_on(mgr: &SpaceManager, pet: u32, mob: u32) -> bool {
    mgr.get_entity(pet).unwrap().threat_list.contains_key(&mob)
}

/// A stance engagement is two-sided, and the owner is in the fight the same
/// turn: the mob is Fighting with the pet on its threat list (so it fights
/// back), the pet lists the mob, and the owner is mirrored into combat with
/// it. Checking only the pet's side let an engagement that never touched the
/// mob pass (Copilot, #896).
fn assert_engaged_both_ways(mgr: &SpaceManager, pet: u32, mob: u32) {
    assert!(threat_on(mgr, pet, mob), "the pet lists the mob");
    let m = mgr.get_entity(mob).unwrap();
    assert_eq!(m.ai_state(), AiState::Fighting, "the mob fights back");
    assert!(m.threat_list.contains_key(&pet), "the mob lists the pet");
    let owner = mgr.get_entity(OWNER).unwrap();
    assert!(
        owner.threatened_mobs.contains(&mob),
        "the owner is mirrored into the fight"
    );
    assert_ne!(owner.state_field & BSF_IN_COMBAT, 0, "and is in combat");
}

/// The negative: a Passive pet that is hit takes no threat and does not
/// leave Follow. Without the refusal `generate_threat` preempts every NPC
/// into Fighting on the first hit.
#[tokio::test]
async fn passive_pet_does_not_engage_when_hit() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [20.0, 0.0, 10.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Passive);
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);

    let logs = LogCapture::install();
    let _ = generate_threat(&mut mgr, MOB, pet, 50.0, AggroCause::Damage);
    assert_eq!(state(&mgr, pet), AiState::Follow, "a hit must not engage");
    assert!(!threat_on(&mgr, pet, MOB), "and must not add threat");
    let row = pets_ai_row(&logs, "pet_passive_ignored").expect("refusal row");
    assert!(row.has_field("cause", "damage"), "{row:?}");
    assert!(row.has_field("event", "passive_ignored"), "{row:?}");
    assert!(row.has_field("reason", "passive_stance"), "{row:?}");
    assert!(row.has_field("target_id", &MOB.to_string()), "{row:?}");
    assert_owner_identity(&row);

    // The mob keeps attacking; the next turn still does not engage.
    mob_fights(&mut mgr, MOB, pet);
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
}

/// Passive also ignores whatever attacks its owner.
#[tokio::test]
async fn passive_pet_does_not_defend_its_owner() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [20.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, OWNER);
    set_stance(&mut mgr, pet, PetStance::Passive);

    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(!threat_on(&mgr, pet, MOB));
}

/// Switched to Passive mid-fight: the pet drops the fight on its next turn.
#[tokio::test]
async fn passive_mid_fight_disengages() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    let _ = generate_threat(&mut mgr, MOB, pet, 50.0, AggroCause::Damage);
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    set_stance(&mut mgr, pet, PetStance::Passive);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    let row = pets_ai_row(&logs, "pet_follow_rearmed").expect("rearm row");
    assert!(row.has_field("trigger", "passive_stance"), "{row:?}");
}

/// Defensive (the default): a mob fighting the owner is engaged.
#[tokio::test]
async fn defensive_pet_defends_its_owner() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [25.0, 0.0, 10.0], HOSTILE);
    mob_fights(&mut mgr, MOB, OWNER);
    assert_eq!(
        mgr.get_entity(pet).unwrap().pet.as_deref().unwrap().stance,
        PetStance::Defensive
    );

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert_engaged_both_ways(&mgr, pet, MOB);
    let row = pets_ai_row(&logs, "pet_engaged").expect("pet_engaged row");
    assert!(row.has_field("why", "defend_owner"), "{row:?}");
    assert!(row.has_field("event", "engaged"), "{row:?}");
    assert!(row.has_field("target_id", &MOB.to_string()), "{row:?}");
    assert_owner_identity(&row);
    assert_eq!(
        mgr.get_entity(pet).unwrap().follow_target_id,
        Some(OWNER),
        "the follow target survives the fight"
    );
}

/// Defensive fights back when hit (the ordinary damage preempt).
#[tokio::test]
async fn defensive_pet_fights_back_when_hit() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [20.0, 0.0, 10.0], HOSTILE);
    tick(&mut mgr).await;
    let _ = generate_threat(&mut mgr, MOB, pet, 50.0, AggroCause::Damage);
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert!(threat_on(&mgr, pet, MOB));
}

/// Defensive leaves a hostile mob alone that attacks nobody.
#[tokio::test]
async fn defensive_pet_ignores_an_idle_hostile() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [10.0, 0.0, 16.0], HOSTILE);
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(!threat_on(&mgr, pet, MOB));
}

/// Aggressive engages a hostile NPC within 15 u of the pet.
#[tokio::test]
async fn aggressive_pet_engages_a_nearby_hostile() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [10.0, 0.0, 18.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Aggressive);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert_engaged_both_ways(&mgr, pet, MOB);
    let row = pets_ai_row(&logs, "pet_engaged").expect("pet_engaged row");
    assert!(row.has_field("why", "aggressive_scan"), "{row:?}");
}

/// The scan's edges: beyond 15 u, or not hostile, is left alone.
#[tokio::test]
async fn aggressive_scan_skips_far_and_friendly_npcs() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    // The pet stands at (10, 0, 8): 20 u away, and a friendly 5 u away.
    add_mob(&mut mgr, MOB, [10.0, 0.0, 28.0], HOSTILE);
    add_mob(&mut mgr, MOB_2, [15.0, 0.0, 8.0], 0);
    set_stance(&mut mgr, pet, PetStance::Aggressive);

    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
}

/// Aggressive takes the owner's target once the owner is in combat, even
/// beyond the 15 u scan; Defensive does not.
#[tokio::test]
async fn aggressive_pet_takes_the_owners_target_in_combat() {
    for (stance, engages) in [(PetStance::Aggressive, true), (PetStance::Defensive, false)] {
        let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
        add_mob(&mut mgr, MOB, [10.0, 0.0, 35.0], HOSTILE);
        set_stance(&mut mgr, pet, stance);
        let owner = mgr.get_entity_mut(OWNER).unwrap();
        owner.state_field |= BSF_IN_COMBAT;
        owner.current_target_id = Some(MOB as i32);

        let logs = LogCapture::install();
        tick(&mut mgr).await;
        assert_eq!(threat_on(&mgr, pet, MOB), engages, "{stance:?}");
        if engages {
            assert_engaged_both_ways(&mgr, pet, MOB);
            let row = pets_ai_row(&logs, "pet_engaged").expect("pet_engaged row");
            assert!(row.has_field("why", "owner_target"), "{row:?}");
        }
    }
}

/// The owner merely targeting a mob, out of combat, does not send the pet.
#[tokio::test]
async fn owner_target_out_of_combat_is_not_engaged() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [10.0, 0.0, 35.0], HOSTILE);
    set_stance(&mut mgr, pet, PetStance::Aggressive);
    mgr.get_entity_mut(OWNER).unwrap().current_target_id = Some(MOB as i32);

    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
}

/// A pet is never hostile to players, even with a hostile override, so it
/// never runs the player proximity scan or joins an assist.
#[test]
fn a_pet_is_never_hostile_to_players() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let e = mgr.get_entity_mut(pet).unwrap();
    e.faction = HOSTILE;
    e.aggro.override_level = Some(cimmeria_entity::cell_entity::MobAggression::Hostile);
    assert!(!crate::cell::combat::is_hostile_to_players(e));
}
