//! The owner-anchored leash and the re-arm after a fight (A-28, A-29): a pet
//! never walks to `spawn_position`, and a finished fight goes back to Follow.

use super::*;
use crate::cell::combat::{generate_threat, AggroCause, BSF_DEAD};
use crate::test_support::LogCapture;
use cimmeria_entity::cell_entity::PetState;

/// A pet in a fight against `MOB`, with its spawn point pinned at `spawn`.
fn fighting_pet(owner: [f32; 3], pet_at: [f32; 3], mob_at: [f32; 3]) -> (SpaceManager, u32) {
    let (mut mgr, pet) = world_with_pet(owner);
    move_to(&mut mgr, pet, pet_at);
    add_mob(&mut mgr, MOB, mob_at, HOSTILE);
    let _ = generate_threat(&mut mgr, MOB, pet, 50.0, AggroCause::Damage);
    assert_eq!(state(&mgr, pet), AiState::Fighting, "precondition");
    (mgr, pet)
}

fn set_spawn(mgr: &mut SpaceManager, id: u32, at: [f32; 3]) {
    mgr.get_entity_mut(id).unwrap().spawn_position = Some(Vector3::new(at[0], at[1], at[2]));
}

/// 200 u from where it was summoned, but beside its owner: the pet keeps
/// fighting. Measured from `spawn_position`, it would have leashed.
#[tokio::test]
async fn pet_far_from_its_spawn_but_near_its_owner_keeps_fighting() {
    let (mut mgr, pet) = fighting_pet([200.0, 0.0, 10.0], [200.0, 0.0, 8.0], [204.0, 0.0, 8.0]);
    set_spawn(&mut mgr, pet, [0.0, 0.0, 0.0]);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert!(
        pets_ai_row(&logs, "pet_follow_rearmed").is_none(),
        "no leash: the anchor is the owner, 2 u away"
    );
}

/// Dragged past the leash radius from its owner, the pet gives the fight up
/// and goes straight back to Follow: not Leashing, no route to its spawn,
/// not healed. It stands at its spawn point here, so a spawn-anchored leash
/// could not have fired at all. The target stays within 40 u of the owner,
/// so it is not simply dropped as left behind: the leash itself fires.
#[tokio::test]
async fn pet_past_the_leash_from_its_owner_rearms_follow_without_walking_home() {
    let (mut mgr, pet) = fighting_pet([100.0, 0.0, 10.0], [0.0, 0.0, 0.0], [70.0, 0.0, 10.0]);
    set_spawn(&mut mgr, pet, [0.0, 0.0, 0.0]);
    if let Some(h) = mgr
        .get_entity_mut(pet)
        .unwrap()
        .stats
        .get_mut(cimmeria_entity::stats::HEALTH)
    {
        let max = h.max;
        h.set_current(max / 2);
    }
    let hp = |m: &SpaceManager| {
        m.get_entity(pet)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .unwrap()
            .cur
    };
    let hp_before = hp(&mgr);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    let e = mgr.get_entity(pet).unwrap();
    assert_eq!(e.ai_state(), AiState::Follow, "not Leashing");
    assert!(e.threat_list.is_empty());
    assert_eq!(e.follow_target_id, Some(OWNER));
    assert!(
        e.nav_path.is_empty(),
        "no route home was installed: {:?}",
        e.nav_path
    );
    assert_eq!(hp(&mgr), hp_before, "a pet is not healed by a leash");
    let row = pets_ai_row(&logs, "pet_follow_rearmed").expect("rearm row");
    assert!(row.has_field("reason", "leash_out"), "{row:?}");
    assert!(row.has_field("event", "follow_rearmed"), "{row:?}");
    assert_owner_identity(&row);
}

/// The target died: the fight ends in Follow, not in the walk home and not
/// in Idle (A-29). The pre-pass drops the corpse (`target_dead`) before the
/// fight handler runs, so the re-arm sees an empty list.
#[tokio::test]
async fn pet_rearms_follow_after_its_target_dies() {
    let (mut mgr, pet) = fighting_pet([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]);
    mgr.get_entity_mut(MOB).unwrap().state_field |= BSF_DEAD;

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert_eq!(mgr.get_entity(pet).unwrap().follow_target_id, Some(OWNER));
    let drop = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    assert!(drop.has_field("reason", "target_dead"), "{drop:?}");
    let row = pets_ai_row(&logs, "pet_follow_rearmed").expect("rearm row");
    assert!(row.has_field("trigger", "threat_empty"), "{row:?}");
}

/// Leashing set from outside the AI (content, the GM console): the pet's
/// next turn puts it back on its owner instead of walking to its spawn.
#[tokio::test]
async fn a_leashing_pet_is_put_back_on_its_owner() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    set_spawn(&mut mgr, pet, [-300.0, 0.0, 0.0]);
    add_mob(&mut mgr, MOB, [60.0, 0.0, 10.0], HOSTILE);
    let e = mgr.get_entity_mut(pet).unwrap();
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Leashing);
    // Content can leave threat behind; the re-arm must clear it.
    e.threat_list.insert(MOB, 5.0);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    let e = mgr.get_entity(pet).unwrap();
    assert_eq!(e.ai_state(), AiState::Follow);
    assert!(e.threat_list.is_empty(), "{:?}", e.threat_list);
    let row = pets_ai_row(&logs, "pet_follow_rearmed").expect("rearm row");
    assert!(row.has_field("trigger", "leashing"), "{row:?}");
    assert!(
        e.nav_path.back().is_none_or(|end| end.x > -100.0),
        "no route toward the spawn at x = -300: {:?}",
        e.nav_path
    );
}

