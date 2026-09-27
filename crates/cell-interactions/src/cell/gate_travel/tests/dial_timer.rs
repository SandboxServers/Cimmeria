//! The dial timer, the post-crossing cinematic hold, their cancellations,
//! and the crossing gate.
//!
//! `begin_gate_dial` arms the dial timer, `cancel_gate_dial` drops it,
//! `gate_dial_tick` opens the gate exactly once, and
//! `handle_stargate_region_entered` is a no-op unless a dial is armed AND
//! passable. A successful crossing arms a second, independent deadline —
//! `CROSSING_CINEMATIC_HOLD` — via `begin_crossing_hold`, drained by
//! `crossing_tick` (NA35).
//!
//! These drive `SpaceManager`'s deadlines directly rather than sleeping
//! real time — `take_opened_gate_dials` / `take_ready_crossings` take
//! `now`, so the tests move the clock instead of waiting on it. The tests
//! that must go through `gate_dial_tick` / `crossing_tick` (which read
//! `Instant::now()` themselves) rewind the relevant deadline into the past
//! first.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::super::sequences::{EVENT_STARGATE_CROSS_GATE, EVENT_STARGATE_MAKE_GATE};
use super::super::{
    crossing_tick, gate_dial_tick, handle_dial_gate, handle_stargate_region_entered,
};
use super::{
    engine, grant_all_addresses, make_manager_with_stargates, SEQ_CROSS_GATE, SEQ_MAKE_GATE,
};
use crate::cell::client_methods::gate_travel::ON_STARGATE_PASSAGE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::BSF_MOVEMENT_LOCK;
use crate::cell::space_manager::SpaceManager;

const DIALER: u32 = 1;

async fn armed_dialer() -> (
    SpaceManager,
    mpsc::Receiver<CellToBaseMsg>,
    mpsc::Sender<CellToBaseMsg>,
) {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(DIALER, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, DIALER);
    if let Some(e) = mgr.get_entity_mut(DIALER) {
        e.is_player = true;
        e.player_id = Some(42);
    }
    mgr.connect_entity(DIALER);

    let (tx, rx) = mpsc::channel(64);
    handle_dial_gate(DIALER, 2, 0, &tx, &mut mgr, &engine()).await;
    (mgr, rx, tx)
}

/// Rewind the armed dial's deadline so `gate_dial_tick`'s internal
/// `Instant::now()` sees it as elapsed.
fn expire_the_timer(mgr: &mut SpaceManager) {
    let dial = mgr
        .pending_gate_dials
        .get_mut(&DIALER)
        .expect("a dial must be armed");
    dial.open_at = Instant::now() - Duration::from_millis(1);
}

/// Rewind `entity_id`'s armed crossing hold so `crossing_tick`'s internal
/// `Instant::now()` sees it as elapsed. Mirrors `expire_the_timer`.
fn expire_the_crossing_hold(mgr: &mut SpaceManager, entity_id: u32) {
    let crossing = mgr
        .pending_crossings
        .get_mut(&entity_id)
        .expect("a crossing hold must be armed");
    crossing.travel_at = Instant::now() - Duration::from_millis(1);
}

/// End-to-end: arm → tick → `Stargate_MakeGate` on the wire, gate
/// passable. The frame must not appear before the timer expires.
#[tokio::test]
async fn the_tick_opens_the_gate_and_sends_make_gate_once() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;

    gate_dial_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "a tick before the 4s deadline must emit nothing"
    );
    assert!(!mgr.gate_dial(DIALER).unwrap().passable);

    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;

    let msg = rx.try_recv().expect("Stargate_MakeGate must be sent");
    match msg {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id, args, ..
        } => {
            assert_eq!(witness_id, DIALER);
            assert_eq!(
                i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
                SEQ_MAKE_GATE
            );
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "one frame, no witnesses in fixture");
    assert!(mgr.gate_dial(DIALER).unwrap().passable);

    // Second tick: `gateDialTimerExpired` clears its timer before
    // sending, so the gate opens exactly once.
    gate_dial_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "Stargate_MakeGate must fire once per dial, not every tick"
    );
}

/// `cancelDialing` (client sends address -1) drops the pending open.
#[tokio::test]
async fn cancel_dial_stops_the_pending_make_gate() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;

    handle_dial_gate(DIALER, -1, 0, &tx, &mut mgr, &engine()).await;
    assert!(mgr.gate_dial(DIALER).is_none());

    // Even well past the original deadline, nothing fires.
    gate_dial_tick(&tx, &mut mgr).await;
    assert!(mgr
        .take_opened_gate_dials(Instant::now() + Duration::from_secs(10))
        .is_empty());
    gate_dial_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "a cancelled dial must never emit Stargate_MakeGate"
    );
}

