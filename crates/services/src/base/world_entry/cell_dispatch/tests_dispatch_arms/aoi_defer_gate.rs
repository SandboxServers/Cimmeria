//! AoI-bearing dispatch arms: pre/post-`onClientReady` behavior.
//!
//! The defer-gate is the load-bearing piece behind the deferred-AoI
//! buffer. Each arm must:
//!
//! - `EnteredAoI` — buffer pre-ready, emit phase-1 + phase-2 packets post-ready.
//! - `LeftAoI`    — buffer pre-ready (pairs with buffered ENTERED on flush).
//! - `EntityMoved`— **drop** pre-ready (position is unreliable and
//!   superseded by the next frame; buffering would target an entity the
//!   client hasn't created yet).
//! - `EntityMethodCall` — buffer pre-ready.
//! - `EntityMethodCallBatch` — buffer pre-ready, unrolling N calls into
//!   N individual buffer entries (the flush path replays them via the
//!   per-call handler).

use super::super::*;
use super::one_session;
use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::test_support::TestTransport;

/// Pre-`onClientReady`: `EnteredAoI` must buffer into `deferred_aoi_msgs`
/// and NOT hit the wire. Reverting the `should_defer` check on the
/// `EnteredAoI` arm would surface here: post-revert, the wire would see
/// 2 packets (phase-1 + phase-2 of `entered_aoi`) and the buffer would
/// stay empty.
#[tokio::test]
async fn entered_aoi_pre_client_ready_buffers_and_skips_wire() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 700u32;
    let (addr, connected, entity_to_addr) = one_session(witness_id, /*pre_ready=*/ true);

    handle_cell_message(
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id: 800,
            space_id: 1,
            class_id: 1,
            position: [1.0, 2.0, 3.0],
            direction: [0.0, 0.0, 0.0],
            level: 5,
            npc_data: None,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(
        typed_transport.is_empty(),
        "pre-onClientReady EnteredAoI must NOT emit on the wire"
    );
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    assert_eq!(buf.len(), 1, "EnteredAoI must be buffered exactly once");
    match &buf[0] {
        DeferredAoiMsg::EnteredAoI {
            entity_id,
            class_id,
            position,
            level,
            ..
        } => {
            assert_eq!(*entity_id, 800);
            assert_eq!(*class_id, 1);
            assert_eq!(*position, [1.0, 2.0, 3.0]);
            assert_eq!(*level, 5);
        }
        other => panic!("expected DeferredAoiMsg::EnteredAoI, got {other:?}"),
    }
}

/// Post-`onClientReady` (default session): `EnteredAoI` bypasses the
/// defer gate and reaches `aoi::entered_aoi`, which emits the phase-1
/// CREATE_ENTITY + UPDATE_AVATAR packet and the phase-2 property
/// cascade — exactly 2 reliable packets, both to the witness's addr.
#[tokio::test]
async fn entered_aoi_post_client_ready_emits_two_packets_to_witness_addr() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 701u32;
    let (addr, connected, entity_to_addr) = one_session(witness_id, /*pre_ready=*/ false);

    handle_cell_message(
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id: 801,
            space_id: 1,
            class_id: 1,
            position: [0.0; 3],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert_eq!(
        typed_transport.send_count_to(addr),
        2,
        "post-ready EnteredAoI emits exactly 2 packets (phase-1 + phase-2)"
    );
    assert_eq!(typed_transport.len(), 2, "no traffic to any other address");
    assert!(
        connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .deferred_aoi_msgs
            .is_empty(),
        "post-ready EnteredAoI must NOT push to the deferred buffer"
    );
}

/// Pre-`onClientReady`: `LeftAoI` must buffer (not drop, not emit).
/// Buffering is required so a LEFT event that races with its matching
/// ENTERED event preserves the pairing on flush (both buffered, both
/// dispatched in order). Dropping LEFT pre-ready would orphan the
/// ENTERED on the client side.
#[tokio::test]
async fn left_aoi_pre_client_ready_buffers_and_skips_wire() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 702u32;
    let (addr, connected, entity_to_addr) = one_session(witness_id, true);

    handle_cell_message(
        CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id: 802,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(
        typed_transport.is_empty(),
        "pre-ready LeftAoI must not emit"
    );
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    assert_eq!(buf.len(), 1, "LeftAoI must be buffered exactly once");
    match &buf[0] {
        DeferredAoiMsg::LeftAoI { entity_id } => assert_eq!(*entity_id, 802),
        other => panic!("expected DeferredAoiMsg::LeftAoI, got {other:?}"),
    }
}

