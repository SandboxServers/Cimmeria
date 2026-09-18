//! H02 -- releasing the rings a departing player was holding.
//!
//! Two shapes: a real client disconnect (`SpaceManager::disconnect_entity`,
//! async, releases synchronously) and every other destroy path
//! (`destroy_entity`, synchronous, reconciled on the next ring tick). A
//! cross-world ring hand-off also destroys the cell entity and must NOT be
//! mistaken for either.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::ring_transport::{
    forget_player, handle_select_destination, run_tick_with_engine, State,
};

// ---------------------------------------------------------------------------

/// A disconnect during `SendWait` must free the pad immediately, not after
/// the 60s bound.
///
/// Reverting the `forget_player` hook in `SpaceManager::disconnect_entity`
/// leaves ring 1 in `SendWait` with `reserved_by == [42]` and ring 2 in
/// `RecvWait`, so the follow-up `handle_select_destination` is rejected.
#[tokio::test]
async fn disconnect_mid_send_wait_leaves_no_pending_entry_and_frees_the_peer() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    spawn_player(&mut mgr, 43, 701);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::SendWait);
    assert_eq!(state_of(&mgr, 2), State::RecvWait);

    // The client drops. No clock advance anywhere in this test — the point
    // is that recovery does NOT wait for a deadline.
    mgr.disconnect_entity(42, &tx).await;

    let src = mgr.ring_transporters.get(1).unwrap();
    assert!(
        src.reserved_by.is_empty(),
        "the disconnected player must not stay in the pad's pending set"
    );
    assert!(src.players.is_empty());
    assert!(src.send_players.is_empty());
    assert_eq!(src.state, State::Idle);
    assert_eq!(
        state_of(&mgr, 2),
        State::Idle,
        "the other end must not be stuck holding a reservation for a player who left"
    );

    // Selectable again, with no tick and no elapsed time.
    handle_select_destination(3, 2, 43, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 3), State::SendWait);
    assert_eq!(state_of(&mgr, 2), State::RecvWait);
}

/// The other destroy paths (GM despawn, gate travel, respawn, content
/// transport) go through the synchronous `destroy_entity`, which cannot
/// dispatch effects and so defers the reconciliation to the next ring tick.
/// The ring must still end up `Idle`.
#[tokio::test]
async fn destroy_mid_send_wait_is_reconciled_on_the_next_tick() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    mgr.destroy_entity(42);

    // Deferred by design: state and wire effects move together, in the tick.
    assert_eq!(state_of(&mgr, 1), State::SendWait);

    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::Idle);
    assert_eq!(state_of(&mgr, 2), State::Idle);
}

/// A cross-world ring hand-off destroys the traveller's cell entity as a
/// normal step (`Effect::TeleportCrossWorld` → `destroy_entity`) while the
/// destination is correctly parked in `RemoteLoadWait` expecting them.
///
/// The `destroy_entity` reconciliation is source-side only precisely so it
/// cannot mistake that for a drop, empty the expectation and fast-path the
/// destination to `Idle` before the traveller arrives. If someone widens
/// `forget_source_side` to touch `expected_players`, this fails.
#[tokio::test]
async fn cross_world_handoff_destroy_does_not_strand_the_traveller() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(1, now);
        dst.remote_send(now);
        dst.remote_expect(vec![42]);
        dst.remote_transport(now);
    }
    // This is the hand-off destroy, not a disconnect.
    mgr.destroy_entity(42);
    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 2),
        State::RemoteLoadWait,
        "the destination must keep holding the slot for a traveller who is mid-hand-off"
    );
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().expected_players,
        vec![42]
    );
}

/// A genuine disconnect of one of two co-travellers must shrink the
/// expectation rather than wedging the destination. Removing the passenger
/// from `expected_players` alone (leaving them in `players_loaded`) would
/// make the readiness equality unsatisfiable — a fresh stall in place of the
/// old one.
#[tokio::test]
async fn disconnect_of_one_co_traveller_lets_the_other_complete() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    spawn_player(&mut mgr, 43, 701);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(1, now);
        dst.remote_send(now);
        dst.remote_expect(vec![42, 43]);
        dst.remote_transport(now);
        // 43 arrived; 42 is still loading.
        dst.player_loaded(43);
    }
    assert_eq!(state_of(&mgr, 2), State::RemoteLoadWait);

    forget_player(42, &tx, &mut mgr).await;
    {
        let dst = mgr.ring_transporters.get(2).unwrap();
        assert_eq!(dst.expected_players, vec![43]);
        assert_eq!(dst.players_loaded, vec![43]);
    }

    // The queued re-check runs on the tick and completes the trip for 43
    // without waiting out REMOTE_LOAD_WAIT_TIMEOUT.
    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_ne!(
        state_of(&mgr, 2),
        State::RemoteLoadWait,
        "the surviving traveller's trip must advance once the expectation shrinks"
    );
}
