//! The summon VFX queue (PT-03): scrubbed with its pet, dropped when the
//! owner never sees the pet, never queued for a non-pet.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::Level;

use crate::test_support::LogCapture;

use super::*;
use crate::cell::pets::{
    despawn_pet, drain_arrivals, PetArrival, PetDespawnReason, ARRIVAL_TIMEOUT,
};

fn arrival() -> PetArrival {
    PetArrival::new(OWNER, 2293, vec![1, 2, 3])
}

/// Every teardown path goes through `forget_pet`, which drops the queued
/// VFX, so a despawned pet's effect can never play on a recycled id.
#[tokio::test]
async fn despawning_the_pet_drops_its_queued_vfx() {
    let (mut mgr, pet) = world_with_pet();
    mgr.pets.queue_arrival(pet, arrival());
    assert!(mgr.pets.pending_arrival(pet).is_some());
    let (tx, _rx) = mpsc::channel(64);

    let _outcome = despawn_pet(&mut mgr, pet, PetDespawnReason::Dismissed, &tx).await;

    assert!(mgr.pets.pending_arrival(pet).is_none());
}

/// An owner who never witnesses the pet (the intro went missing) does not
/// hold the VFX forever: it is dropped after `ARRIVAL_TIMEOUT`, unsent.
#[tokio::test]
async fn vfx_is_dropped_when_the_owner_never_witnesses_the_pet() {
    let (mut mgr, pet) = world_with_pet();
    mgr.pets.queue_arrival(pet, arrival());
    let (tx, mut rx) = mpsc::channel(64);

    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 0);
    assert!(mgr.pets.pending_arrival(pet).is_some(), "still waiting");

    let late = Instant::now() + ARRIVAL_TIMEOUT + Duration::from_millis(10);
    assert_eq!(drain_arrivals(late, &tx, &mut mgr).await, 0);
    assert!(mgr.pets.pending_arrival(pet).is_none(), "dropped");
    assert!(rx.try_recv().is_err(), "nothing sent");
}

/// The queue only holds registered pets.
#[test]
fn queue_arrival_ignores_an_entity_that_is_not_a_pet() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.pets.queue_arrival(OWNER, arrival());
    assert!(mgr.pets.pending_arrival(OWNER).is_none());
}

/// Seam: the 2 s drop is a WARN with `reason=owner_never_witnessed` and the
/// owner's identity (the intro went missing; no client can cause it).
#[tokio::test]
async fn never_witnessed_drop_logs_warn_with_reason() {
    let (mut mgr, pet) = world_with_pet();
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(77);
    mgr.pets.queue_arrival(pet, arrival());
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    let late = Instant::now() + ARRIVAL_TIMEOUT + Duration::from_millis(10);
    drain_arrivals(late, &tx, &mut mgr).await;
    let c = logs
        .find_event(Level::WARN, "summon VFX dropped", "owner_never_witnessed")
        .expect("WARN arrival_vfx_dropped reason=owner_never_witnessed");
    assert!(c.has_field("event", "arrival_vfx_dropped"), "{c:?}");
    assert!(c.has_field("pet_id", &pet.to_string()), "{c:?}");
    assert!(c.has_field("owner_id", &OWNER.to_string()), "{c:?}");
    assert!(c.has_field("account_id", &OWNER.to_string()), "{c:?}");
    assert!(c.has_field("player_id", "77"), "{c:?}");
}

/// Seam: a queued VFX whose owner no longer matches the registry (an id
/// reused past a missed scrub) is dropped with a WARN
/// `reason=owner_mismatch`, and never sent.
#[tokio::test]
async fn owner_mismatch_drop_logs_warn_with_reason() {
    let (mut mgr, pet) = world_with_pet();
    mgr.pets
        .queue_arrival(pet, PetArrival::new(OTHER, 2293, vec![1, 2, 3]));
    let (tx, mut rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 0);
    let c = logs
        .find_event(Level::WARN, "summon VFX dropped", "owner_mismatch")
        .expect("WARN arrival_vfx_dropped reason=owner_mismatch");
    assert!(c.has_field("owner_id", &OTHER.to_string()), "{c:?}");
    assert!(
        c.has_field("registered_owner_id", &OWNER.to_string()),
        "{c:?}"
    );
    assert!(mgr.pets.pending_arrival(pet).is_none());
    assert!(rx.try_recv().is_err(), "nothing sent");
}

/// Seam: the pet's entity is gone while the registry still lists it (the
/// sweep has not scrubbed it yet): DEBUG `reason=pet_gone`.
#[tokio::test]
async fn pet_gone_drop_logs_debug_with_reason() {
    let (mut mgr, pet) = world_with_pet();
    mgr.pets.queue_arrival(pet, arrival());
    // Remove the entity without the registry scrub `destroy_entity` does.
    let space_id = mgr.get_entity_space_id(pet).unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().entities.remove(&pet);
    let (tx, _rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    drain_arrivals(Instant::now(), &tx, &mut mgr).await;
    let c = logs
        .find_event(Level::DEBUG, "summon VFX dropped", "pet_gone")
        .expect("DEBUG arrival_vfx_dropped reason=pet_gone");
    assert!(c.has_field("pet_id", &pet.to_string()), "{c:?}");
    assert!(mgr.pets.pending_arrival(pet).is_none());
}
