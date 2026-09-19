//! Destination-side readiness accounting (PR #662 review, finding 5).
//!
//! `try_advance_after_load` fires when `players_loaded.len() ==
//! num_remote_players()` — a *length* comparison. Membership is the only
//! thing that makes the two sides describe the same people, so a load
//! notification from outside the FSM has to be checked against
//! `expected_players` before it is counted. The cross-world
//! `AdvanceRingDestination` hook is exactly such a notification, and it can
//! arrive arbitrarily late.

use std::time::Duration;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::ring_transport::transporter::REMOTE_LOAD_WAIT_TIMEOUT;
use crate::cell::ring_transport::{
    handle_remote_player_loaded, run_tick_with_engine, State, BSF_MOVEMENT_LOCK,
};

/// Trip 1 sends 42 to ring 2. 42's client is still loading when
/// `REMOTE_LOAD_WAIT_TIMEOUT` releases the ring. Trip 2 then claims ring 2
/// for 43. Only *now* does the base report 42 loaded.
///
/// 42 must be released as a late arrival and must not touch trip 2's books.
/// Before the fix, `player_loaded(42)` pushed 42 into `players_loaded`,
/// `len() == 1 == num_remote_players()` (the expectation being `[43]`), and
/// `all_players_loaded` fired — advancing ring 2 to `RemoteWarmup` for a trip
/// whose real passenger, 43, was still loading and would then be carried
/// through `Cooldown → Idle` without ever being counted. Reverting either the
/// `expects_player` guard in `player_loaded` or the membership half of
/// `handle_remote_player_loaded`'s gate fails the first two assertions here.
#[tokio::test]
async fn a_stale_late_arrival_cannot_satisfy_the_next_trips_readiness_gate() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    spawn_player(&mut mgr, 43, 701);
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // ── Trip 1: ring 1 → ring 2, carrying 42 (cross-world, so the load is
    //    deferred to the base's AdvanceRingDestination callback).
    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(1, now);
        dst.remote_send(now);
        dst.remote_expect(vec![42]);
        dst.remote_transport(now);
    }
    if let Some(p) = mgr.get_entity_mut(42) {
        p.set_state_flag(BSF_MOVEMENT_LOCK);
        p.destination_ring_id = Some(2);
    }

    // 42's client is still loading when the bound fires.
    clock.advance(REMOTE_LOAD_WAIT_TIMEOUT + Duration::from_secs(1));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 2), State::Idle);

    // ── Trip 2: ring 3 → ring 2, carrying 43. Ring 2 is back in
    //    RemoteLoadWait, for a completely different passenger list.
    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(3, now);
        dst.remote_send(now);
        dst.remote_expect(vec![43]);
        dst.remote_transport(now);
    }
    if let Some(p) = mgr.get_entity_mut(43) {
        p.set_state_flag(BSF_MOVEMENT_LOCK);
        p.destination_ring_id = Some(2);
    }
    // 42 is re-created on the destination world with its own hand-off lock.
    if let Some(p) = mgr.get_entity_mut(42) {
        p.set_state_flag(BSF_MOVEMENT_LOCK);
        p.destination_ring_id = Some(2);
    }
    while rx.try_recv().is_ok() {}

    // ── 42's load finally lands, on a ring mid-trip for somebody else.
    handle_remote_player_loaded(2, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 2),
        State::RemoteLoadWait,
        "trip 2 must still be waiting for 43 — a stranger's arrival must not \
         advance it"
    );
    assert!(
        mgr.ring_transporters
            .get(2)
            .unwrap()
            .players_loaded
            .is_empty(),
        "42 is not on trip 2's passenger list and must not be recorded against it"
    );
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().expected_players,
        vec![43],
        "the real expectation must be untouched"
    );

    // 42 takes the late-arrival release: visible, unlocked, no longer routed.
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the late arrival must still be released — refusing to count them is \
         not the same as dropping them"
    );
    assert_eq!(mgr.get_entity(42).unwrap().destination_ring_id, None);

    // 43 is mid-trip and must be left exactly as it was.
    assert_ne!(
        mgr.get_entity(43).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "trip 2's real passenger is still in flight and must stay locked"
    );

    // And trip 2 completes normally once 43 actually lands.
    handle_remote_player_loaded(2, 43, &tx, &mut mgr, &engine).await;
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().players_loaded,
        vec![43]
    );
    assert_ne!(
        state_of(&mgr, 2),
        State::RemoteLoadWait,
        "the expected passenger must still advance the trip"
    );
}
