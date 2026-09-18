//! H02 -- the Harset plaza's five-ring all-to-all mesh (regions 4-8, point
//! sets 2052-2056) keyed on `db/resources/Worlds/Seed/ring_transport_regions.sql`.
//!
//! The mesh is where a single unbounded stall is most expensive: every pad
//! lists the other four as destinations, so one wedged ring is removed from
//! four peers at once.

use std::time::Duration;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{harset_mgr, spawn_player, state_of, FakeClock};
use crate::cell::ring_transport::transporter::RECV_WAIT_TIMEOUT;
use crate::cell::ring_transport::{handle_select_destination, run_tick_with_engine, State};

// ---------------------------------------------------------------------------

/// R-01/R-02 mesh guard: no single stalled trip may permanently remove a
/// destination from the other four pads.
///
/// Region 8 is the stall victim on purpose — it is the one Harset pad whose
/// tag carries the odd `HarsetinRing…` spelling, so a future rename that
/// only fixes four of the five rows shows up here.
///
/// Reverting the stall deadline leaves ring 8 in `RecvWait` forever: the
/// post-timeout loop over rings 4-7 fails on the first iteration.
#[tokio::test]
async fn harset_mesh_survives_a_stall_on_every_ring_in_turn() {
    let clock = FakeClock::new();
    let mut mgr = harset_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    spawn_player(&mut mgr, 43, 701);
    let (tx, mut _rx) = mpsc::channel(256);
    let engine = ChainEngine::new();

    // Stall 4 → 8 and confirm it starves 8 for every other pad while live.
    handle_select_destination(4, 8, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 4), State::SendWait);
    assert_eq!(state_of(&mgr, 8), State::RecvWait);
    for src in [5, 6, 7] {
        handle_select_destination(src, 8, 43, &tx, &mut mgr, &engine).await;
        assert_eq!(
            state_of(&mgr, src),
            State::Idle,
            "ring {src} must roll back while 8 is legitimately busy"
        );
    }

    // Let the reservation expire. RECV_WAIT_TIMEOUT is the outer bound of
    // the pair, so one advance past it settles both ends.
    clock.advance(RECV_WAIT_TIMEOUT + Duration::from_secs(1));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    for id in [4, 5, 6, 7, 8] {
        assert_eq!(
            state_of(&mgr, id),
            State::Idle,
            "ring {id} must be Idle after the stalled trip is bounded"
        );
    }

    // Every remaining pad can still reach 8, and 8 can still reach each of
    // them. Each round-trip is torn down by its own timeout before the next.
    for src in [5, 6, 7, 4] {
        handle_select_destination(src, 8, 43, &tx, &mut mgr, &engine).await;
        assert_eq!(
            state_of(&mgr, src),
            State::SendWait,
            "ring {src} must be able to select 8 after the stall was cleared"
        );
        assert_eq!(state_of(&mgr, 8), State::RecvWait);

        clock.advance(RECV_WAIT_TIMEOUT + Duration::from_secs(1));
        run_tick_with_engine(&tx, &mut mgr, &engine).await;
        assert_eq!(state_of(&mgr, src), State::Idle);
        assert_eq!(state_of(&mgr, 8), State::Idle);
    }

    for dst in [4, 5, 6, 7] {
        handle_select_destination(8, dst, 43, &tx, &mut mgr, &engine).await;
        assert_eq!(
            state_of(&mgr, 8),
            State::SendWait,
            "ring 8 must still be able to send to {dst}"
        );
        clock.advance(RECV_WAIT_TIMEOUT + Duration::from_secs(1));
        run_tick_with_engine(&tx, &mut mgr, &engine).await;
        assert_eq!(state_of(&mgr, 8), State::Idle);
        assert_eq!(state_of(&mgr, dst), State::Idle);
    }
}