/// Pre-`onClientReady`: `EntityMoved` is DROPPED (not buffered, not
/// emitted). Position updates are unreliable and self-correcting; the
/// next post-ready frame supersedes anything we would have buffered.
/// A regression that switched the EntityMoved arm to `push_deferred`
/// would surface as a non-empty buffer here.
#[tokio::test]
async fn entity_moved_pre_client_ready_drops_silently_without_buffering() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 703u32;
    let (addr, connected, entity_to_addr) = one_session(witness_id, true);

    handle_cell_message(
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id: 803,
            space_id: 1,
            position: [1.0; 3],
            direction: [0.0; 3],
            velocity: [0.5; 3],
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(
        typed_transport.is_empty(),
        "pre-ready EntityMoved must not emit"
    );
    assert!(
        connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .deferred_aoi_msgs
            .is_empty(),
        "EntityMoved is DROPPED pre-ready, never buffered (next position frame supersedes)"
    );
}

/// Pre-`onClientReady`: `EntityMethodCall` whose target is the
/// pre-ready session must buffer. Reverting the defer check would
/// fire the method against a client that hasn't yet created the
/// receiving entity.
#[tokio::test]
async fn entity_method_call_pre_client_ready_buffers_and_skips_wire() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let entity_id = 704u32;
    let (addr, connected, entity_to_addr) = one_session(entity_id, true);

    handle_cell_message(
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: 0x42,
            args: vec![0xAA, 0xBB, 0xCC],
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(typed_transport.is_empty());
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    assert_eq!(buf.len(), 1);
    match &buf[0] {
        DeferredAoiMsg::EntityMethodCall {
            entity_id: eid,
            method_index,
            args,
        } => {
            assert_eq!(*eid, 704);
            assert_eq!(*method_index, 0x42);
            assert_eq!(*args, vec![0xAA, 0xBB, 0xCC]);
        }
        other => panic!("expected DeferredAoiMsg::EntityMethodCall, got {other:?}"),
    }
}

/// Pre-`onClientReady`: `EntityMethodCallBatch` must unroll its N
/// calls into N separate `EntityMethodCall` entries in the deferred
/// buffer. The flush path replays them via the single-call handler,
/// so a regression that buffered the whole batch as one entry (or
/// dropped the unroll loop) would surface here as `buf.len() == 1` or
/// `0` instead of `3`.
#[tokio::test]
async fn entity_method_call_batch_pre_ready_unrolls_into_individual_buffer_entries() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let entity_id = 705u32;
    let (addr, connected, entity_to_addr) = one_session(entity_id, true);

    let calls = vec![
        (0x10u16, vec![0x01]),
        (0x11u16, vec![0x02, 0x03]),
        (0x12u16, vec![0x04, 0x05, 0x06]),
    ];

    handle_cell_message(
        CellToBaseMsg::EntityMethodCallBatch {
            entity_id,
            calls: calls.clone(),
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(typed_transport.is_empty(), "pre-ready batch must not emit");
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    assert_eq!(
        buf.len(),
        3,
        "batch must unroll into N individual EntityMethodCall buffer entries"
    );
    for (i, expected) in calls.iter().enumerate() {
        match &buf[i] {
            DeferredAoiMsg::EntityMethodCall {
                entity_id: eid,
                method_index,
                args,
            } => {
                assert_eq!(*eid, 705, "entry {i}: entity_id");
                assert_eq!(*method_index, expected.0, "entry {i}: method_index");
                assert_eq!(*args, expected.1, "entry {i}: args");
            }
            other => panic!("entry {i}: expected EntityMethodCall, got {other:?}"),
        }
    }
}
