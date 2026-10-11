//! CH-14 (D-CH10): chaos armed for an account reaches the first flush of
//! that account's next world entry. `playCharacter` promotes the arm, tagged
//! with the entity it just created, so the `onClientReady` that follows
//! reads it, and the next world entry finds nothing. A session replaced
//! during the entry gets nothing, and an entry that fails after promoting
//! gives the arm back.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_base_session::base::lab_chaos::FlushChaos;
use cimmeria_base_session::base::lab_chaos::LabChaos;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use super::super::cell_dispatch::{flush_delay, lend_registry};
use super::super::handle_on_client_ready;
use super::chaos::promote_if_owner;
use super::handle_play_character;
use super::test_fakes::{
    cell_replacing_the_session, Connected, FailingTransport, ReplacingTransport,
};
use crate::base::deferred_aoi::DeferredAoiMsg;
use crate::base::PendingClientReadyInfo;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

const ACCOUNT: u32 = 0x4242;
const DELAY_MS: u32 = 1500;

/// A registry with `flush` armed for [`ACCOUNT`], lent to this thread, and
/// the arm's expiry.
fn lend_armed(flush: FlushChaos) -> (&'static LabChaos, Instant) {
    let registry: &'static LabChaos = Box::leak(Box::new(LabChaos::new()));
    let expires_at = registry
        .arm(
            ACCOUNT,
            "flush test".into(),
            None,
            Some(flush),
            60,
            Instant::now(),
        )
        .expect("a valid arm");
    lend_registry(Some(registry));
    (registry, expires_at)
}

/// A registry with a `Delay` armed for [`ACCOUNT`], lent to this thread.
fn lend_armed_delay() -> &'static LabChaos {
    lend_armed(FlushChaos::Delay { ms: DELAY_MS }).0
}

fn session(addr: SocketAddr) -> Connected {
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT;
    Arc::new(Mutex::new(HashMap::from([(addr, state)])))
}

/// `playCharacter` for [`ACCOUNT`] through `transport`, `db_pool` and
/// `cell_tx`.
async fn play_with(
    connected: &Connected,
    addr: SocketAddr,
    transport: Arc<dyn Transport>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    handle_play_character(
        &transport,
        addr,
        [0u8; 32],
        ACCOUNT,
        7,
        connected,
        db_pool,
        &entity_manager,
        cell_tx,
    )
    .await
}

async fn play(connected: &Connected, addr: SocketAddr) -> u32 {
    play_with(
        connected,
        addr,
        Arc::new(TestTransport::new()),
        &None,
        &None,
    )
    .await
    .expect("the no-DB world entry succeeds");
    connected.lock().unwrap()[&addr]
        .player_entity_id
        .expect("playCharacter set the player entity")
}

/// The arm is pending again, as it was armed.
fn assert_arm_pending(registry: &LabChaos, flush: FlushChaos, expires_at: Instant) {
    let now = Instant::now();
    assert!(registry.status(now).is_empty(), "no session entry");
    let armed = registry.armed_status(now);
    assert_eq!(armed.len(), 1, "the arm is still the account's");
    assert_eq!(armed[0].flush, Some(flush));
    assert_eq!(armed[0].expires_at, expires_at);
}

/// Arm, then a world entry on a session that had no chaos: `playCharacter`
/// puts the arm on the session under the new entity, and `onClientReady`,
/// the first flush, holds the AoI traffic for the armed delay. Fails if the
/// promotion is removed or moved after the flush (no hold), or tags another
/// entity (the flush's stale-entity check drops it).
#[tokio::test(start_paused = true)]
async fn armed_delay_reaches_the_first_flush_of_the_next_world_entry() {
    let addr: SocketAddr = "127.0.0.1:55690".parse().unwrap();
    let connected = session(addr);
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

/// A brand-new character's first login (the #1341 case CH-11 reproduces):
/// the armed `Pack` is on the session after `playCharacter`, and the first
/// flush, held for the intro movie and released on its timer, is packed.
/// Fails if the promotion misses the first-login path.
#[tokio::test(start_paused = true)]
async fn armed_pack_reaches_a_first_login_flush() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:55692".parse().unwrap();
    let connected = session(addr);
    let (registry, _) = lend_armed(FlushChaos::Pack { records: 3 });

    let entity_id = play(&connected, addr).await;
    assert_eq!(
        registry.flush_for(addr, Instant::now()),
        Some(FlushChaos::Pack { records: 3 })
    );
    {
        let mut clients = connected.lock().unwrap();
        let c = clients.get_mut(&addr).unwrap();
        c.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id,
            player_id: 7,
            world_name: "Castle_CellBlock".to_string(),
            appearance_args: vec![0xAB],
            tint_args: vec![0xCD],
            first_login: 1,
        });
        c.deferred_aoi_msgs = (0..3)
            .map(|i| DeferredAoiMsg::EnteredAoI {
                entity_id: 100_200 + i,
                class_id: 1,
                position: [i as f32, 0.0, 0.0],
                direction: [0.0; 3],
                level: 1,
                npc_data: None,
                player_data: None,
            })
            .collect();
    }
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
    // Past the movie hold, so its timer releases the flush.
    tokio::time::sleep(Duration::from_secs(60)).await;
    lend_registry(None);
    let packed = capture
        .find_message(
            Level::INFO,
            "lab chaos: first phase-1 records packed into one packet",
        )
        .expect("the first-login flush was packed");
    assert!(
        packed.has_field("witness_id", &entity_id.to_string()),
        "{packed:#?}"
    );
    assert!(packed.has_field("records", "3"), "{packed:#?}");
}