/// Leaving the space cancels the dial — the "dialer leaves" half of the
/// cancellation requirement. `destroy_entity` is the choke point every
/// departure (disconnect, teleport, death) routes through.
#[tokio::test]
async fn leaving_the_space_stops_the_pending_make_gate() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;

    // Expire the deadline FIRST. Without this the dial is still four
    // seconds out, so the tick emits nothing whether or not
    // `destroy_entity` scrubbed it — the test would pass with the scrub
    // reverted and prove nothing.
    expire_the_timer(&mut mgr);
    mgr.destroy_entity(DIALER);

    gate_dial_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "a dialer who left the space must not get a late gate-open, even \
         though their dial deadline has passed"
    );
    assert!(mgr.gate_dial(DIALER).is_none());
}

/// Re-dialling restarts the timer: `beginDialing` cancels the in-flight
/// dial first, so an already-open gate closes again and the new
/// destination is what the crossing will use.
#[tokio::test]
async fn redialling_restarts_the_timer_and_replaces_the_destination() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;
    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;
    rx.try_recv().expect("first gate opened");
    assert!(mgr.gate_dial(DIALER).unwrap().passable);

    // Re-dial the SAME destination — the address is irrelevant, the
    // point is that a second dial re-arms.
    handle_dial_gate(DIALER, 2, 0, &tx, &mut mgr, &engine()).await;

    let dial = mgr.gate_dial(DIALER).expect("the re-dial must be armed");
    assert!(
        !dial.passable,
        "a re-dial closes the gate again — crossing now must not travel"
    );

    // And the crossing is indeed refused while it is shut.
    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;
    assert!(rx.try_recv().is_err(), "no cross-gate, no GateTravel");
    assert!(mgr.get_entity(DIALER).is_some());
}

/// `stargatePassed` with nothing dialled is a no-op. Walking through the
/// gate volume on the way past must not teleport anyone.
#[tokio::test]
async fn crossing_without_a_dial_does_nothing() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(DIALER, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, DIALER);
    mgr.connect_entity(DIALER);
    let (tx, mut rx) = mpsc::channel(16);

    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;

    assert!(rx.try_recv().is_err());
    assert!(mgr.get_entity(DIALER).is_some());
}

/// `stargatePassed` while the gate is still dialling is a no-op too —
/// `gatePassable` gates it, not `dialedAddress` alone.
#[tokio::test]
async fn crossing_before_the_gate_opens_does_nothing() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;

    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;

    assert!(rx.try_recv().is_err(), "no sequence and no GateTravel");
    assert!(mgr.get_entity(DIALER).is_some());
    assert!(
        mgr.gate_dial(DIALER).is_some(),
        "an early crossing must not consume the dial"
    );
}

