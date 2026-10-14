//! Lab chaos `Delay` (CH-07) through the AoI hold: the receive path never
//! waits, the witness's AoI traffic stays buffered for the delay, and the
//! release sends introductions first, then the traffic held behind them.

use super::*;
use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::base::world_entry::cell_dispatch::lend_registry;
use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry_appearance::handle_on_client_ready;
use crate::base::PendingClientReadyInfo;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_base_session::base::lab_chaos::FlushChaos;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::parse_incoming;
use tracing::Level;

const PLAYER: u32 = 2;
const CORPSE: u32 = 100_150;
/// An NPC the client already has, which walks out of range mid-delay.
const OTHER: u32 = 100_151;
const DELAY: Duration = Duration::from_millis(1500);

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddr = Arc<Mutex<HashMap<u32, SocketAddr>>>;

fn corpse_entered() -> DeferredAoiMsg {
    DeferredAoiMsg::EnteredAoI {
        entity_id: CORPSE,
        class_id: 0,
        position: [-322.5, 73.47, -209.83],
        direction: [0.0; 3],
        level: 1,
        npc_data: None,
        player_data: None,
    }
}

const WITNESS_METHOD: u16 = 3;
fn witness_method_args() -> Vec<u8> {
    vec![0x00, 0x00, 0x40, 0x00]
}

fn session(addr: SocketAddr, state: ConnectedClientState) -> (Connected, EntityToAddr) {
    (
        Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        Arc::new(Mutex::new(HashMap::from([(PLAYER, addr)]))),
    )
}

/// A registry with `Delay` set for `PLAYER` at `addr`, lent to this thread.
fn lend_delay(addr: SocketAddr) {
    let registry: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    registry
        .set(
            addr,
            PLAYER,
            "flush:delay".into(),
            None,
            Some(FlushChaos::Delay {
                ms: DELAY.as_millis() as u32,
            }),
            60,
            std::time::Instant::now(),
        )
        .expect("a valid set");
    lend_registry(Some(registry));
}

fn hold_active(connected: &Connected, addr: SocketAddr) -> bool {
    connected.lock().unwrap()[&addr]
        .cinematic_aoi_hold
        .is_some()
}

fn buffered(connected: &Connected, addr: SocketAddr) -> usize {
    connected.lock().unwrap()[&addr].deferred_aoi_msgs.len()
}

/// Packet bodies, which carry no sequence number, so two sessions that sent
/// a different number of packets first still compare.
fn bodies(packets: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let session = MercuryEncryption::from_session_key([0u8; 32]);
    packets
        .iter()
        .map(|raw| {
            let plain = session.decrypt(raw).unwrap();
            parse_incoming(&plain).unwrap().body.to_vec()
        })
        .collect()
}