/// After giving up a fight at the owner-anchored leash, a pet whose teleport
/// is still rate-limited must not re-engage the mob that is still fighting
/// it: it would leash again at once, every turn, for up to 5 s.
#[tokio::test]
async fn a_pet_left_behind_does_not_re_engage_while_its_teleport_waits() {
    let (mut mgr, pet) = world_with_pet([100.0, 0.0, 10.0]);
    move_to(&mut mgr, pet, [0.0, 0.0, 0.0]);
    // Out of its attack range, so it chases rather than hits: a hit would
    // preempt the pet into Fighting by itself, which is not what is tested.
    add_mob(&mut mgr, MOB, [0.0, 0.0, 35.0], HOSTILE);
    mob_fights(&mut mgr, MOB, pet);
    let recent = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(1))
        .unwrap();
    let e = mgr.get_entity_mut(pet).unwrap();
    e.extensions.get_mut::<PetState>().unwrap().last_teleport_at = Some(recent);
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Follow);
    e.follow_target_id = Some(OWNER);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    // The flap is engage-then-leash inside one turn, which ends in Follow
    // too; only the rows show it.
    assert!(
        pets_ai_row(&logs, "pet_engaged").is_none(),
        "a left-behind pet must not engage"
    );
    assert!(pets_ai_row(&logs, "pet_follow_rearmed").is_none());
    assert_eq!(
        state(&mgr, pet),
        AiState::Follow,
        "no re-engage while left behind"
    );
    assert!(
        mgr.get_entity(pet).unwrap().threat_list.is_empty(),
        "{:?} {:?}",
        mgr.get_entity(pet).unwrap().threat_list,
        mgr.get_entity(MOB).unwrap().ai_state()
    );
}

/// A pet's target that gave up and is walking home evades; the pet drops it
/// instead of chasing it home and re-pulling it there.
#[tokio::test]
async fn a_fighting_pet_drops_a_target_that_is_walking_home() {
    let (mut mgr, pet) = fighting_pet([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]);
    crate::cell::service::npc_ai::force_ai_state(
        mgr.get_entity_mut(MOB).unwrap(),
        AiState::Leashing,
    );

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(
        mgr.get_entity(pet).unwrap().threat_list.is_empty(),
        "{:?} {:?}",
        mgr.get_entity(pet).unwrap().threat_list,
        mgr.get_entity(MOB).unwrap().ai_state()
    );
    let row = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    // `target_just_reset` when the mob finished its reset earlier in the
    // same tick (the tick visits NPCs in hash order).
    assert!(
        row.has_field("reason", "target_resetting") || row.has_field("reason", "target_just_reset"),
        "{row:?}"
    );
    assert!(row.has_field("target_id", &MOB.to_string()), "{row:?}");
}

/// A target that has just finished its leash reset is back at home and Idle,
/// inside its re-aggro window. The pet drops it too: hitting it now would pull
/// it straight back into Fighting.
#[tokio::test]
async fn a_fighting_pet_drops_a_target_that_just_reset() {
    let (mut mgr, pet) = fighting_pet([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]);
    let mob = mgr.get_entity_mut(MOB).unwrap();
    crate::cell::service::npc_ai::force_ai_state(mob, AiState::Idle);
    mob.leash.reaggro_suppressed_until =
        Some(std::time::Instant::now() + std::time::Duration::from_secs(5));

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    let row = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    assert!(row.has_field("reason", "target_just_reset"), "{row:?}");
}

/// The owner teleported (PT-02 moved the pet beside it): the pet's old
/// target, now far from the owner, is dropped and the pet follows again
/// instead of running back to it.
#[tokio::test]
async fn after_the_owner_teleports_the_pet_drops_its_old_target() {
    let (mut mgr, pet) = fighting_pet([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]);
    move_to(&mut mgr, OWNER, [300.0, 0.0, 10.0]);
    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    assert_eq!(
        crate::cell::pets::on_owner_teleported(
            OWNER,
            crate::cell::pets::OwnerPath::GmTravel,
            &tx,
            &mut mgr
        )
        .await,
        1
    );
    assert_eq!(state(&mgr, pet), AiState::Fighting, "PT-02 keeps the state");

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(state(&mgr, pet), AiState::Follow);
    assert!(mgr.get_entity(pet).unwrap().threat_list.is_empty());
    let row = pets_ai_row(&logs, "pet_target_dropped").expect("drop row");
    assert!(row.has_field("reason", "target_far_from_owner"), "{row:?}");
    assert_owner_identity(&row);
}

/// The dispatcher's state snapshot is taken before the loop. A mob that ran
/// earlier in the same tick can preempt the pet into Fighting; the pre-pass
/// must act on that, not on the stale `Follow` snapshot, or it re-arms Follow
/// and leaves the pet with a live threat list and no fight.
#[tokio::test]
async fn the_pre_pass_acts_on_the_pets_current_state_not_the_snapshot() {
    let (mut mgr, pet) = fighting_pet([10.0, 0.0, 10.0], [10.0, 0.0, 8.0], [14.0, 0.0, 8.0]);
    let (tx, _rx) = tokio::sync::mpsc::channel(64);

    let run = super::super::pre_pass(pet, AiState::Follow, &tx, &mut mgr).await;
    assert_eq!(run, Some(AiState::Fighting));
    assert_eq!(state(&mgr, pet), AiState::Fighting);
    assert!(mgr.get_entity(pet).unwrap().threat_list.contains_key(&MOB));
}