/// A traveller who lands in the gate volume while ANOTHER player has the
/// gate open is not crossed.
///
/// This is the arrival case: gate travellers arrive on the gate row, which
/// for Harset is inside the `Harset.Stargate` volume, so their first region
/// hint is an enter on the gate. If the dial were keyed on the gate or the
/// world rather than on the dialling entity, a traveller landing while
/// someone else's wormhole is open would be sent straight through it, to a
/// destination they never dialled. `SGWPlayer` kept `dialedAddress` and
/// `gatePassable` on the player, and so does `pending_gate_dials`.
#[tokio::test]
async fn a_traveller_arriving_while_another_player_holds_an_open_dial_is_not_crossed() {
    const ARRIVER: u32 = 2;

    let (mut mgr, mut rx, tx) = armed_dialer().await;
    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;
    rx.try_recv().expect("the dialer's gate opened");
    assert!(mgr.gate_dial(DIALER).unwrap().passable);

    // The traveller materialises on the gate row, inside the volume.
    mgr.create_entity(ARRIVER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(&mut mgr, ARRIVER);
    if let Some(e) = mgr.get_entity_mut(ARRIVER) {
        e.is_player = true;
        e.player_id = Some(43);
    }
    mgr.connect_entity(ARRIVER);

    handle_stargate_region_entered(ARRIVER, &tx, &mut mgr, &engine()).await;

    assert!(
        rx.try_recv().is_err(),
        "the arriving traveller was sent a sequence or a GateTravel through \
         another player's open gate"
    );
    assert!(mgr.get_entity(ARRIVER).is_some(), "the traveller stays put");
    assert!(
        mgr.gate_dial(ARRIVER).is_none(),
        "the traveller holds no dial"
    );
    let dial = mgr
        .gate_dial(DIALER)
        .expect("the dialer's open gate must survive someone else's hint");
    assert!(dial.passable);
}

/// The crossing: `Stargate_CrossGate` and `onStargatePassage` go out
/// immediately, the dial is consumed, and the traveller's movement is
/// locked — but the world transition (`GateTravel`) is DEFERRED behind the
/// crossing hold (NA35), not issued synchronously. This is the wire-order
/// and byte-exact half of the coordinator's four required test shapes;
/// `crossing_hold_elapsing_runs_the_deferred_travel` below covers the
/// hold-then-travel sequence.
#[tokio::test]
async fn crossing_an_open_gate_sends_cross_gate_then_onstargatepassage_and_defers_travel() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;
    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;
    rx.try_recv().expect("gate opened");

    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;

    let first = rx.try_recv().expect("Stargate_CrossGate must come first");
    match first {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id, args, ..
        } => {
            assert_eq!(witness_id, DIALER);
            assert_eq!(
                i32::from_le_bytes([args[0], args[1], args[2], args[3]]),
                SEQ_CROSS_GATE,
                "6113 resolves through the ORIGIN gate's event set"
            );
        }
        other => panic!("the crossing animation must precede everything else, got {other:?}"),
    }

    let second = rx
        .try_recv()
        .expect("onStargatePassage must follow the sequence");
    match second {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, DIALER, "sent to the traveller only, not fanned");
            assert_eq!(method_index, ON_STARGATE_PASSAGE, "client method 68");
            assert_eq!(
                args,
                2_i32.to_le_bytes().to_vec(),
                "addressId is the destination stargate id, 4-byte LE INT32 \
                 per entities/defs/interfaces/GateTravel.def"
            );
        }
        other => panic!("expected onStargatePassage EntityMethodCall, got {other:?}"),
    }

    // The movement-lock update is the third and last message this call
    // produces — `onStateFieldUpdate` (method 19) carrying the new
    // `state_field` with `BSF_MovementLock` set.
    let third = rx.try_recv().expect("movement-lock update must follow");
    match third {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, DIALER);
            assert_eq!(
                method_index,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE
            );
            let state_field = u32::from_le_bytes(args.try_into().unwrap());
            assert_ne!(
                state_field & BSF_MOVEMENT_LOCK,
                0,
                "the traveller must be movement-locked for the hold"
            );
        }
        other => panic!("expected onStateFieldUpdate EntityMethodCall, got {other:?}"),
    }

    assert!(
        rx.try_recv().is_err(),
        "no GateTravel yet — the world transition is deferred behind the \
         crossing hold, not issued synchronously"
    );

    assert!(
        mgr.get_entity(DIALER).is_some(),
        "the traveller is still resident in the OLD world during the hold"
    );
    assert!(mgr.gate_dial(DIALER).is_none(), "dial consumed on crossing");
    let hold = mgr
        .crossing_hold(DIALER)
        .expect("a crossing hold must be armed");
    assert_eq!(hold.target_address_id, 2);
    let locked_state = mgr
        .get_entity(DIALER)
        .expect("entity still resident")
        .state_field;
    assert_ne!(
        locked_state & BSF_MOVEMENT_LOCK,
        0,
        "the entity's own state_field must carry the lock, not just the \
         wire notification"
    );
}

/// The other half of the coordinator's hold-then-travel shape: once
/// `CROSSING_CINEMATIC_HOLD` elapses, `crossing_tick` runs the deferred
/// `GateTravel` and tears the entity down. Movement-lock cleanup is
/// implicit here (the entity no longer exists to hold a stale flag); the
/// explicit-cleanup path is `deferred_travel_failure_clears_the_movement_lock`
/// below.
#[tokio::test]
async fn crossing_hold_elapsing_runs_the_deferred_travel() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;
    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;
    rx.try_recv().expect("gate opened");

    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;
    // Drain the three crossing-start messages (sequence, passage, lock) —
    // asserted in detail by the test above.
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();
    rx.try_recv().unwrap();

    // A tick before the hold elapses must do nothing.
    crossing_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "a tick before the hold elapses must emit nothing"
    );
    assert!(mgr.get_entity(DIALER).is_some());

    expire_the_crossing_hold(&mut mgr, DIALER);
    crossing_tick(&tx, &mut mgr).await;

    let msg = rx
        .try_recv()
        .expect("GateTravel must be sent once the hold elapses");
    match msg {
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            ..
        } => {
            assert_eq!(entity_id, DIALER);
            assert_eq!(target_world_name, "Castle");
        }
        other => panic!("expected GateTravel, got {other:?}"),
    }

    assert!(mgr.get_entity(DIALER).is_none(), "entity torn down");
    assert!(
        mgr.crossing_hold(DIALER).is_none(),
        "the hold is consumed, not left armed"
    );

    // A second tick must not re-run the travel against a now-nonexistent
    // entity.
    crossing_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "the deferred travel must run exactly once per crossing"
    );
}

