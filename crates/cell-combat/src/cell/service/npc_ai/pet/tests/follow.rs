//! Following the owner and the teleport back (D-PT07).

use cimmeria_entity::cell_entity::PetState;
use std::time::{Duration, Instant};

use super::*;
use crate::test_support::LogCapture;

/// A freshly summoned pet is Idle, and an Idle pet is neither hostile nor
/// has a patrol or wander, so before PT-05 the tick never admitted it and it
/// stood where it was summoned. It must be armed to follow its owner.
#[tokio::test]
async fn idle_pet_is_armed_to_follow_its_owner() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    assert_eq!(state(&mgr, pet), AiState::Idle, "precondition: spawns Idle");
    move_to(&mut mgr, OWNER, [30.0, 0.0, 10.0]);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    let e = mgr.get_entity(pet).unwrap();
    assert_eq!(e.ai_state(), AiState::Follow);
    assert_eq!(e.follow_target_id, Some(OWNER));
    assert_eq!((e.follow_min_distance, e.follow_max_distance), (2.0, 5.0));
    let row = pets_ai_row(&logs, "pet_follow_armed").expect("pet_follow_armed row");
    assert!(row.has_field("from", "idle"), "{row:?}");
}

/// More than 40 u behind: the pet is put beside its owner in one tick.
#[tokio::test]
async fn pet_left_far_behind_teleports_beside_its_owner() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    move_to(&mut mgr, OWNER, [70.0, 0.0, 10.0]);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    let p = pos(&mgr, pet);
    assert!(
        p.distance_to(&pos(&mgr, OWNER)) <= 2.5,
        "pet must land beside the owner at (70, 0, 10), got {p:?}"
    );
    let e = mgr.get_entity(pet).unwrap();
    assert!(e
        .extensions
        .get::<PetState>()
        .unwrap()
        .last_teleport_at
        .is_some());
    assert!(e.nav_path.is_empty(), "the snap stops the pet");
    let row = pets_ai_row(&logs, "pet_teleported").expect("pet_teleported row");
    assert!(row.has_field("reason", "distance"), "{row:?}");
    assert_owner_identity(&row);
    assert!(row.has_field("event", "teleported"), "{row:?}");
    // The move itself is PT-02's, and says which path moved the pet.
    let moved = logs
        .all()
        .into_iter()
        .find(|c| c.target == "pets.lifecycle" && c.has_field("event", "owner_teleported"))
        .expect("PT-02's owner_teleported row");
    assert!(moved.has_field("path", "pet_left_behind"), "{moved:?}");
}

/// The owner went up a floor: the pet follows it there even though it is
/// close horizontally.
#[tokio::test]
async fn pet_on_another_floor_teleports() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    move_to(&mut mgr, OWNER, [12.0, 7.0, 10.0]);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert!(
        (pos(&mgr, pet).y - 7.0).abs() < 0.01,
        "{:?}",
        pos(&mgr, pet)
    );
    let row = pets_ai_row(&logs, "pet_teleported").expect("pet_teleported row");
    assert!(row.has_field("reason", "floor_band"), "{row:?}");
}

/// Inside 40 u and on the owner's floor the pet walks; it never teleports.
#[tokio::test]
async fn pet_within_teleport_distance_walks() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let before = pos(&mgr, pet);
    move_to(&mut mgr, OWNER, [45.0, 1.0, 10.0]);

    let logs = LogCapture::install();
    tick(&mut mgr).await;

    assert_eq!(pos(&mgr, pet), before, "no snap: the follow handler walks");
    assert!(pets_ai_row(&logs, "pet_teleported").is_none());
    assert_eq!(state(&mgr, pet), AiState::Follow);
}

/// One teleport per 5 s (D-PT07): a pet left behind again 1 s after a
/// teleport walks instead, and teleports once the interval has passed.
#[tokio::test]
async fn teleport_is_rate_limited_to_once_per_five_seconds() {
    let (mut mgr, pet) = world_with_pet([10.0, 0.0, 10.0]);
    let recent = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    mgr.get_entity_mut(pet)
        .unwrap()
        .extensions
        .get_mut::<PetState>()
        .unwrap()
        .last_teleport_at = Some(recent);
    let before = pos(&mgr, pet);
    move_to(&mut mgr, OWNER, [90.0, 0.0, 10.0]);

    let logs = LogCapture::install();
    tick(&mut mgr).await;
    assert_eq!(pos(&mgr, pet), before, "rate-limited: no snap");
    assert!(pets_ai_row(&logs, "pet_teleport_rate_limited").is_some());
    assert!(pets_ai_row(&logs, "pet_teleported").is_none());

    let old = Instant::now().checked_sub(Duration::from_secs(6)).unwrap();
    mgr.get_entity_mut(pet)
        .unwrap()
        .extensions
        .get_mut::<PetState>()
        .unwrap()
        .last_teleport_at = Some(old);
    tick(&mut mgr).await;
    assert!(pos(&mgr, pet).distance_to(&pos(&mgr, OWNER)) <= 2.5);
    assert!(pets_ai_row(&logs, "pet_teleported").is_some());
}
