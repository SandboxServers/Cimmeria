//! CH-14 (D-CH10): chaos armed for an account reaches the first flush of
//! that account's next world entry. `playCharacter` promotes the arm, tagged
//! with the entity it just created, so the `onClientReady` that follows
//! reads it, and the next world entry finds nothing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_base_session::base::lab_chaos::FlushChaos;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::cell_dispatch::{flush_delay, lend_registry};
use super::handle_on_client_ready;
use super::play_character::handle_play_character;
use crate::base::{ConnectedClientState, PendingClientReadyInfo};
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, TestTransport};

const ACCOUNT: u32 = 0x4242;
const DELAY_MS: u32 = 1500;

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

/// A registry with a `Delay` armed for [`ACCOUNT`], lent to this thread.
fn lend_armed_delay() -> &'static LabChaos {
    let registry: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    registry
        .arm(
            ACCOUNT,
            "flush:delay".into(),
            None,
            Some(FlushChaos::Delay { ms: DELAY_MS }),
            60,
            Instant::now(),
        )
        .expect("a valid arm");
    lend_registry(Some(registry));
    registry
}

async fn play(connected: &Connected, addr: SocketAddr) -> u32 {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
    handle_play_character(
        &transport,
        addr,
        [0u8; 32],
        ACCOUNT,
        7,
        connected,
        &None,
        &entity_manager,
        &cell_tx,
    )
    .await
    .expect("the no-DB world entry succeeds");
    connected.lock().unwrap()[&addr]
        .player_entity_id
        .expect("playCharacter set the player entity")
}

/// Arm, then a world entry on a session that had no chaos: `playCharacter`
/// puts the arm on the session under the new entity, and `onClientReady`,
/// the first flush, holds the AoI traffic for the armed delay. Fails if the
/// promotion is removed or moved after the flush (no hold), or tags another
/// entity (the flush's stale-entity check drops it).
#[tokio::test(start_paused = true)]
async fn armed_delay_reaches_the_first_flush_of_the_next_world_entry() {
    let addr: SocketAddr = "127.0.0.1:55690".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT;
    let connected: Connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let registry = lend_armed_delay();

    let entity_id = play(&connected, addr).await;
    let promoted = registry.status(Instant::now());
    assert_eq!(promoted.len(), 1, "promoted at playCharacter");
    assert_eq!(promoted[0].entity_id, entity_id);
    assert!(registry.armed_status(Instant::now()).is_empty());

    // What `enableEntities` leaves for `onClientReady` on a returning
    // character.
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .pending_client_ready = Some(PendingClientReadyInfo {
        entity_id,
        player_id: 7,
        world_name: "CombatSim".to_string(),
        appearance_args: vec![0xAB],
        tint_args: vec![0xCD],
        first_login: 0,
    });
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
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
    let held = connected.lock().unwrap()[&addr]
        .cinematic_aoi_hold
        .is_some();
    lend_registry(None);
    assert!(held, "the armed delay holds the first flush");
}

/// One-shot: after a return to character select (which clears the
/// session's chaos), the account's next world entry is clean. A brand-new
/// character's first login takes the same `playCharacter` path, so the
/// first entry's read is checked here directly as the flush makes it.
#[tokio::test]
async fn an_arm_reaches_one_world_entry_only() {
    let addr: SocketAddr = "127.0.0.1:55691".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT;
    let connected: Connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let registry = lend_armed_delay();

    let first = play(&connected, addr).await;
    assert_eq!(
        flush_delay(addr, first),
        Some(Duration::from_millis(u64::from(DELAY_MS)))
    );

    // Character select: the session stays, its chaos goes, and the entry
    // latch reopens for the next `playCharacter`.
    registry.clear(Some(addr));
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .world_entry_sent = false;
    let second = play(&connected, addr).await;
    let read = flush_delay(addr, second);
    lend_registry(None);
    assert_eq!(read, None, "the arm was spent on the first entry");
}