/// Disconnect during the hold (coordinator's fourth required test shape):
/// the pending crossing must be cancelled, not run against a departed
/// session, mirroring `disconnect_entity_drops_the_pending_dial` for the
/// dial timer.
#[tokio::test]
async fn disconnect_during_the_crossing_hold_cancels_the_deferred_travel() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;
    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;
    rx.try_recv().expect("gate opened");

    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;
    assert!(mgr.crossing_hold(DIALER).is_some(), "hold must be armed");

    // Expire the hold FIRST, so the test would fail if `disconnect_entity`
    // did not scrub the pending crossing — without this a tick emits
    // nothing anyway whether or not the scrub ran, proving nothing.
    expire_the_crossing_hold(&mut mgr, DIALER);
    mgr.disconnect_entity(DIALER, &tx).await;

    // Drain whatever `disconnect_entity` itself sent (AoI leave notices
    // etc.) before asserting on the crossing tick specifically.
    while rx.try_recv().is_ok() {}

    crossing_tick(&tx, &mut mgr).await;
    assert!(
        rx.try_recv().is_err(),
        "a disconnected traveller must not get a deferred GateTravel run \
         against their dead session"
    );
    assert!(mgr.crossing_hold(DIALER).is_none());
}

/// A deferred travel can still fail its own arrival validation (H01) after
/// the hold has already elapsed — the destination could vanish from the
/// stargate cache, or (as exercised here, the simplest failure to force)
/// the entity itself could be gone by the time the tick runs. Whichever
/// way `perform_gate_travel` returns `false`, the movement lock set at
/// crossing time must not be left stuck.
///
/// This test forces the failure via a destination that has no
/// `resources.stargates` row by the time the tick runs, which is the same
/// "destination vanished from the cache" branch `perform_gate_travel`
/// already logs — cheaper to construct here than an off-mesh arrival
/// fixture, and it exercises the exact code path `crossing_tick`'s
/// failure arm covers.
#[tokio::test]
async fn deferred_travel_failure_clears_the_movement_lock() {
    let (mut mgr, mut rx, tx) = armed_dialer().await;
    expire_the_timer(&mut mgr);
    gate_dial_tick(&tx, &mut mgr).await;
    rx.try_recv().expect("gate opened");

    handle_stargate_region_entered(DIALER, &tx, &mut mgr, &engine()).await;
    while rx.try_recv().is_ok() {}

    assert_ne!(
        mgr.get_entity(DIALER).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "precondition: movement must be locked for the hold"
    );

    // The destination vanishes from the cache before the hold elapses —
    // e.g. a hot-reload of `resources.stargates` mid-session.
    mgr.stargates.remove(&2);

    expire_the_crossing_hold(&mut mgr, DIALER);
    crossing_tick(&tx, &mut mgr).await;

    assert!(
        mgr.get_entity(DIALER).is_some(),
        "a failed deferred travel must leave the traveller in place"
    );
    assert_eq!(
        mgr.get_entity(DIALER).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "the movement lock must be released on a failed deferred travel — \
         otherwise the traveller is stuck immobile with no completed travel"
    );

    // The lock-release itself must have gone out over the wire, not just
    // updated local state.
    let unlock = rx
        .try_recv()
        .expect("a movement-lock release must be sent on failure");
    match unlock {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(entity_id, DIALER);
            assert_eq!(
                method_index,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE
            );
            let state_field = u32::from_le_bytes(args.try_into().unwrap());
            assert_eq!(state_field & BSF_MOVEMENT_LOCK, 0);
        }
        other => panic!("expected an onStateFieldUpdate release, got {other:?}"),
    }
}

/// Event ids are original data (`entities/defs/enumerations.xml:821,834`)
/// — pin them so a "tidy up the constants" refactor can't quietly point
/// the emitter at `Stargate_DestroyGate` (6103) or a chevron event
/// (6106-6112), which the 2009 server never sent.
///
/// That the emitter *refuses* an unmapped event is a separate claim, and
/// `sequences::missing_event_set_or_sequence_emits_nothing` pins it
/// against 6103. Asserting here that the fixture's own two-entry map
/// lacks the other twelve would only be the fixture checking itself.
#[test]
fn only_6100_and_6113_are_wired() {
    assert_eq!(EVENT_STARGATE_MAKE_GATE, 6100);
    assert_eq!(EVENT_STARGATE_CROSS_GATE, 6113);
}
