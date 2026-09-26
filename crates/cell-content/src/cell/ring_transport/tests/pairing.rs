//! H02 -- the source/destination cross-link guard.
//!
//! A ring pair is linked by `remote_region_id` on both ends. Advancing a
//! destination on its *state* alone is not enough: a pad legitimately
//! reserved by another source is also in `RecvWait`.
//!
//! PR #662 review, finding 6 extends that: *not advancing* the destination is
//! only half an answer. The source is already in `SendWarmup` by then, and
//! nothing downstream rescues it — `run_one_deadline`'s warmup arm only
//! aborts when the static `ring_regions` table has no row for the
//! destination, which it always does. So the source has to abort here, or its
//! passengers get teleported onto a pad another trip is holding.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::{handle_region_trigger, State, BSF_MOVEMENT_LOCK};
use crate::mercury::method_idx::ON_VISIBLE;

/// `kick_off_warmup` advances the destination only when its back-pointer
/// names this source. Without that check it advanced on `state == RecvWait`
/// alone, so a pad reserved by a *different* source (whose own `RecvWait`
/// bound has not yet expired) would be dragged into this trip and would then
/// receive passengers it was holding no slot for. The
/// `RECV_WAIT_TIMEOUT > SEND_WAIT_TIMEOUT` margin is the cheap guard; this is
/// the structural one.
///
/// Reverting the abort (the pre-review code only logged and fell through to
/// `dispatch_effects`) leaves ring 1 in `SendWarmup` with entity 42 locked
/// and hidden, and its warmup then teleports 42 onto ring 2's pad — the pad
/// ring 3 reserved.
#[tokio::test]
async fn a_destination_reserved_by_another_source_aborts_the_trip() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Ring 2 is in RecvWait but reserved for ring 3, not ring 1.
    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(3, now);
    }
    // Ring 1 believes it is sending to 2 and its passenger steps on the pad.
    {
        let src = mgr.ring_transporters.get_mut(1).unwrap();
        src.enter_send_wait(2, 42, now);
    }
    handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 1),
        State::Idle,
        "the source must abort: it has no prepared destination and nothing \
         downstream would release its passengers"
    );
    assert_eq!(
        state_of(&mgr, 2),
        State::RecvWait,
        "the destination belongs to ring 3's trip and must not be advanced — \
         nor torn down by the aborting source"
    );
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().remote_region_id,
        Some(3),
        "the destination's reservation must be left pointing at its real source"
    );
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the passenger must not be left movement-locked by a trip that never ran"
    );

    let mut saw_teleport = false;
    let mut saw_visible = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::TeleportPlayer { .. } => saw_teleport = true,
            CellToBaseMsg::WitnessEntityMethod {
                entity_id: 42,
                method_index,
                ref args,
                ..
            } if method_index == ON_VISIBLE && args == &[1] => saw_visible = true,
            _ => {}
        }
    }
    assert!(!saw_teleport, "no passenger may reach the reserved pad");
    assert!(saw_visible, "the abort must leave the passenger visible");
}

/// The other half of "the destination is not prepared": its own `RecvWait`
/// reservation lapsed in the gap between the player choosing a destination
/// and the player walking onto the source pad, so it is back in `Idle`.
///
/// Before the fix the source sailed on into `SendWarmup`. Its warmup then
/// teleported the passenger onto the destination pad and called
/// `advance_destination_after_warmup`, which only drives `RecvWarmup →
/// RemoteLoadWait` — so the `Idle` destination took the expectation but never
/// the state, `try_advance_after_load` could never fire, and no deadline was
/// armed on either end. The passenger sat hidden and movement-locked forever:
/// the unbounded-state shape H02 exists to prevent.
#[tokio::test]
async fn an_idle_destination_aborts_the_trip_instead_of_stranding_the_passenger() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    let now = mgr.ring_transporters.now();
    {
        // Ring 2 never reserved anything (or its reservation already lapsed).
        let src = mgr.ring_transporters.get_mut(1).unwrap();
        src.enter_send_wait(2, 42, now);
    }
    assert_eq!(state_of(&mgr, 2), State::Idle, "precondition");

    handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(state_of(&mgr, 1), State::Idle, "the source must abort");
    assert_eq!(state_of(&mgr, 2), State::Idle);
    assert!(
        mgr.ring_transporters
            .get(2)
            .unwrap()
            .expected_players
            .is_empty(),
        "an Idle destination must not be left holding an expectation it has no \
         state to act on"
    );
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the passenger must not be left movement-locked"
    );
}
