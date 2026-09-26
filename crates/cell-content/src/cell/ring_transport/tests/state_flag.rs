//! H02 — the ring's movement-lock writes must respect the ref-count.
//!
//! `BSF_MOVEMENT_LOCK` is a ref-counted flag
//! (`crates/entity/src/cell_entity/state_flags.rs`) with death and the `Stun`
//! effect script as other writers. The ring used to write it with a raw
//! `state_field |= flag` / `&= !flag`, which never touches the counter — so a
//! ring release would clear a bit that death still owned, and conversely a
//! later counted `unset_state_flag` after a raw set would see count == 0,
//! take the no-op branch, and leave the bit stuck forever.
//!
//! The abort path H02 added fires precisely in the states where something
//! else has already gone wrong, which is what turns this from a latent
//! hazard into a reachable one.

use tokio::sync::mpsc;

use super::support::{spawn_player, three_ring_mgr, FakeClock};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::wire_helpers::update_state_flag;
use crate::cell::ring_transport::BSF_MOVEMENT_LOCK;
use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;

fn count_state_field_updates(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity_id: u32) -> usize {
    let mut n = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: e,
            method_index,
            ..
        } = msg
        {
            if e == entity_id && method_index == ON_STATE_FIELD_UPDATE {
                n += 1;
            }
        }
    }
    n
}

/// A player who dies during `SendWarmup` holds `BSF_MOVEMENT_LOCK` from the
/// death path's **counted** set. The ring's lock bumps the count to 2; the
/// ring's release drops it to 1 and must leave the bit **set** — the corpse
/// stays locked.
///
/// Under the old raw `state_field &= !flag` the ring's release clears the bit
/// outright and frees the corpse, and the assertions on both the bit and the
/// wire-message count fail.
#[tokio::test]
async fn a_ring_release_does_not_free_a_corpse_that_death_still_locks() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock);
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut rx) = mpsc::channel(64);

    // The death path's counted set (as `abilities::damage_apply` does it).
    let transitioned = mgr
        .get_entity_mut(42)
        .unwrap()
        .set_state_flag(BSF_MOVEMENT_LOCK);
    assert!(transitioned, "first set must be the 0 -> 1 transition");
    while rx.try_recv().is_ok() {}

    // The ring locks for the trip: count 1 -> 2, bit already set, so no
    // redundant broadcast.
    update_state_flag(42, BSF_MOVEMENT_LOCK, true, &tx, &mut mgr).await;
    assert_eq!(
        count_state_field_updates(&mut rx, 42),
        0,
        "a second holder must not re-broadcast a bit the client already has set"
    );

    // The abort releases the ring's hold: count 2 -> 1, bit STAYS set.
    update_state_flag(42, BSF_MOVEMENT_LOCK, false, &tx, &mut mgr).await;
    assert_ne!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the ring released its own hold, but death still owns the lock — \
         clearing the bit here frees the corpse"
    );
    assert_eq!(
        count_state_field_updates(&mut rx, 42),
        0,
        "no bit transition happened, so nothing should reach the client"
    );

    // Death's own release drains the counter: 1 -> 0, bit clears, one message.
    let cleared = mgr
        .get_entity_mut(42)
        .unwrap()
        .unset_state_flag(BSF_MOVEMENT_LOCK);
    assert!(cleared);
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0
    );
}

/// The ordinary case still works end to end and still reaches the wire: a
/// ring lock with no other holder is a real 0 -> 1 transition, and its
/// release is a real 1 -> 0.
#[tokio::test]
async fn a_solitary_ring_lock_and_release_each_reach_the_client_once() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock);
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut rx) = mpsc::channel(64);
    while rx.try_recv().is_ok() {}

    update_state_flag(42, BSF_MOVEMENT_LOCK, true, &tx, &mut mgr).await;
    assert_ne!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0
    );
    assert_eq!(count_state_field_updates(&mut rx, 42), 1);

    update_state_flag(42, BSF_MOVEMENT_LOCK, false, &tx, &mut mgr).await;
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0
    );
    assert_eq!(count_state_field_updates(&mut rx, 42), 1);
}
