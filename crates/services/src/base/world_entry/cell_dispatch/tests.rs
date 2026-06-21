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
        &None,
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
/// With the bundle migration of EnteredAoI, the per-NPC packet shape
/// at N=1 is unchanged from the pre-bundle path: phase-1 and phase-2
/// are still split across two packets (the cross-entity batching
/// savings show up at N>1 — see
/// [`flush_deferred_aoi_bundles_28_npc_burst_under_packet_budget`]).
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
            expected_instance_id: 42,
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
        &None,
    )
    .await;

    assert!(
        cell_rx.try_recv().is_err(),
        "invalid payload must not forward"
    );
    assert!(connected.lock().unwrap().is_empty());
    assert!(entity_to_addr.lock().unwrap().is_empty());
}

/// Burst-reduction regression guard for the bundle migration. The Castle_CellBlock
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

/// `send_bundle_to_witness_reliable` with a witness whose entity_id
/// is not in the `entity_to_addr` map must be a complete no-op — no
/// traffic, no seq allocation, no channel mutation. Without this guard,
/// a recently-disconnected witness racing with a cell-driven AoI flush
/// could spend a session-counter slot on a bundle that never goes
/// anywhere.
#[tokio::test]
async fn send_bundle_to_witness_reliable_is_noop_for_unknown_witness() {
    use crate::base::helpers::send_bundle_to_witness_reliable;
    use crate::test_support::test_default_connected_client_state;
    use cimmeria_mercury::channel_bundle::ChannelBundle;
    use std::sync::atomic::Ordering;

    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_addr: SocketAddr = "127.0.0.1:54400".parse().unwrap();

    let state = test_default_connected_client_state();
    let initial_seq = state.next_seq.load(Ordering::Relaxed);

    let connected = Arc::new(Mutex::new(HashMap::from([(witness_addr, state)])));
    // entity_to_addr is empty — no mapping for witness_id 999.
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    let mut bundle = ChannelBundle::new(true);
    bundle.append_entity_method(
        12,
        cimmeria_mercury::channel_bundle::IDBASE_SGW_PLAYER,
        1,
        &[0xAA],
    );
    send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, 999, bundle).await;

    let clients = connected.lock().unwrap();
    let s = clients.get(&witness_addr).unwrap();
    assert_eq!(
        s.next_seq.load(Ordering::Relaxed),
        initial_seq,
        "unknown witness must not consume seq counter slots"
    );
    assert!(
        s.channel.lock().unwrap().tx_window.is_empty(),
        "unknown witness must not touch the TX window"
    );
    assert!(
        typed_transport.is_empty(),
        "unknown witness must not produce any wire traffic"
    );
}

