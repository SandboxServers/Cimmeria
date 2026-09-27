//! The summon VFX queue (PT-03): scrubbed with its pet, dropped when the
//! owner never sees the pet, never queued for a non-pet.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

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