/// One-shot: after a return to character select (which clears the
/// session's chaos), the account's next world entry is clean.
#[tokio::test]
async fn an_arm_reaches_one_world_entry_only() {
    let addr: SocketAddr = "127.0.0.1:55691".parse().unwrap();
    let connected = session(addr);
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

/// A relaunch takeover while `query_world_entry` waits on the cell: the
/// session at the address is no longer this entry's, so nothing is promoted
/// onto it and the arm waits for the account's next world entry. Fails if
/// `promote_if_owner` promotes by address alone.
#[tokio::test]
async fn a_session_replaced_during_the_entry_gets_no_chaos() {
    let addr: SocketAddr = "127.0.0.1:55693".parse().unwrap();
    let connected = session(addr);
    let flush = FlushChaos::Pack { records: 5 };
    let (registry, expires_at) = lend_armed(flush.clone());
    let cell_tx = Some(cell_replacing_the_session(&connected, addr, ACCOUNT));

    play_with(
        &connected,
        addr,
        Arc::new(TestTransport::new()),
        &None,
        &cell_tx,
    )
    .await
    .expect("an abandoned entry is not an error");
    lend_registry(None);
    assert_eq!(registry.flush_for(addr, Instant::now()), None);
    assert_arm_pending(registry, flush, expires_at);
}

/// A takeover after the promotion (during the RESET_ENTITIES send): the
/// entry is abandoned, so the promoted entry goes and the arm comes back.
#[tokio::test]
async fn a_session_replaced_after_the_promotion_gives_the_arm_back() {
    let addr: SocketAddr = "127.0.0.1:55694".parse().unwrap();
    let connected = session(addr);
    let flush = FlushChaos::Pack { records: 6 };
    let (registry, expires_at) = lend_armed(flush.clone());
    let transport = Arc::new(ReplacingTransport {
        connected: Arc::clone(&connected),
        addr,
        account_id: ACCOUNT,
    });

    play_with(&connected, addr, transport, &None, &None)
        .await
        .expect("an abandoned entry is not an error");
    lend_registry(None);
    assert_arm_pending(registry, flush, expires_at);
}

/// The RESET_ENTITIES send fails after the promotion: the handler errors,
/// the promoted entry goes and the arm is back with its knobs and expiry,
/// for the account's next real login. Fails if the handler keeps the spent
/// arm on the dead entry.
#[tokio::test]
async fn a_failed_entry_after_the_promotion_gives_the_arm_back() {
    let addr: SocketAddr = "127.0.0.1:55695".parse().unwrap();
    let connected = session(addr);
    let flush = FlushChaos::Pack { records: 7 };
    let (registry, expires_at) = lend_armed(flush.clone());

    let result = play_with(&connected, addr, Arc::new(FailingTransport), &None, &None).await;
    lend_registry(None);
    assert!(result.is_err(), "the send failure reaches the caller");
    assert_arm_pending(registry, flush, expires_at);
}

/// The `NO_ENTITY_ID` bail (the character row did not load) comes before
/// the promotion: the arm stays pending. Nothing listens on port 1, so the
/// world-entry query fails fast.
#[tokio::test]
async fn a_world_entry_that_never_loads_leaves_the_arm() {
    let addr: SocketAddr = "127.0.0.1:55696".parse().unwrap();
    let connected = session(addr);
    let flush = FlushChaos::Pack { records: 8 };
    let (registry, expires_at) = lend_armed(flush.clone());
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");

    play_with(
        &connected,
        addr,
        Arc::new(TestTransport::new()),
        &Some(Arc::new(unreachable)),
        &None,
    )
    .await
    .expect("the bail is not an error");
    lend_registry(None);
    assert!(
        !connected.lock().unwrap()[&addr].world_entry_sent,
        "the bail reopened the latch, so the entry did fail"
    );
    assert_arm_pending(registry, flush, expires_at);
}

/// The owner check itself, with nothing after it to undo a wrong promotion:
/// the session the entry latched on was replaced by a fresh one (a new
/// owner token), so `promote_if_owner` sets nothing and the arm stays. Fails
/// if it promotes by address alone.
#[tokio::test]
async fn promote_if_owner_skips_a_replaced_session() {
    let addr: SocketAddr = "127.0.0.1:55697".parse().unwrap();
    let connected = session(addr);
    let (registry, expires_at) = lend_armed(FlushChaos::Pack { records: 9 });
    let owner = Arc::clone(&connected.lock().unwrap()[&addr].cancelled);
    connected
        .lock()
        .unwrap()
        .insert(addr, test_default_connected_client_state());

    let promoted = promote_if_owner(&connected, addr, &owner, ACCOUNT, 77);
    let fresh_owner = Arc::clone(&connected.lock().unwrap()[&addr].cancelled);
    let own = promote_if_owner(&connected, addr, &fresh_owner, ACCOUNT, 78);
    lend_registry(None);
    assert!(promoted.is_none(), "not onto the session that replaced it");
    assert!(own.is_some(), "the session's own entry still promotes");
    let status = registry.status(Instant::now());
    assert_eq!(status.len(), 1);
    assert_eq!(status[0].entity_id, 78);
    assert_eq!(status[0].expires_at, expires_at);
}
