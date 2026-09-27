//! 2824 Holy Warrior: a toggle that gives the owner's pet +100 Accuracy and
//! -100 Defense.

use cimmeria_entity::stats::{ACCURACY, DEFENSE};
use tokio::sync::mpsc;

use super::*;
use crate::cell::abilities::use_ability::handle_use_ability;

/// **Regression guard (PT-08).** The first press buffs the owner's pet, not
/// the client's target: the owner aims at the other player, whom the #444
/// gate would refuse, and the pet gets the buff. The second press takes it
/// off. Each press tells the owner the new state, and the pet's stats reach
/// its witnesses. Fails when the redirect is removed (the press is refused
/// by #444 and nothing changes) or the toggle is (the second press stacks).
#[tokio::test]
async fn holy_warrior_toggles_the_pets_buff_on_and_off() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, OTHER as i32, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(stat(&mgr, pet, ACCURACY), 100);
    assert_eq!(stat(&mgr, pet, DEFENSE), -100);
    assert_eq!(
        stat(&mgr, OTHER, ACCURACY),
        0,
        "the client's target is ignored"
    );
    assert!(mgr.has_pet_buff(pet, E_HOLY_WARRIOR));
    assert!(got_line(&sent, OWNER, "Holy Warrior is on."));
    assert!(
        stat_update_to(&sent, OWNER, pet),
        "the owner sees the pet's stats change"
    );
    assert!(error_codes(&sent, OWNER).is_empty());

    ready_again(&mut mgr);
    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(stat(&mgr, pet, ACCURACY), 0);
    assert_eq!(stat(&mgr, pet, DEFENSE), 0);
    assert!(!mgr.has_pet_buff(pet, E_HOLY_WARRIOR));
    assert!(got_line(&sent, OWNER, "Holy Warrior is off."));
}

/// Defense defaults to `[0, 0]`, which would clamp the -100 away. The pet's
/// bound widens to admit it, and toggling off restores exactly the delta
/// applied: an Accuracy change made meanwhile by something else survives.
#[tokio::test]
async fn holy_warrior_widens_defense_and_restores_only_its_own_delta() {
    let (mut mgr, pet) = world();
    let (tx, _rx) = mpsc::channel(256);
    let defense = mgr.get_entity(pet).unwrap().stats.get(DEFENSE).cloned();
    assert_eq!(defense.map(|s| (s.min, s.cur, s.max)), Some((0, 0, 0)));

    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    let defense = mgr
        .get_entity(pet)
        .unwrap()
        .stats
        .get(DEFENSE)
        .cloned()
        .unwrap();
    assert_eq!((defense.min, defense.cur), (-100, -100));

    // Something else moves Accuracy while the toggle is on.
    mgr.get_entity_mut(pet)
        .unwrap()
        .stats
        .get_mut(ACCURACY)
        .unwrap()
        .change(7);
    ready_again(&mut mgr);
    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    assert_eq!(
        stat(&mgr, pet, ACCURACY),
        7,
        "only the toggle's +100 is taken back"
    );
    assert_eq!(stat(&mgr, pet, DEFENSE), 0);
}

/// The client names another player's pet: the caster's own pet is buffed
/// and the named pet is not. The target is never read.
#[tokio::test]
async fn holy_warrior_never_buffs_a_pet_the_client_names() {
    let (mut mgr, pet) = world();
    let their_pet = mgr
        .spawn_pet_from_template(
            OTHER,
            cimmeria_cell_world::test_fixtures::PET_FIXTURE_TEMPLATE_ID,
            1643,
        )
        .expect("the other player's pet");
    let (tx, _rx) = mpsc::channel(256);

    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, their_pet as i32, &tx, &mut mgr).await);
    assert_eq!(stat(&mgr, pet, ACCURACY), 100);
    assert_eq!(stat(&mgr, their_pet, ACCURACY), 0);
    assert!(!mgr.has_pet_buff(their_pet, E_HOLY_WARRIOR));
}

/// The pet leaves (dismissed, replaced): its buff goes with the entity, and
/// the owner's next press finds no pet instead of an "on" state to clear.
#[tokio::test]
async fn a_despawned_pet_takes_its_toggle_with_it() {
    let (mut mgr, pet) = world();
    let (tx, mut rx) = mpsc::channel(256);
    assert!(handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    let _ = cimmeria_cell_world::cell::pets::despawn_pet(
        &mut mgr,
        pet,
        cimmeria_cell_world::cell::pets::PetDespawnReason::Dismissed,
        &tx,
    )
    .await;
    let _ = drain(&mut rx);
    ready_again(&mut mgr);

    assert!(!handle_use_ability(OWNER, HOLY_WARRIOR, 0, &tx, &mut mgr).await);
    let sent = drain(&mut rx);
    assert_eq!(error_codes(&sent, OWNER), vec![(HOLY_WARRIOR, 190)]);
    assert!(got_line(&sent, OWNER, "You have no pet to use that on."));
}
