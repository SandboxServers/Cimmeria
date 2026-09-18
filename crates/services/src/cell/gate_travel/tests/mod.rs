//! `cell::gate_travel` tests.
//!
//! - this file — `onDialGate` validation and the arm-vs-travel decision.
//! - [`sequences`] — `Stargate_MakeGate` / `Stargate_CrossGate` wire
//!   layout and witness fan-out cardinality.
//! - [`dial_timer`] — the 4-second timer, its cancellations, and the
//!   crossing gate.

use super::super::spawner::StargateEntry;
use super::*;

mod dial_timer;
mod sequences;

/// Castle's real gate event set (`stargates.event_set_id` for
/// `stargate_id = 2`). Its sequences are 10145 (6100) … 10158 (6113).
pub(super) const CASTLE_EVENT_SET: i32 = 10011;
pub(super) const SEQ_MAKE_GATE: i32 = 10145;
pub(super) const SEQ_CROSS_GATE: i32 = 10158;

/// Two-world fixture with a stargate in each.
///
/// Agnos additionally carries a `REGION_FLAG_Stargate` region so the
/// arm-then-cross path is the default in these tests; the
/// no-gate-region fallback is opted into with
/// [`strip_stargate_regions`].
pub(super) fn make_manager_with_stargates() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" />
            <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        </Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Agnos" />
            <Space WorldName="Castle" />
        </Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    // Populate stargates cache (simulates DB load). Gate 1 is Agnos's own
    // gate — the lowest stargate_id in that world, so it is also the
    // `dialingStargate` whose event set supplies the sequences.
    mgr.stargates.insert(
        1,
        StargateEntry {
            world_name: "Agnos".to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            event_set_id: Some(CASTLE_EVENT_SET),
        },
    );
    mgr.stargates.insert(
        2,
        StargateEntry {
            world_name: "Castle".to_string(),
            x: 761.677,
            y: 63.466,
            z: 551.716,
            yaw: 2.152,
            event_set_id: Some(CASTLE_EVENT_SET),
        },
    );
    mgr.stargates.insert(
        15,
        StargateEntry {
            world_name: "Agnos".to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            event_set_id: None,
        },
    );

    mgr.sequence_map.insert(
        (CASTLE_EVENT_SET, super::sequences::EVENT_STARGATE_MAKE_GATE),
        SEQ_MAKE_GATE,
    );
    mgr.sequence_map.insert(
        (
            CASTLE_EVENT_SET,
            super::sequences::EVENT_STARGATE_CROSS_GATE,
        ),
        SEQ_CROSS_GATE,
    );

    register_stargate_region(&mut mgr, "Agnos", 1002);
    mgr
}

/// Register a `REGION_FLAG_Stargate | REGION_FLAG_ClientHinted` region,
/// the shape `point_sets` row 1002 (`Castle.Stargate`, flags = 3) loads as.
pub(super) fn register_stargate_region(mgr: &mut SpaceManager, world: &str, db_set_id: i32) -> u32 {
    use crate::cell::space_manager::{RegionData, REGION_FLAG_CLIENT_HINTED, REGION_FLAG_STARGATE};
    let runtime_id = mgr.next_region_id;
    mgr.next_region_id += 1;
    mgr.regions.insert(
        runtime_id,
        RegionData {
            runtime_id,
            db_set_id,
            tag: format!("{world}.Stargate"),
            world_name: world.to_string(),
            height: 10.0,
            radius: 2.5,
            flags: REGION_FLAG_CLIENT_HINTED | REGION_FLAG_STARGATE,
            points: vec![[0.0; 3]; 4],
        },
    );
    runtime_id
}

/// Drop every gate region so `handle_dial_gate` takes the
/// no-gate-volume fallback.
pub(super) fn strip_stargate_regions(mgr: &mut SpaceManager) {
    use crate::cell::space_manager::REGION_FLAG_STARGATE;
    mgr.regions
        .retain(|_, r| r.flags & REGION_FLAG_STARGATE == 0);
}

pub(super) fn engine() -> ChainEngine {
    ChainEngine::new()
}

#[tokio::test]
async fn dial_gate_to_unknown_address_is_noop() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 999, 0, &tx, &mut mgr, &engine()).await;
    assert!(rx.try_recv().is_err());
    assert!(
        mgr.gate_dial(1).is_none(),
        "no dial armed for a bad address"
    );
}

/// Every rejection branch cancels the dial in flight, as
/// `SGWPlayer.onDialGate` does (`SGWPlayer.py:2050`, `:2061`, `:2067`
/// each call `cancelDialing()` before returning).
///
/// The bug shape without this: dial Castle, then re-dial a bad address.
/// The Castle dial stays armed, opens on its timer, and the player walks
/// into the gate and is sent to a world they never dialled. All three
/// branches are exercised against a live armed dial.
#[tokio::test]
async fn a_rejected_dial_cancels_the_dial_already_in_flight() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    let (tx, _rx) = tokio::sync::mpsc::channel(16);

    // ── unknown address ──
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;
    assert!(mgr.gate_dial(1).is_some(), "precondition: a dial is armed");
    handle_dial_gate(1, 999, 0, &tx, &mut mgr, &engine()).await;
    assert!(
        mgr.gate_dial(1).is_none(),
        "an unknown address must cancel the dial in flight, not leave the \
         previous destination armed and crossable"
    );

    // ── same world as the dialer (gate 1 is Agnos's own gate) ──
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;
    assert!(mgr.gate_dial(1).is_some(), "precondition: a dial is armed");
    handle_dial_gate(1, 1, 0, &tx, &mut mgr, &engine()).await;
    assert!(
        mgr.gate_dial(1).is_none(),
        "dialling the world you are already in must cancel the dial"
    );

    // ── entity not found ──
    // `destroy_entity` scrubs the dial itself, so re-arm the map directly
    // afterwards to isolate `handle_dial_gate`'s entity-missing branch.
    mgr.destroy_entity(1);
    mgr.begin_gate_dial(1, 2, "Castle".to_string(), Some(CASTLE_EVENT_SET));
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;
    assert!(
        mgr.gate_dial(1).is_none(),
        "a dial from an entity that no longer exists must be cancelled, \
         not left to fire against a reused entity id"
    );
}

