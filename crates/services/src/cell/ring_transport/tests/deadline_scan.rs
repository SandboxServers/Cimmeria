//! Deadline-scan ordering and staleness (PR #662 review, finding 4).
//!
//! `run_tick_with_engine` snapshots `ready_regions(now)` once and then
//! applies the entries one at a time. Applying one entry can change another
//! region — the source's warmup drives its peer `RecvWarmup →
//! RemoteLoadWait` and re-arms the peer's stall with a fresh 90s bound — so
//! the snapshot is advisory, not authoritative. Two mechanisms keep it
//! honest, and both are pinned here:
//!
//! 1. **Ordering.** `ready_regions` puts bounded-stall aborts last, and the
//!    tick defers them until no real transition is left anywhere. An abort
//!    is a last resort; it must not pre-empt the transition that would make
//!    the ring healthy again.
//! 2. **Revalidation.** `run_one_deadline` re-reads the live deadline before
//!    applying it and skips an entry that no longer matches.

use std::time::Duration;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::ring_transport::transporter::RECV_WARMUP_TIMEOUT;
use crate::cell::ring_transport::{
    handle_select_destination, run_tick_with_engine, State, BSF_MOVEMENT_LOCK,
};

/// A tick that lags past 15 seconds holds BOTH the source's 4.0s warmup and
/// the peer's 15s `RecvWarmup` stall. The warmup is what ends the peer's
/// `RecvWarmup` — so running the peer's abort first tears down a trip that
/// was one transition from completing, leaving the passenger dumped back
/// where they started with the ring pair reset.
///
/// Before the fix the snapshot was applied in `HashMap` order with no
/// deferral and no revalidation, so the peer's stall could win: reverting
/// either the stall-last ordering in `ready_regions` or the non-stall-first
/// batching in `run_tick_with_engine` drives ring 2 to `Idle` (or trips
/// `hide_timer_expired`'s state assertion on the torn-down source), and this
/// test fails.
#[tokio::test]
async fn a_lagging_tick_completes_the_warmup_instead_of_firing_the_peers_stall() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Player already on pad 1, so selecting kicks warmup off immediately:
    // ring 1 arms hide (+3.5s) and warmup (+4.0s), ring 2 enters RecvWarmup
    // and arms its 15s stall.
    mgr.ring_transporters
        .get_mut(1)
        .unwrap()
        .region_triggered(true, 42);
    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::SendWarmup);
    assert_eq!(state_of(&mgr, 2), State::RecvWarmup);

    // One tick, arriving after everything: hide, warmup AND the peer's stall
    // are all elapsed in the same scan.
    clock.advance(RECV_WARMUP_TIMEOUT + Duration::from_secs(1));
    let ready = mgr
        .ring_transporters
        .ready_regions(mgr.ring_transporters.now());
    assert_eq!(
        ready.len(),
        2,
        "precondition: this test is only meaningful when both the source's \
         deadline and the peer's stall are in the same snapshot ({ready:?})"
    );
    assert!(
        !ready[0].1.is_stall() && ready[1].1.is_stall(),
        "ready_regions must order bounded-stall aborts last ({ready:?})"
    );

    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 1),
        State::Idle,
        "the source hands the trip off and resets — it does not abort"
    );
    assert_eq!(
        state_of(&mgr, 2),
        State::RemoteWarmup,
        "the peer must complete the hand-off (RecvWarmup → RemoteLoadWait → \
         RemoteWarmup), not be aborted by a stall deadline the warmup had \
         already superseded"
    );
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().players_loaded,
        vec![42],
        "the passenger must be counted at the destination"
    );
    assert_ne!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "mid-trip the passenger is still locked; an abort is what would have \
         released them here"
    );

    let pos = mgr.get_entity(42).unwrap().position;
    assert!(
        (pos.x - 10.0).abs() < 0.001
            && (pos.y - 20.0).abs() < 0.001
            && (pos.z - 30.0).abs() < 0.001,
        "the passenger must have been teleported to pad 2, got {pos:?}"
    );
}

/// The revalidation contract itself, at the seam the tick reads it through.
///
/// A snapshot entry taken before a peer advances must not still describe
/// that peer afterwards — otherwise `run_one_deadline` would apply a
/// `Stall` abort to a ring that is now healthily in `RemoteLoadWait` with a
/// fresh 90s bound.
#[tokio::test]
async fn a_snapshot_entry_goes_stale_when_the_peer_advances_within_the_tick() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    let now_then = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(1, now_then);
        dst.remote_send(now_then);
    }
    clock.advance(RECV_WARMUP_TIMEOUT + Duration::from_secs(1));
    let now = mgr.ring_transporters.now();

    let snapshot = mgr.ring_transporters.ready_regions(now);
    let (_, stale) = *snapshot
        .iter()
        .find(|(id, _)| *id == 2)
        .expect("ring 2's RecvWarmup stall must be elapsed");
    assert!(stale.is_stall());

    // Exactly what the source's warmup does to its peer.
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_expect(vec![42]);
        dst.remote_transport(now);
    }
    assert_eq!(state_of(&mgr, 2), State::RemoteLoadWait);

    assert_eq!(
        mgr.ring_transporters.current_deadline(2, now),
        None,
        "the re-armed 90s RemoteLoadWait bound is not elapsed, so the snapshot's \
         Stall entry no longer describes this ring and must be skipped"
    );
}
