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
    PetArrival::new(OWNER, PET_FIXTURE_TEMPLATE_ID, 2293, vec![1, 2, 3])
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
/// owner's identity as captured at summon (the intro went missing; no
/// client can cause it).
#[tokio::test]
async fn never_witnessed_drop_logs_warn_with_reason() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(77);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
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
    assert!(
        c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()),
        "{c:?}"
    );
    assert!(c.has_field("account_id", &OWNER.to_string()), "{c:?}");
    assert!(c.has_field("player_id", "77"), "{c:?}");
}

/// Seam: a queued VFX whose owner no longer matches the registry (an id
/// reused past a missed scrub) is dropped with a WARN
/// `reason=owner_mismatch`, and never sent.
#[tokio::test]
async fn owner_mismatch_drop_logs_warn_with_reason() {
    let (mut mgr, pet) = world_with_pet();
    mgr.pets.queue_arrival(
        pet,
        PetArrival::new(OTHER, PET_FIXTURE_TEMPLATE_ID, 2293, vec![1, 2, 3]),
    );
    let (tx, mut rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 0);
    let c = logs
        .find_event(Level::WARN, "summon VFX dropped", "owner_mismatch")
        .expect("WARN arrival_vfx_dropped reason=owner_mismatch");
    assert!(c.has_field("owner_id", &OTHER.to_string()), "{c:?}");
    assert!(
        c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()),
        "{c:?}"
    );
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
    assert!(
        c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()),
        "{c:?}"
    );
    assert!(mgr.pets.pending_arrival(pet).is_none());
}

/// The owner was destroyed and its entity id handed to another player
/// before the sweep: the new holder is not the summoner, so the VFX is
/// dropped (WARN `reason=owner_identity_mismatch`, the summoner's identity
/// on the row) and never sent, even though the registry's owner id still
/// matches (Copilot, #870).
#[tokio::test]
async fn reused_owner_id_drops_the_vfx_with_identity_mismatch() {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(77);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
    mgr.pets.queue_arrival(pet, arrival());
    reuse_owner_id_by_another_player(&mut mgr);
    assert_eq!(mgr.pets.owner_of(pet), Some(OWNER), "not swept yet");
    let (tx, mut rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 0);
    let c = logs
        .find_event(Level::WARN, "summon VFX dropped", "owner_identity_mismatch")
        .expect("WARN arrival_vfx_dropped reason=owner_identity_mismatch");
    assert!(
        c.has_field("player_id", "77"),
        "the summoner, not 4243: {c:?}"
    );
    assert!(
        c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()),
        "{c:?}"
    );
    assert!(mgr.pets.pending_arrival(pet).is_none());
    assert!(rx.try_recv().is_err(), "nothing sent");
}

/// The one `pets.lifecycle` row with this `event`, at this level.
fn lifecycle_event(
    all: &[crate::test_support::Captured],
    level: Level,
    event: &str,
) -> crate::test_support::Captured {
    all.iter()
        .find(|c| c.target == "pets.lifecycle" && c.level == level && c.has_field("event", event))
        .cloned()
        .unwrap_or_else(|| panic!("no {level} pets.lifecycle event={event}: {all:#?}"))
}

/// `world_with_pet`, with the owner's `player_id` known at summon, a VFX
/// queued, and the AoI tick run so the owner witnesses the pet.
fn witnessed_pet_with_vfx() -> (SpaceManager, u32) {
    let mut mgr = make_world();
    add_pet_owner(&mut mgr, OWNER, "Agnos", [10.0, 0.0, 10.0], 12);
    mgr.get_entity_mut(OWNER).unwrap().player_id = Some(77);
    let pet = mgr
        .spawn_pet_from_template(OWNER, PET_FIXTURE_TEMPLATE_ID, 1643)
        .expect("pet spawns");
    mgr.pets.queue_arrival(pet, arrival());
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(pet).contains(&OWNER),
        "fixture: the owner witnesses the pet"
    );
    (mgr, pet)
}

/// A delivered VFX is counted and logs DEBUG `arrival_vfx_sent` with the
/// Rule 5 correlators (`entity_id` = the pet, the summoner's ids).
#[tokio::test]
async fn delivered_vfx_is_counted_and_logs_sent_with_correlators() {
    let (mut mgr, pet) = witnessed_pet_with_vfx();
    let (tx, mut rx) = mpsc::channel(64);

    let logs = LogCapture::install();
    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 1);
    assert!(rx.try_recv().is_ok(), "the owner got the VFX");
    let c = lifecycle_event(&logs.all(), Level::DEBUG, "arrival_vfx_sent");
    assert!(c.has_field("entity_id", &pet.to_string()), "{c:?}");
    assert!(c.has_field("account_id", &OWNER.to_string()), "{c:?}");
    assert!(c.has_field("player_id", "77"), "{c:?}");
    assert!(
        c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()),
        "{c:?}"
    );
    assert!(c.has_field("delivered_count", "1"), "{c:?}");
}

/// Seam: every witness send fails (the base channel is closed). The VFX is
/// not counted, `arrival_vfx_sent` is not logged, each failed send logs
/// WARN `arrival_vfx_send_failed`, and the whole attempt logs WARN
/// `arrival_vfx_undelivered`, all with the Rule 5 correlators.
#[tokio::test]
async fn vfx_with_every_send_failing_is_undelivered_not_sent() {
    let (mut mgr, pet) = witnessed_pet_with_vfx();
    let (tx, rx) = mpsc::channel(64);
    drop(rx);

    let logs = LogCapture::install();
    assert_eq!(drain_arrivals(Instant::now(), &tx, &mut mgr).await, 0);
    assert!(mgr.pets.pending_arrival(pet).is_none(), "not retried");
    let all = logs.all();
    assert!(
        !all.iter().any(|c| c.has_field("event", "arrival_vfx_sent")),
        "nothing reached a client, so nothing is logged as sent: {all:#?}"
    );
    for event in ["arrival_vfx_send_failed", "arrival_vfx_undelivered"] {
        let c = lifecycle_event(&all, Level::WARN, event);
        assert!(c.has_field("entity_id", &pet.to_string()), "{c:?}");
        assert!(c.has_field("account_id", &OWNER.to_string()), "{c:?}");
        assert!(c.has_field("player_id", "77"), "{c:?}");
        assert!(
            c.has_field("template_id", &PET_FIXTURE_TEMPLATE_ID.to_string()),
            "{c:?}"
        );
    }
}
