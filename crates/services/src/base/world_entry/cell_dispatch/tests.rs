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
///   2. dispatch reaches the wire — `state.next_seq` advances and each
///      reliable send mirrors into the per-session `Channel`'s TX window
///      via `register_sent_packet`.
///
/// With the issue #356 bundle migration of EnteredAoI, the per-NPC
/// packet shape at N=1 is unchanged from the pre-bundle path: phase-1
/// and phase-2 are still split across two packets (the cross-entity
/// batching savings show up at N>1 — see
/// [`flush_deferred_aoi_bundles_28_npc_burst_into_seven_packets`]).
/// One NPC's EnteredAoI = 2 packets (phase-1 bundle + phase-2 bundle),
/// EntityMethodCall = 1, LeftAoI = 1, totalling 4 packets — same as
/// the pre-bundle path emitted.
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

/// Issue #356 burst-reduction regression guard. The Castle_CellBlock
/// instance has 28 NPCs; before the bundle migration each NPC's
/// EnteredAoI emitted 2 separate reliable packets (CREATE_ENTITY +
/// UPDATE_AVATAR pair, then property cascade), totalling **56 reliable
/// packets** through the deferred-AoI flush. That saturated the 32-slot
/// TX window before mapLoaded's own burst could land.
///
/// With the bundle migration the same 28 EnteredAoIs collapse to two
/// cross-entity bundles — phase-1 (CREATE_ENTITY/UPDATE_AVATAR per NPC,
/// ~37 bytes each → ~1KB body → 1 packet) and phase-2 (cascade per NPC,
/// the bigger one because of stat updates → 28×442 = ~12KB body →
/// 10 packets after fragmentation at 1300 bytes). Total = **~11
/// packets**, a ~5× reduction.
///
/// If this assertion fires below the asserted-upper bound, the bundle
/// path probably regressed back to per-NPC emit. If above the bound,
/// either the cascade got bigger (legitimate — bump the bound) or
/// fragmentation broke and we're emitting one packet per message
/// again (regression — investigate).
#[tokio::test]
async fn flush_deferred_aoi_bundles_28_npc_burst_under_packet_budget() {
    use crate::base::deferred_aoi::DeferredAoiMsg;
    use crate::test_support::test_default_connected_client_state;
    use std::sync::atomic::Ordering;

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let witness_addr: SocketAddr = "127.0.0.1:54323".parse().unwrap();
    let witness_id: u32 = 200;

    let mut state = test_default_connected_client_state();
    // 28 NPCs — the Castle_CellBlock first-load burst the bundle
    // abstraction was designed to absorb.
    for i in 0..28 {
        state.deferred_aoi_msgs.push(DeferredAoiMsg::EnteredAoI {
            entity_id: 1000 + i,
            class_id: 1,
            position: [(i as f32) * 10.0, 0.0, 0.0],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
        });
    }
    assert_eq!(state.deferred_aoi_msgs.len(), 28);

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
    let packet_count = s.next_seq.load(Ordering::Relaxed) as usize;
    let tx_window_len = s.channel.lock().unwrap().tx_window.len();

    // Upper bound: 28-NPC burst must finalize into well under the
    // pre-bundle 56-packet shape. 15 is comfortable headroom against
    // future cascade-payload growth while still being a strong signal
    // that bundling is engaged (the unmigrated path would be 56).
    assert!(
        packet_count <= 15,
        "28-NPC AoI burst must bundle to <= 15 packets (pre-bundle was 56); got {packet_count}",
    );
    // Lower bound: must be more than 1 — if it were just 1 packet,
    // either fragmentation broke (unlikely — phase-2 alone is ~12KB)
    // or the test wired no actual sends through (a setup bug).
    assert!(
        packet_count >= 2,
        "28-NPC burst must emit phase-1 AND phase-2 bundles (≥2 packets); got {packet_count}",
    );
    assert_eq!(
        tx_window_len, packet_count,
        "every bundle-emitted packet must mirror into the TX window for retransmit"
    );
    assert!(
        s.deferred_aoi_msgs.is_empty(),
        "burst flush drains the buffer"
    );
}