/// What the ordinary flush sends for the corpse's introduction followed by
/// a witness method about it and the other NPC's leave: the order the
/// delayed release must reproduce.
async fn reference_bodies() -> Vec<Vec<u8>> {
    lend_registry(None);
    let addr: SocketAddr = "127.0.0.1:55830".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.deferred_aoi_msgs = vec![
        corpse_entered(),
        DeferredAoiMsg::WitnessEntityMethod {
            entity_id: CORPSE,
            method_index: WITNESS_METHOD,
            args: witness_method_args(),
            entity_is_player: false,
        },
        DeferredAoiMsg::LeftAoI { entity_id: OTHER },
    ];
    let (connected, entity_to_addr) = session(addr, state);
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    flush_deferred_aoi(
        PLAYER,
        addr,
        "reference",
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    bodies(&typed.filter_to(addr))
}

async fn dispatch(
    msg: CellToBaseMsg,
    transport: &Arc<dyn Transport>,
    connected: &Connected,
    entity_to_addr: &EntityToAddr,
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

/// A returning player with `Delay` set: `onClientReady` returns without the
/// clock moving (a sleep in the flush would stall the base's receive loop
/// for every session), the player-self call goes out, and the corpse's
/// introduction stays held. AoI traffic arriving during the delay buffers
/// behind it instead of overtaking it. After the delay the buffer goes out
/// exactly as the ordinary flush would send it: phase 1, phase 2, then the
/// held traffic in arrival order. Fails on the old code (the flush slept
/// inline, so the call took the whole delay and the gate was open).
#[tokio::test(start_paused = true)]
async fn delay_holds_aoi_traffic_without_stalling_client_ready() {
    let expected = reference_bodies().await;
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55831".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    state.active_player_id = Some(42);
    state.player_name = Some("Tester".to_string());
    state.pending_client_ready = Some(PendingClientReadyInfo {
        entity_id: PLAYER,
        player_id: 42,
        world_name: "Castle_CellBlock".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0,
    });
    state.deferred_aoi_msgs.push(corpse_entered());
    state
        .deferred_aoi_msgs
        .push(DeferredAoiMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index: 0x42,
            args: vec![0x01],
        });
    let (connected, entity_to_addr) = session(addr, state);
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    lend_delay(addr);

    let start = tokio::time::Instant::now();
    handle_on_client_ready(
        addr,
        [0u8; 32],
        &connected,
        &None,
        &transport,
        &entity_to_addr,
        &None,
    )
    .await
    .unwrap();
    assert_eq!(start.elapsed(), Duration::ZERO, "onClientReady never waits");
    assert!(hold_active(&connected, addr), "the delay holds AoI traffic");
    assert!(
        matches!(
            connected.lock().unwrap()[&addr]
                .deferred_aoi_msgs
                .as_slice(),
            [DeferredAoiMsg::EnteredAoI {
                entity_id: CORPSE,
                ..
            }]
        ),
        "the introduction stays held; the player-self call has gone"
    );
    let delayed = capture
        .find_message(Level::INFO, "lab chaos: deferred-AoI flush held back")
        .expect("the delay is logged");
    assert!(delayed.has_field("event", "flush_delayed"), "{delayed:#?}");
    assert!(delayed.has_field("delay_ms", "1500"), "{delayed:#?}");
    assert!(delayed.has_field("buffered_introductions", "1"));
    let sent_at_ready = typed.len();

    // Live traffic during the delay: a method about the held corpse, and
    // another NPC leaving. Both must wait behind the introduction.
    dispatch(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id: PLAYER,
            entity_id: CORPSE,
            method_index: WITNESS_METHOD,
            args: witness_method_args(),
            entity_is_player: false,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    dispatch(
        CellToBaseMsg::LeftAoI {
            witness_id: PLAYER,
            entity_id: OTHER,
        },
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert_eq!(buffered(&connected, addr), 3, "live traffic buffered");
    assert_eq!(typed.len(), sent_at_ready, "nothing overtakes the hold");

    tokio::time::sleep(DELAY - Duration::from_millis(50)).await;
    assert!(hold_active(&connected, addr), "held for the whole delay");
    assert_eq!(typed.len(), sent_at_ready);

    tokio::time::sleep(Duration::from_millis(100)).await;
    lend_registry(None);
    assert!(!hold_active(&connected, addr), "the timer lifts the hold");
    assert_eq!(buffered(&connected, addr), 0);
    let released = typed.filter_to(addr);
    assert_eq!(bodies(&released[sent_at_ready..]), expected);
    let lifted = capture
        .find_message(Level::INFO, "Cinematic AoI hold: released")
        .expect("the release is logged");
    assert!(lifted.has_field("reason", "client_ready"), "{lifted:#?}");
    assert!(lifted.has_field("flushed", "3"), "{lifted:#?}");
}

/// A first-login hold with `Delay` set: the release `cancelMovie` claims
/// returns at once and keeps the hold up for the delay, a second release
/// is refused (the hold is claimed), and the drain after the timer sends the
/// introduction without waiting again.
#[tokio::test(start_paused = true)]
async fn cancel_movie_with_delay_returns_at_once_and_delays_once() {
    let addr: SocketAddr = "127.0.0.1:55832".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(PLAYER);
    begin(&mut state);
    state.deferred_aoi_msgs.push(corpse_entered());
    let (connected, entity_to_addr) = session(addr, state);
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    lend_delay(addr);

    let start = tokio::time::Instant::now();
    release_on_cancel(PLAYER, addr, &transport, &connected, &entity_to_addr).await;
    assert_eq!(start.elapsed(), Duration::ZERO, "cancelMovie never waits");
    assert!(hold_active(&connected, addr));
    assert!(typed.is_empty());

    release_on_cancel(PLAYER, addr, &transport, &connected, &entity_to_addr).await;
    assert!(
        hold_active(&connected, addr),
        "a claimed hold is left alone"
    );

    // Just past one delay the introduction is out: the drain's own flush
    // did not wait a second time with the chaos still set.
    tokio::time::sleep(DELAY + Duration::from_millis(1)).await;
    lend_registry(None);
    assert!(!hold_active(&connected, addr));
    assert_eq!(
        typed.len(),
        2,
        "phase-1 and phase-2 bundles after one delay"
    );
}
