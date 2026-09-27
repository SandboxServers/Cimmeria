//! A pet fights only what its owner could attack (`combat::player_may_attack`,
//! the #444 rule, pets PT-05): every stance pick, every target it keeps,
//! every threat it accepts and every fight it mirrors into its owner's
//! combat state. Plus the Aggressive scan's line-of-sight gate, which is the
//! NPC acquisition gate (`aggro_gates::same_room`) and fails closed where a
//! navmesh exists.

use cimmeria_entity::cell_entity::MobAggression;

use super::*;
use crate::cell::combat::{generate_threat, AggroCause, BSF_IN_COMBAT};
use crate::test_support::LogCapture;

/// A second player standing beside the owner.
const FRIEND: u32 = 8;

/// An NPC hostile to players by its aggression (a content `set_aggression`),
/// but not of the hostile faction: a player cannot attack it (#444).
fn add_hostile_but_unattackable(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    add_mob(mgr, id, pos, 0);
    mgr.get_entity_mut(id).unwrap().aggro.override_level = Some(MobAggression::Hostile);
    assert!(crate::cell::combat::is_hostile_to_players(
        mgr.get_entity(id).unwrap()
    ));
}

/// Aggressive: an NPC that aggroes players but that the owner cannot attack
/// is not scanned in.
#[tokio::test]
async fn aggressive_pet_leaves_an_npc_its_owner_cannot_attack() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_hostile_but_unattackable(&mut mgr, MOB, [10.0, 0.0, 14.0]);
    set_stance(&mut mgr, pet, PetStance::Aggressive);

    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
}

/// Defensive: a non-hostile-faction NPC a content chain set fighting the
/// owner is not "defended against"; the owner could not hit it either.
#[tokio::test]
async fn defensive_pet_does_not_attack_an_npc_its_owner_cannot_attack() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], 0);
    mob_fights(&mut mgr, MOB, OWNER);

    tick(&mut mgr).await;

    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
}

/// Player-to-pet threat: a friendly player's hit (or a content chain aiming
/// threat from a player) never turns a Defensive pet on that player, and says
/// why.
#[tokio::test]
async fn a_pet_refuses_threat_from_a_player() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_pet_owner(&mut mgr, FRIEND, "Agnos", [12.0, 0.0, 12.0], 5);
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);

    let logs = LogCapture::install();
    let _ = generate_threat(&mut mgr, FRIEND, pet, 50.0, AggroCause::Damage);

    assert_eq!(
        state(&mgr, pet),
        AiState::Follow,
        "a player's hit must not engage"
    );
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    let row = pets_ai_row(&logs, "pet_threat_refused").expect("refusal row");
    assert!(row.has_field("event", "threat_refused"), "{row:?}");
    assert!(row.has_field("reason", "attacker_not_hostile"), "{row:?}");
    assert!(row.has_field("target_id", &FRIEND.to_string()), "{row:?}");
    assert_owner_identity(&row);
    // Nothing leaks into the owner's combat state either.
    let owner = mgr.get_entity(OWNER).unwrap();
    assert!(owner.threatened_mobs.is_empty());
    assert_eq!(owner.state_field & BSF_IN_COMBAT, 0);
}

/// Keep side: a fighting pet drops a target its owner cannot attack (here a
/// hostile mob a content chain turned friendly mid-fight).
#[tokio::test]
async fn a_fighting_pet_drops_a_target_its_owner_cannot_attack() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], HOSTILE);
    tick(&mut mgr).await;
    let _ = generate_threat(&mut mgr, MOB, pet, 50.0, AggroCause::Damage);
    assert_eq!(state(&mgr, pet), AiState::Fighting, "precondition");
    mgr.get_entity_mut(MOB).unwrap().faction = 0;

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert!(!mgr.get_entity(pet).unwrap().threat_list.contains_key(&MOB));
    let row = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    assert!(row.has_field("reason", "target_not_hostile"), "{row:?}");
}

/// Owner combat mirror: a mob the owner cannot attack that still lists the
/// pet does not put the owner in combat.
#[tokio::test]
async fn the_owner_is_not_put_in_combat_by_an_npc_it_cannot_attack() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    add_mob(&mut mgr, MOB, [14.0, 0.0, 10.0], 0);
    mob_fights(&mut mgr, MOB, pet);

    tick(&mut mgr).await;

    let owner = mgr.get_entity(OWNER).unwrap();
    assert!(
        owner.threatened_mobs.is_empty(),
        "{:?}",
        owner.threatened_mobs
    );
    assert_eq!(owner.state_field & BSF_IN_COMBAT, 0);
}

/// Line of sight: on a navmeshed space the Aggressive scan fails closed on an
/// `Unknown` ray, exactly like NPC proximity aggro (D-NA08). The old scan let
/// `Unknown` through and only refused `Blocked`.
#[tokio::test]
async fn aggressive_scan_fails_closed_on_unknown_line_of_sight() {
    use cimmeria_cell_world::cell::arrival::{test_fixture_mesh, test_insert_navmesh_space};
    use cimmeria_entity::navigation::LineOfSight;
    let Some(mesh) = test_fixture_mesh() else {
        return;
    };
    let mut mgr = make_world();
    test_insert_navmesh_space(&mut mgr, "CellblockNav", mesh);
    // Far off the fixture mesh: every ray from here is `Unknown`, the case
    // the old `!= Blocked` test let in.
    let off_mesh = [5000.0_f32, 68.5, 5000.0];
    add_pet_owner(&mut mgr, OWNER, "CellblockNav", off_mesh, 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(OWNER_PLAYER_ID);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 0)
        .unwrap();
    set_stance(&mut mgr, pet, PetStance::Aggressive);
    mgr.spawn_npc(MOB, "CellblockNav", [5004.0, 68.5, 5000.0], [0.0; 3])
        .unwrap();
    let mob = mgr.get_entity_mut(MOB).unwrap();
    mob.faction = HOSTILE;
    if let Some(h) = mob.stats.get_mut(cimmeria_entity::stats::HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    assert_eq!(
        mgr.npc_line_of_sight(pet, MOB).los,
        LineOfSight::Unknown,
        "precondition: the ray is Unknown"
    );

    assert_eq!(
        super::super::stance::pick_engagement(&mgr, pet, OWNER, PetStance::Aggressive),
        None
    );
}
