//! H02 -- the four bounded stall deadlines (audit defect H-B3).
//!
//! `SendWait`, `RecvWait`, `RecvWarmup` and `RemoteLoadWait` wait on things
//! outside the FSM and used to wait forever. The regression shape each guard
//! reproduces is *destination starvation*: `handle_select_destination`
//! refuses any destination that is not `Idle`, so a ring parked in a
//! non-terminating state is removed from every peer that can reach it. The
//! assertion that matters is therefore always "a later
//! `handle_select_destination` to that ring succeeds", which is exactly what
//! fails when the timeout is reverted.

use std::time::Duration;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::support::{harset_mgr, spawn_player, state_of, three_ring_mgr, FakeClock};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::transporter::{REMOTE_LOAD_WAIT_TIMEOUT, SEND_WAIT_TIMEOUT};
use crate::cell::ring_transport::{
    handle_select_destination, run_tick_with_engine, State, BSF_MOVEMENT_LOCK,
};
use crate::mercury::method_idx::{ON_STATE_FIELD_UPDATE, ON_VISIBLE};

// ---------------------------------------------------------------------------

/// The `SendWait` stall: a player picks a destination from the console and
/// then never walks onto the pad (walks away, alt-tabs, dies).
///
/// Before H02 both rings sat in `SendWait` / `RecvWait` for the life of the
/// process. Reverting the stall deadline makes the final
/// `handle_select_destination` fail: ring 2 is still `RecvWait`, the
/// non-Idle-destination rejection fires, and ring 3 never reaches
/// `SendWait`.
#[tokio::test]
async fn send_wait_stall_returns_both_rings_to_idle_and_frees_the_destination() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    spawn_player(&mut mgr, 43, 701);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // 42 reserves 1 → 2 and never steps onto pad 1.
    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::SendWait);
    assert_eq!(state_of(&mgr, 2), State::RecvWait);
    assert_eq!(mgr.ring_transporters.get(1).unwrap().reserved_by, vec![42]);

    // While the trip is live, ring 2 is legitimately unavailable to anyone
    // else. This is the starvation mechanism, asserted so the test documents
    // what the timeout is protecting against.
    handle_select_destination(3, 2, 43, &tx, &mut mgr, &engine).await;
    assert_eq!(
        state_of(&mgr, 3),
        State::Idle,
        "a busy destination must roll the would-be source back to Idle"
    );

    // Not yet expired: a tick just short of the bound changes nothing.
    clock.advance(SEND_WAIT_TIMEOUT - Duration::from_secs(1));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::SendWait);
    assert_eq!(state_of(&mgr, 2), State::RecvWait);

    // Expired: both ends released.
    clock.advance(Duration::from_secs(2));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::Idle);
    assert_eq!(
        state_of(&mgr, 2),
        State::Idle,
        "the peer must be dragged back to Idle, not left holding a reservation"
    );
    assert!(mgr.ring_transporters.get(1).unwrap().reserved_by.is_empty());

    // The point of the whole packet: ring 2 is selectable again.
    handle_select_destination(3, 2, 43, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 3), State::SendWait);
    assert_eq!(state_of(&mgr, 2), State::RecvWait);
}

/// The `RemoteLoadWait` stall: the cross-world arrival hand-off never comes
/// back, so the destination holds a passenger that will never report loaded.
///
/// Also the wire guard for the release: the surviving passenger must get
/// `onVisible(1)` and a movement-lock clear, in that order. Reverting the
/// stall deadline leaves ring 2 in `RemoteLoadWait`, emits neither message,
/// and keeps the player invisible and frozen.
#[tokio::test]
async fn remote_load_wait_stall_releases_the_hidden_passenger_on_the_wire() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Park ring 2 exactly where a cross-world hand-off leaves it: expecting
    // 42, who is locked and hidden and whose `AdvanceRingDestination` never
    // arrives.
    let now = mgr.ring_transporters.now();
    {
        let dst = mgr.ring_transporters.get_mut(2).unwrap();
        dst.remote_wait(1, now);
        dst.remote_send(now);
        dst.remote_expect(vec![42]);
        dst.remote_transport(now);
    }
    mgr.get_entity_mut(42)
        .unwrap()
        .set_state_flag(BSF_MOVEMENT_LOCK);
    while rx.try_recv().is_ok() {}

    clock.advance(REMOTE_LOAD_WAIT_TIMEOUT + Duration::from_secs(1));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    assert_eq!(state_of(&mgr, 2), State::Idle);
    assert!(mgr
        .ring_transporters
        .get(2)
        .unwrap()
        .expected_players
        .is_empty());
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().num_remote_players(),
        0
    );
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the abort must clear the movement lock, not just the FSM state"
    );

    // Wire order: onVisible(1) strictly before the state-field clear.
    let mut saw_visible_at = None;
    let mut saw_unlock_at = None;
    let mut idx = 0usize;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::WitnessEntityMethod {
                entity_id: 42,
                method_index,
                ref args,
                ..
            } if method_index == ON_VISIBLE && args == &[1] => {
                saw_visible_at.get_or_insert(idx);
            }
            CellToBaseMsg::EntityMethodCall {
                entity_id: 42,
                method_index,
                ..
            } if method_index == ON_STATE_FIELD_UPDATE => {
                saw_unlock_at.get_or_insert(idx);
            }
            _ => {}
        }
        idx += 1;
    }
    let visible = saw_visible_at.expect("abort must re-show the stranded passenger");
    let unlock = saw_unlock_at.expect("abort must clear the passenger's movement lock");
    assert!(
        visible < unlock,
        "show must precede unlock: unlocking first lets the player move while \
         witnesses still hold them hidden, and send_visible resolves its witness \
         set at call time (show={visible}, unlock={unlock})"
    );
}

