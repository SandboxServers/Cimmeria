//! Dispatch arms under the first-login cinematic AoI hold.
//!
//! The hold is post-`onClientReady`, so `pending_client_ready` is already
//! gone and only `cinematic_aoi_hold` keeps traffic back. Bug shapes pinned:
//!
//! 1. **Hold ignored on an entity-scoped arm** — the create reaches a client
//!    that is playing a fullscreen movie, which is the invisible-corpse repro.
//! 2. **Dependent traffic overtakes the held create** — `WitnessEntityMethod`
//!    and `EntityInvisible` are ungated pre-ready; under the hold they must
//!    buffer, or they hit a client with no such entity and are lost.
//! 3. **Hold leaks onto player-self traffic** — mission, dialog and hotbar
//!    calls must still flow during the movie.

use std::time::Instant;

use super::super::*;
use super::one_session;
use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::base::world_entry_appearance::CinematicAoiHold;
use crate::test_support::TestTransport;

/// A post-ready session with an active cinematic hold.
fn held_session(
    entity_id: u32,
) -> (
    SocketAddr,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let (addr, connected, entity_to_addr) = one_session(entity_id, /*pre_ready=*/ false);
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .cinematic_aoi_hold = Some(CinematicAoiHold {
        token: 1,
        started: Instant::now(),
    });
    (addr, connected, entity_to_addr)
}

async fn dispatch(
    msg: CellToBaseMsg,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_cell_message(
        msg,
        transport,
        connected,
        entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;
}

/// The repro shape: a `class_id 0` static entity enters the witness's AoI
/// after `onClientReady` while the movie plays. Reverting the
/// `should_hold_entity_traffic` gate sends its two create packets straight
/// to the client and leaves the buffer empty.
#[tokio::test]
async fn entered_aoi_during_cinematic_hold_buffers_and_skips_wire() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 710u32;
    let (addr, connected, entity_to_addr) = held_session(witness_id);

    dispatch(
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id: 810,
            space_id: 1,
            class_id: 0,
            position: [-322.5, 73.47, -209.83],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
            player_data: None,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert!(
        typed_transport.is_empty(),
        "an entity introduction must not reach a client that is mid-cinematic"
    );
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    assert!(
        matches!(
            buf.as_slice(),
            [DeferredAoiMsg::EnteredAoI {
                entity_id: 810,
                class_id: 0,
                ..
            }]
        ),
        "EnteredAoI must be buffered exactly once: {buf:?}"
    );
}

/// `LeftAoI` buffers behind the held create; `EntityMoved` drops, exactly as
/// it does pre-ready.
#[tokio::test]
async fn left_aoi_buffers_and_entity_moved_drops_during_cinematic_hold() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 711u32;
    let (addr, connected, entity_to_addr) = held_session(witness_id);

    dispatch(
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id: 811,
            space_id: 1,
            position: [1.0; 3],
            direction: [0.0; 3],
            velocity: [0.5; 3],
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    dispatch(
        CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id: 811,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert!(typed_transport.is_empty(), "nothing reaches the wire");
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    assert!(
        matches!(buf.as_slice(), [DeferredAoiMsg::LeftAoI { entity_id: 811 }]),
        "LeftAoI buffered, EntityMoved dropped: {buf:?}"
    );
}

/// `WitnessEntityMethod` and `EntityInvisible` are ungated pre-ready, so the
/// hold is the only thing keeping them behind the create they depend on.
/// Reverting `held_witness_addr` on either arm sends one packet here.
#[tokio::test]
async fn witness_method_and_invisible_buffer_in_order_during_cinematic_hold() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 712u32;
    let (addr, connected, entity_to_addr) = held_session(witness_id);

    dispatch(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id: 812,
            method_index: 3,
            args: vec![0x00, 0x00, 0x40, 0x00],
            entity_is_player: false,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    dispatch(
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id: 812,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert!(typed_transport.is_empty(), "nothing reaches the wire");
    let clients = connected.lock().unwrap();
    let buf = &clients.get(&addr).unwrap().deferred_aoi_msgs;
    match buf.as_slice() {
        [DeferredAoiMsg::WitnessEntityMethod {
            entity_id: 812,
            method_index: 3,
            args,
            entity_is_player: false,
        }, DeferredAoiMsg::EntityInvisible { entity_id: 812 }] => {
            assert_eq!(args, &vec![0x00, 0x00, 0x40, 0x00]);
        }
        other => panic!("expected WitnessEntityMethod then EntityInvisible, got {other:?}"),
    }
}

/// Without a hold the two arms keep their pre-existing behavior: one packet,
/// no buffering. Guards against the hold gate firing for every session.
#[tokio::test]
async fn witness_method_without_hold_still_sends_immediately() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 713u32;
    let (addr, connected, entity_to_addr) = one_session(witness_id, /*pre_ready=*/ false);

    dispatch(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id: 813,
            method_index: 3,
            args: vec![0x01],
            entity_is_player: false,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert_eq!(typed_transport.send_count_to(addr), 1);
    assert!(connected
        .lock()
        .unwrap()
        .get(&addr)
        .unwrap()
        .deferred_aoi_msgs
        .is_empty());
}

/// The hold is about entities the client has not created yet. The player's
/// own entity exists, so its method calls — mission accept, intro dialog,
/// hotbar seed — must not wait for a 13-second movie.
#[tokio::test]
async fn player_self_method_call_is_not_held_by_cinematic_hold() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let entity_id = 714u32;
    let (addr, connected, entity_to_addr) = held_session(entity_id);

    dispatch(
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: 0x42,
            args: vec![0xAA],
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert_eq!(
        typed_transport.send_count_to(addr),
        1,
        "player-self method call goes out during the hold"
    );
    assert!(connected
        .lock()
        .unwrap()
        .get(&addr)
        .unwrap()
        .deferred_aoi_msgs
        .is_empty());
}