#[tokio::test]
async fn dial_gate_cancel_is_noop() {
    let mut mgr = make_manager_with_stargates();
    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, -1, 0, &tx, &mut mgr, &engine()).await;
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn dial_gate_same_world_is_noop() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 1, 0, &tx, &mut mgr, &engine()).await;
    assert!(rx.try_recv().is_err());
    assert!(mgr.get_entity(1).is_some());
    assert!(mgr.gate_dial(1).is_none());
}

/// A closed base channel must leave the traveller in place. Destroying
/// the entity first and *then* discovering the send failed produces a
/// player who is in no space with no transfer in flight — recoverable
/// only by relogging.
#[tokio::test]
async fn dial_gate_with_closed_base_channel_leaves_the_entity_in_place() {
    let mut mgr = make_manager_with_stargates();
    // Fallback path: this test is about the GateTravel send failing, so
    // it needs the dial to reach `perform_gate_travel` in one call.
    strip_stargate_regions(&mut mgr);
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    let space_before = mgr.get_entity_space_id(1);

    let (tx, rx) = tokio::sync::mpsc::channel(16);
    drop(rx); // base side is gone

    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;

    assert!(
        mgr.get_entity(1).is_some(),
        "a failed GateTravel enqueue must not tear the traveller out of their space"
    );
    assert_eq!(
        mgr.get_entity_space_id(1),
        space_before,
        "the traveller must still be in their origin space"
    );
}

/// On a world with no `REGION_FLAG_Stargate` volume there is nothing to
/// walk into, so the dial still travels immediately — the documented
/// fallback that keeps the ~18 gate-region-less worlds reachable.
#[tokio::test]
async fn dial_gate_without_a_gate_region_travels_immediately() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_manager_with_stargates();
    strip_stargate_regions(&mut mgr);
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;

    // Negative-logging convention: taking the fallback is a documented
    // divergence from the 2009 flow, so it must be visible in the log.
    // Silently travelling on the dial is indistinguishable from the
    // walk-through path in a live trace, which is exactly the "why did
    // this world behave differently?" question an operator will ask.
    assert!(
        capture
            .find_event(
                tracing::Level::WARN,
                "no REGION_FLAG_Stargate region",
                "no_stargate_region",
            )
            .is_some(),
        "must WARN with reason=no_stargate_region when the origin world \
         has no gate volume. Captured events: {:#?}",
        capture.all()
    );

    assert!(mgr.get_entity(1).is_none());

    let msg = rx.try_recv().expect("Expected GateTravel message");
    match msg {
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            destination_ring_id,
            destination_space_id,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(target_world_name, "Castle");
            assert!((position[0] - 761.677).abs() < 0.01);
            assert_eq!(destination_ring_id, None);
            assert_eq!(destination_space_id, None);
        }
        _ => panic!("Expected GateTravel message, got {:?}", msg),
    }
}

/// The CA10 behaviour change, stated as an assertion: on a world WITH a
/// gate region the dial arms and does not travel. Reverting the arm
/// branch turns this into an immediate `GateTravel`, which fails both
/// assertions.
#[tokio::test]
async fn dial_gate_with_a_gate_region_arms_instead_of_travelling() {
    let mut mgr = make_manager_with_stargates();
    mgr.create_entity(1, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);

    let (tx, mut rx) = tokio::sync::mpsc::channel(16);
    handle_dial_gate(1, 2, 0, &tx, &mut mgr, &engine()).await;

    assert!(
        mgr.get_entity(1).is_some(),
        "dialling must not tear the player out of their space — the gate \
         has to open first"
    );
    assert!(
        rx.try_recv().is_err(),
        "no GateTravel (and no sequence) until the 4s timer expires"
    );

    let dial = mgr.gate_dial(1).expect("the dial must be armed");
    assert_eq!(dial.target_address_id, 2);
    assert_eq!(dial.target_world_name, "Castle");
    assert_eq!(
        dial.origin_event_set_id,
        Some(CASTLE_EVENT_SET),
        "sequences come from the ORIGIN world's gate (Agnos gate 1), not \
         the dialled destination"
    );
    assert!(!dial.passable);
}

/// `origin_gate_event_set` picks the lowest `stargate_id` in the world —
/// the `world.stargates[0]` the Python indexed. Agnos has gates 1 (event
/// set present) and 15 (NULL); picking 15 would silently disable every
/// gate animation on that world.
#[test]
fn origin_event_set_comes_from_the_lowest_stargate_id_in_the_world() {
    let mgr = make_manager_with_stargates();
    assert_eq!(
        super::sequences::origin_gate_event_set(&mgr, "Agnos"),
        Some(CASTLE_EVENT_SET)
    );
    assert_eq!(
        super::sequences::origin_gate_event_set(&mgr, "Nowhere"),
        None
    );
}
