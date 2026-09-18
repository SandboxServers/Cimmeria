//! The 4-second dial timer, its cancellations, and the crossing gate.
//!
//! `SGWPlayer.beginDialing` arms the timer, `cancelDialing` drops it,
//! `gateDialTimerExpired` opens the gate exactly once, and
//! `stargatePassed` is a no-op unless `dialedAddress is not None and
//! gatePassable`.
//!
//! These drive `SpaceManager`'s deadline directly rather than sleeping
//! four real seconds — `take_opened_gate_dials` takes `now`, so the tests
//! move the clock instead of waiting on it. The one test that must go
//! through `gate_dial_tick` (which reads `Instant::now()` itself) rewinds
//! `open_at` into the past first.

use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use super::super::sequences::{EVENT_STARGATE_CROSS_GATE, EVENT_STARGATE_MAKE_GATE};
use super::super::{gate_dial_tick, handle_dial_gate, handle_stargate_region_entered};
use super::{engine, make_manager_with_stargates, SEQ_CROSS_GATE, SEQ_MAKE_GATE};
use crate::cell::messages::CellToBaseMsg;
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

/// The full crossing: `Stargate_CrossGate` goes out BEFORE the
/// `GateTravel` teardown, and the dial is consumed so a second crossing
/// can't re-travel.
#[tokio::test]
async fn crossing_an_open_gate_sends_cross_gate_then_travels() {
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
        other => panic!("the crossing animation must precede the teardown, got {other:?}"),
    }

    let second = rx.try_recv().expect("GateTravel must follow");
    match second {
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
    assert!(mgr.gate_dial(DIALER).is_none(), "dial consumed");
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
