//! Tests for the `handle_cell_message` dispatch.

use super::*;
use crate::test_support::TestTransport;

fn empty_maps() -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    (
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    )
}

#[tokio::test]
async fn minigame_result_forwards_to_cell_service() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr) = empty_maps();
    let (cell_tx, mut cell_rx) = mpsc::channel(1);

    handle_cell_message(
        CellToBaseMsg::MinigameResult {
            entity_id: 10,
            result_code: 2,
            on_victory_chains: vec![100, 200],
        },
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    match cell_rx.try_recv().expect("minigame result forwarded") {
        BaseToCellMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        } => {
            assert_eq!(entity_id, 10);
            assert_eq!(result_code, 2);
            assert_eq!(on_victory_chains, vec![100, 200]);
        }
        // BaseToCellMsg deliberately omits Debug (oneshot::Sender),
        // so we can't print the variant — name the expected one
        // and let test output point at this line.
        _ => panic!("expected BaseToCellMsg::MinigameResult"),
    }
}

/// `flush_deferred_aoi` drains the session's deferred-AoI buffer through
/// the normal AoI handlers once `onClientReady` fires. Pins:
///   1. drain — the buffer is empty after the call.
///   2. dispatch reaches the AoI handlers — the legacy send path
///      (`send_to_witness_reliable`) increments `state.next_seq` and
///      mirrors each reliable send into the per-session `Channel`'s
///      TX window via `register_sent_packet`. Buffer of 3 messages
///      (EnteredAoI = 2 packets, EntityMethodCall = 1, LeftAoI = 1)
///      = 4 packets, so `next_seq` advances 0 → 4 and `tx_window`
///      gains 4 entries.
#[tokio::test]
async fn flush_deferred_aoi_drains_buffer_and_dispatches_to_aoi_handlers() {
    use crate::base::deferred_aoi::DeferredAoiMsg;
    use crate::test_support::test_default_connected_client_state;
    use std::sync::atomic::Ordering;

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let witness_addr: SocketAddr = "127.0.0.1:54321".parse().unwrap();
    let witness_id: u32 = 100;

    // Session in the pre-`onClientReady` state with 3 buffered AoI events.
    let mut state = test_default_connected_client_state();
    state.deferred_aoi_msgs.push(DeferredAoiMsg::EnteredAoI {
        entity_id: 200,
        class_id: 1,
        position: [0.0; 3],
        direction: [0.0; 3],
        level: 1,
        npc_data: None,
    });
    state
        .deferred_aoi_msgs
        .push(DeferredAoiMsg::EntityMethodCall {
            entity_id: 200,
            method_index: 0x10,
            args: vec![0xAA, 0xBB],
        });
    state
        .deferred_aoi_msgs
        .push(DeferredAoiMsg::LeftAoI { entity_id: 200 });

    let connected = Arc::new(Mutex::new(HashMap::from([(witness_addr, state)])));
    // EntityMethodCall dispatches to the target entity's owning client
    // (looked up via entity_to_addr.get(&entity_id)), so entity 200 also
    // needs a mapping. In production the cell only buffers method calls
    // where this lookup succeeded; the test reflects that invariant.
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (200u32, witness_addr),
    ])));

    super::aoi::flush_deferred_aoi(
        witness_id,
        witness_addr,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    // Post-flush: buffer drained, channel saw 4 outbound reliable packets.
    let clients = connected.lock().unwrap();
    let s = clients.get(&witness_addr).unwrap();
    assert!(
        s.deferred_aoi_msgs.is_empty(),
        "flush drains the deferred-AoI buffer"
    );
    assert_eq!(
        s.next_seq.load(Ordering::Relaxed),
        4,
        "AoI handlers ran — 2 packets for EnteredAoI + 1 for EntityMethodCall + 1 for LeftAoI",
    );
    assert_eq!(
        s.channel.lock().unwrap().tx_window.len(),
        4,
        "each reliable send mirrored into the channel's TX window"
    );
}

/// `flush_deferred_aoi` on a session with an empty buffer is a no-op —
/// returns without touching the channel or panicking. This is the steady-
/// state path (most clients arrive at `onClientReady` with nothing to
/// flush) and must stay cheap.
#[tokio::test]
async fn flush_deferred_aoi_is_noop_on_empty_buffer() {
    use crate::test_support::test_default_connected_client_state;
    use std::sync::atomic::Ordering;

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let witness_addr: SocketAddr = "127.0.0.1:54322".parse().unwrap();
    let witness_id: u32 = 101;

    let state = test_default_connected_client_state();
    let connected = Arc::new(Mutex::new(HashMap::from([(witness_addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(witness_id, witness_addr)])));

    super::aoi::flush_deferred_aoi(
        witness_id,
        witness_addr,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let clients = connected.lock().unwrap();
    let s = clients.get(&witness_addr).unwrap();
    assert_eq!(
        s.next_seq.load(Ordering::Relaxed),
        0,
        "empty-buffer flush emits zero packets"
    );
    assert!(s.channel.lock().unwrap().tx_window.is_empty());
}

#[tokio::test]
async fn invalid_bandolier_ammo_update_drops_before_side_effects() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr) = empty_maps();
    let (cell_tx, mut cell_rx) = mpsc::channel(1);

    handle_cell_message(
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id: 10,
            slot_id: -1,
            expected_item_id: 42,
            current_ammo: 17,
            cur_ammo_type: 1,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(
        cell_rx.try_recv().is_err(),
        "invalid payload must not forward"
    );
    assert!(connected.lock().unwrap().is_empty());
    assert!(entity_to_addr.lock().unwrap().is_empty());
}