/// `run_one_deadline`'s warmup branch used to `return` on an unresolvable
/// destination region WITHOUT clearing `warmup_at`, so it re-fired every
/// tick forever while the source sat in `SendWarmup` with its passenger
/// locked and hidden. Same unbounded-state defect class as H-B3; one state
/// the audit did not list.
#[tokio::test]
async fn warmup_with_a_missing_destination_region_aborts_instead_of_spinning() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700);
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Make `ring_regions` and `ring_transporters` disagree, which is the
    // only way this branch is reachable.
    mgr.ring_regions.remove(&2);

    // Player is on the pad, so selecting kicks warmup off immediately.
    mgr.ring_transporters
        .get_mut(1)
        .unwrap()
        .region_triggered(true, 42);
    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::SendWarmup);

    clock.advance(Duration::from_secs(5));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    assert_eq!(
        state_of(&mgr, 1),
        State::Idle,
        "an unresolvable destination must abort the trip, not re-fire the warmup \
         deadline every tick with the passenger locked and hidden"
    );
    assert_eq!(
        mgr.get_entity(42).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0
    );

    // And it stays quiet: a second tick produces no further transition.
    run_tick_with_engine(&tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 1), State::Idle);
}

/// The Harset-specific shape of the `SendWait` stall, raised by the H10
/// worker: the five plaza ring switches are unconditional public
/// interactables (`required_mission_id` is NULL on all five seed rows), so
/// any passer-by can activate one, pick a destination and walk off. Before
/// H02 that wedged the pad for the whole zone until a server restart, with
/// no cost or intent required on the player's part.
///
/// Distinct from the mesh test below in what it pins: this one is about the
/// *unattended* case specifically — no disconnect, no peer failure, the
/// player simply never returns — and it asserts the pad is usable by a
/// **different** player afterwards, which is the zone-level symptom.
#[tokio::test]
async fn unattended_send_leaves_the_ring_selectable_after_the_timeout() {
    let clock = FakeClock::new();
    let mut mgr = harset_mgr(clock.clone());
    spawn_player(&mut mgr, 42, 700); // the passer-by
    spawn_player(&mut mgr, 43, 701); // everyone else in the zone
    let (tx, mut _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Ring 4 (HarsetRingLeftBottom): activate, pick 8, wander off.
    handle_select_destination(4, 8, 42, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 4), State::SendWait);
    assert!(
        mgr.ring_transporters
            .get(4)
            .unwrap()
            .required_mission_id
            .is_none(),
        "the Harset plaza rings are ungated, which is what makes an unattended \
         send a zone-wide denial rather than a self-inflicted one"
    );

    // Nobody else can use pad 4 while it is reserved.
    handle_select_destination(4, 7, 43, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 4), State::SendWait);
    assert_eq!(
        mgr.ring_transporters.get(4).unwrap().remote_region_id,
        Some(8),
        "the abandoned reservation must still name its original destination"
    );

    clock.advance(SEND_WAIT_TIMEOUT + Duration::from_secs(1));
    run_tick_with_engine(&tx, &mut mgr, &engine).await;

    assert_eq!(state_of(&mgr, 4), State::Idle);
    assert_eq!(state_of(&mgr, 8), State::Idle);

    // A different player can now use the same pad, for a different
    // destination.
    handle_select_destination(4, 7, 43, &tx, &mut mgr, &engine).await;
    assert_eq!(state_of(&mgr, 4), State::SendWait);
    assert_eq!(
        mgr.ring_transporters.get(4).unwrap().remote_region_id,
        Some(7)
    );
    assert_eq!(mgr.ring_transporters.get(4).unwrap().reserved_by, vec![43]);
    assert_eq!(state_of(&mgr, 7), State::RecvWait);
}