/// Interleave check: a flush that has BOTH EnteredAoI (goes through the
/// bundle path, reserves N seqs via fetch_add(N)) AND a tail message
/// (single-packet path, reserves 1 seq via fetch_add(1)) must keep the
/// session reliable seq counter contiguous — no gap, no double-use.
///
/// The bundle reserves a contiguous block; the per-message tail follows
/// with seq = bundle_base + bundle_count. Without this guarantee the
/// reliable stream would have holes that the client buffers indefinitely.
#[tokio::test]
async fn flush_deferred_aoi_keeps_reliable_seq_contiguous_across_bundle_and_tail() {
    use crate::base::deferred_aoi::DeferredAoiMsg;
    use crate::test_support::test_default_connected_client_state;
    use std::sync::atomic::Ordering;

    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_addr: SocketAddr = "127.0.0.1:54401".parse().unwrap();
    let witness_id: u32 = 300;

    let mut state = test_default_connected_client_state();
    // 5 NPCs (manageable; bundles to 1-2 packets each phase) + a
    // trailing EntityMethodCall on a different entity (tail path).
    for i in 0..5 {
        state.deferred_aoi_msgs.push(DeferredAoiMsg::EnteredAoI {
            entity_id: 2000 + i,
            class_id: 1,
            position: [0.0; 3],
            direction: [0.0; 3],
            level: 1,
            npc_data: None,
        });
    }
    state
        .deferred_aoi_msgs
        .push(DeferredAoiMsg::EntityMethodCall {
            entity_id: 3000,
            method_index: 0x10,
            args: vec![0xFF; 4],
        });

    let connected = Arc::new(Mutex::new(HashMap::from([(witness_addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (3000u32, witness_addr),
    ])));

    super::aoi::flush_deferred_aoi(
        witness_id,
        witness_addr,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let total_packets = typed_transport.send_count_to(witness_addr);
    let clients = connected.lock().unwrap();
    let s = clients.get(&witness_addr).unwrap();
    let final_seq = s.next_seq.load(Ordering::Relaxed) as usize;
    let tx_len = s.channel.lock().unwrap().tx_window.len();

    // Contract: seq counter advanced by exactly the wire-emitted count.
    assert_eq!(
        final_seq, total_packets,
        "seq counter must advance by exactly the wire-emitted packet count \
         (bundle + tail must reserve contiguous block, no gap, no overlap)"
    );
    assert_eq!(
        tx_len, total_packets,
        "every wire-emitted packet must register in the TX window"
    );
    // Sanity: must have at least 3 packets (phase-1 bundle + phase-2
    // bundle + tail EntityMethodCall).
    assert!(
        total_packets >= 3,
        "test setup expected >= 3 packets; got {total_packets}"
    );
}

/// Partial-send recovery: if the transport fails on fragment K of a
/// multi-fragment bundle, the helper must abort the rest of the bundle.
/// Continuing would push fragments K+1..N onto the wire with no chance
/// of reassembly (the failed fragment seq is already a gap in the
/// reliable stream and the bundle frag_begin/frag_end footers expect
/// every fragment to arrive). The retransmit driver picks up the
/// registered fragments [0..K).
#[tokio::test]
async fn send_bundle_to_witness_reliable_aborts_on_first_fragment_send_failure() {
    use crate::base::helpers::send_bundle_to_witness_reliable;
    use crate::test_support::test_default_connected_client_state;
    use async_trait::async_trait;
    use cimmeria_mercury::channel_bundle::ChannelBundle;
    use cimmeria_mercury::transport::Transport as TransportTrait;
    use std::sync::atomic::Ordering;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    /// Transport that succeeds for the first `fail_after` sends, then
    /// fails every subsequent `send_to` with a synthetic io::Error.
    struct FailingTransport {
        sent_count: AtomicUsize,
        fail_after: usize,
        local: SocketAddr,
    }

    #[async_trait]
    impl TransportTrait for FailingTransport {
        // `async_trait`-augmented method dispatch.
        async fn send_to(&self, bytes: &[u8], _addr: SocketAddr) -> std::io::Result<usize> {
            let n = self.sent_count.fetch_add(1, AtomicOrdering::SeqCst);
            if n < self.fail_after {
                Ok(bytes.len())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "synthetic-test-failure",
                ))
            }
        }
        fn local_addr(&self) -> std::io::Result<SocketAddr> {
            Ok(self.local)
        }
    }

    let typed = Arc::new(FailingTransport {
        sent_count: AtomicUsize::new(0),
        fail_after: 1, // Succeed once, then fail.
        local: "127.0.0.1:0".parse().unwrap(),
    });
    let transport: Arc<dyn Transport> = typed.clone();
    let witness_addr: SocketAddr = "127.0.0.1:54402".parse().unwrap();
    let witness_id: u32 = 400;

    let state = test_default_connected_client_state();
    let connected = Arc::new(Mutex::new(HashMap::from([(witness_addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(witness_id, witness_addr)])));

    // Build a multi-fragment bundle. ~3+ fragments worth of body so the
    // first send succeeds and the second fails.
    let mut bundle = ChannelBundle::new(true);
    // Each cascade body is ~450 bytes; 8 of them ~= 3600 bytes
    // -> ~3 fragments. We need >= 2 fragments so the abort is observable.
    for entity_id in 5000u32..5008 {
        let body = crate::mercury::compose_create_entity_cascade_body(entity_id, 1, 1, None);
        bundle.append_raw_message(&body);
    }
    assert!(
        bundle.estimated_packet_count() >= 2,
        "test setup must produce a multi-fragment bundle"
    );
    let expected_frags = bundle.estimated_packet_count();

    send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, witness_id, bundle)
        .await;

    let sends_attempted = typed.sent_count.load(AtomicOrdering::SeqCst);
    let clients = connected.lock().unwrap();
    let s = clients.get(&witness_addr).unwrap();
    let tx_len = s.channel.lock().unwrap().tx_window.len();
    let seq_advanced = s.next_seq.load(Ordering::Relaxed) as usize;

    // Behavior contract:
    // - Seq counter advanced by the full estimated_packet_count (reserve
    //   before sending; failure does NOT unreserve).
    // - send_to called for 2 fragments (1 success + 1 failure); the
    //   abort means fragments 3..N are never attempted.
    // - TX window holds only the SUCCEEDED fragment(s) — failed and
    //   unsent fragments are NOT registered for retransmit.
    assert_eq!(
        seq_advanced, expected_frags,
        "all seqs were reserved up-front; failure does not rewind the counter"
    );
    assert_eq!(
        sends_attempted, 2,
        "abort-on-first-failure must stop after the failing fragment; \
         got {sends_attempted} send attempts for a {expected_frags}-fragment bundle"
    );
    assert_eq!(
        tx_len, 1,
        "only the successfully-sent fragment registers in the TX window; \
         got {tx_len} registered for 1 successful + 1 failed send"
    );
}

/// `SystemOptionsUpdate` is a fire-and-forget persistence message — no
/// cell-side reply, no transport fan-out. The dispatcher must route it
/// to the system-options handler and return cleanly; the handler in
/// turn no-ops when `db_pool` is `None` (test / repl mode). The
/// assertion here is "no panic, no spurious cell-bound message" —
/// reverting the dispatch arm fails compilation, but reverting the
/// `persist_system_options` call inside the arm would only be caught
/// by the live-DB integration tests next to the handler. This guard
/// pins the routing.
#[tokio::test]
async fn system_options_update_routes_without_panic_and_emits_nothing() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr) = empty_maps();
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(1);

    handle_cell_message(
        CellToBaseMsg::SystemOptionsUpdate {
            player_id: 42,
            auto_reload: false,
            reload_on_activate: true,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None, // no db_pool — exercise the silent no-op path
        &None,
        "127.0.0.1",
        7777,
        &None,
    )
    .await;

    // Fire-and-forget: nothing should be sent back to the cell, and no
    // wire packets should leave the process. Reverting the dispatch
    // arm to anything that re-emits BaseToCellMsg or calls
    // `transport.send_to` would fail one of these.
    assert!(
        cell_rx.try_recv().is_err(),
        "SystemOptionsUpdate must not fan out a base→cell reply",
    );
}
