//! World-entry latch tests for the auth → base → cell handoff seam.
//!
//! `world_entry_sent` is a one-shot latch on ConnectedClientState — it
//! prevents duplicate RESET_ENTITIES dispatches when the client retries
//! `playCharacter`. Cases covered:
//!   * already latched → second call is a no-op (no overwrite of
//!     `pending_player_entity_id`).
//!   * no-DB success path → CombatSim default routes through, latch
//!     flips, world_name + active_player_id get cached.
//!   * unknown addr (ConnectedClientState gone) → no panic, no latch
//!     change.
//!   * a session torn down or replaced mid-entry: nothing of the entry
//!     reaches the replacing session, and the entry's entity is removed.
//!   * the NO_ENTITY_ID bail on a replaced session leaves the replacing
//!     session's latch alone (a database that drops the connection).
//!
//! The DB-success branch needs live-DB scaffolding that doesn't
//! exist yet for this module; filing as a follow-up rather than
//! claiming coverage that isn't there.

use super::test_fakes::{cell_rows, recording_cell, CellLog, FailingTransport};
use super::*;
use crate::test_support::TestTransport;
use cimmeria_common::EntityId;
use cimmeria_mercury::encryption::MercuryEncryption;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::time::Instant;

fn make_connected_state(account_id: u32) -> ConnectedClientState {
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key([0xCDu8; 32]),
        key: [0xCDu8; 32],
        enc_version: cimmeria_mercury::encryption::EncryptionVersion::V1,
        account_id,
        account_name: Some("testacct".into()),
        access_level: 0,
        session_kind: None,
        dnd_message: None,
        afk_message: None,
        ignore: Default::default(),
        char_list_sent: false,
        world_entry_sent: false,
        pending_player_entity_id: None,
        player_entity_id: None,
        next_seq: Arc::new(AtomicU32::new(3)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
        connected_at: Instant::now(),
        account_entity_id: 1,
        next_data_id: 0,
        pending_world_entry: None,
        pending_player_load_data: None,
        pending_map_loaded: None,
        pending_client_ready: None,
        deferred_aoi_msgs: Vec::new(),
        cached_appearance_args: None,
        cached_tint_args: None,
        weapon_holstered: true,
        cancelled: Arc::new(AtomicBool::new(false)),
        cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
        cinematic_aoi_hold: None,
        listed_online: false,
        rate_limits: Default::default(),
        org_invites: Default::default(),
        player_name: None,
        player_level: None,
        player_archetype: None,
        player_class_id: None,
        player_alignment: None,
        world_name: None,
        player_xp: None,
        player_training_points: None,
        active_player_id: None,
        pending_destination_ring_id: None,
        channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
            "127.0.0.1:9999".parse().unwrap(),
        )),
        extensions: Default::default(),
        plugins: Default::default(),
    }
}

fn make_transport() -> Arc<dyn Transport> {
    Arc::new(TestTransport::new())
}

/// Second `playCharacter` on an already-latched session must be a
/// no-op — no UDP send, no state change. Without this, the client
/// could trigger redundant entity teardowns mid-world-entry.
#[tokio::test]
async fn play_character_is_idempotent_when_world_entry_already_sent() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55600".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let mut state = make_connected_state(0xAABB);
    // Pre-latch: world_entry_sent is already true. Also seed an
    // entity_id so we can detect any unwanted overwrite.
    state.world_entry_sent = true;
    state.player_entity_id = Some(0x77_88_99_AA);
    state.active_player_id = Some(42);
    connected.lock().unwrap().insert(addr, state);

    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    handle_play_character(
        &transport,
        addr,
        [0xCDu8; 32],
        0xAABB,
        42,
        &connected,
        &None,
        &entity_manager,
        &cell_tx,
    )
    .await
    .expect("idempotent path returns Ok");

    // Latch unchanged, prior entity_id intact (no second-call clobber).
    let map = connected.lock().unwrap();
    let c = map.get(&addr).unwrap();
    assert!(
        c.world_entry_sent,
        "latch must remain true on idempotent re-call"
    );
    assert_eq!(
        c.player_entity_id,
        Some(0x77_88_99_AA),
        "idempotent path must NOT overwrite the player_entity_id from the first call"
    );
}

/// No-DB CombatSim path: `query_world_entry` allocates a real
/// entity_id and returns a default entry (CombatSim is the
/// smoke-test world). play_character must then populate
/// ConnectedClientState with the allocated entity_id and the
/// "CombatSim" world name. Pins the no-DB code path used by the
/// orchestrator's bring-up smoke tests when no Postgres is reachable.
#[tokio::test]
async fn play_character_no_db_routes_to_combatsim_and_populates_state() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55601".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::new()));
    connected
        .lock()
        .unwrap()
        .insert(addr, make_connected_state(0xCCDD));

    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
    const PLAYER_ID: i32 = 7;

    handle_play_character(
        &transport,
        addr,
        [0xCDu8; 32],
        0xCCDD,
        PLAYER_ID,
        &connected,
        &None, // no DB → CombatSim default entry (NOT a failure)
        &entity_manager,
        &cell_tx,
    )
    .await
    .expect("no-DB CombatSim path returns Ok");

    let map = connected.lock().unwrap();
    let c = map.get(&addr).unwrap();
    // Latch is set — first playCharacter on this connection.
    assert!(c.world_entry_sent, "latch flips on first call");
    // CombatSim default world is what query_world_entry's no-DB
    // branch hard-codes; pin it so a future smoke-test refactor
    // can't silently change which world the no-DB path maps to.
    assert_eq!(
        c.world_name.as_deref(),
        Some("CombatSim"),
        "no-DB path must route to CombatSim"
    );
    // active_player_id was cached from the call argument so
    // gate-travel / respawn target the right character on
    // multi-character accounts.
    assert_eq!(c.active_player_id, Some(PLAYER_ID));
    // Entity id was allocated by EntityManager — must be non-zero
    // (zero is the NO_ENTITY_ID sentinel for the failure paths).
    assert!(
        c.player_entity_id.unwrap_or(0) != 0,
        "no-DB success path allocates a real entity_id, not the NO_ENTITY_ID sentinel"
    );
}

/// Unknown addr (ConnectedClientState absent) is the racy case where
/// the client disconnected between sending playCharacter and base
/// processing it. Handler must NOT panic and must NOT create a
/// stub ConnectedClientState — the connection lifecycle is the only
/// gate that produces those rows.
#[tokio::test]
async fn play_character_for_unknown_addr_is_silent_no_op() {
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55602".parse().unwrap();
    // connected is empty — addr is genuinely unknown.
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    let result = handle_play_character(
        &transport,
        addr,
        [0xCDu8; 32],
        0xEEFF,
        1,
        &connected,
        &None,
        &entity_manager,
        &cell_tx,
    )
    .await;
    assert!(
        result.is_ok(),
        "unknown-addr path must not propagate an error"
    );
    assert!(
        connected.lock().unwrap().is_empty(),
        "unknown addr must NOT cause an entry to be created"
    );
}

/// The fresh session a takeover left at `addr` carries none of the late
/// handler's world entry.
fn assert_untouched(connected: &super::test_fakes::Connected, addr: SocketAddr) {
    let map = connected.lock().unwrap();
    let c = map.get(&addr).expect("the replacing session");
    assert!(!c.world_entry_sent, "the replacing session's own latch");
    assert_eq!(c.player_entity_id, None);
    assert_eq!(c.pending_player_entity_id, None);
    assert_eq!(c.world_name, None);
    assert_eq!(c.active_player_id, None);
    assert!(c.pending_world_entry.is_none());
    assert!(c.pending_player_load_data.is_none());
}

/// The seq the late handler's RESET_ENTITIES goes out under:
/// `make_connected_state` starts the old session's counter at 3.
const OLD_SESSION_RESET_SEQ: u32 = 3;

/// A manager holding one live entity, standing for another session's
/// player, and that entity's id.
fn manager_with_another_player() -> (Arc<Mutex<EntityManager>>, EntityId) {
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let other = entity_manager.lock().unwrap().create_entity("SGWPlayer");
    (entity_manager, other)
}

/// The entry's entity is gone from the manager and the cell got a
/// `DestroyEntity` for it, under the id it was created with, and nothing
/// else (no logout `DisconnectEntity`). `other` stays.
async fn assert_entry_entity_removed(
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell: &CellLog,
    other: EntityId,
) {
    let rows = cell_rows(cell, 2).await;
    let &[("create", created), ("destroy", destroyed)] = rows.as_slice() else {
        panic!("expected the entry's create, then its destroy: {rows:?}");
    };
    assert_eq!(destroyed, created, "the entry's own entity");
    assert_ne!(EntityId(created as i32), other);
    let manager = entity_manager.lock().unwrap();
    assert!(
        manager.get_entity(EntityId(created as i32)).is_none(),
        "the entry's entity is freed"
    );
    assert!(
        manager.get_entity(other).is_some(),
        "another player's stays"
    );
    assert_eq!(manager.entity_count(), 1);
}

/// A relaunch takeover while `query_world_entry` waits on the cell: the late
/// handler sends no RESET_ENTITIES (under the old session's key) and writes
/// none of the old character's state into the session that replaced it, and
/// the entity it created goes, from the cell and the manager. Fails if the
/// owner check before the send is removed (the reset goes out), or if the
/// abandoned entry keeps its entity.
#[tokio::test]
async fn play_character_leaves_a_session_replaced_during_the_cell_wait() {
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let addr: SocketAddr = "127.0.0.1:55603".parse().unwrap();
    let connected: super::test_fakes::Connected = Arc::new(Mutex::new(HashMap::new()));
    connected
        .lock()
        .unwrap()
        .insert(addr, make_connected_state(0xABCD));
    let (cell_tx, cell) = super::test_fakes::cell_replacing_the_session(&connected, addr, 0xABCD);
    let (entity_manager, other) = manager_with_another_player();

    handle_play_character(
        &transport,
        addr,
        [0xCDu8; 32],
        0xABCD,
        9,
        &connected,
        &None,
        &entity_manager,
        &Some(cell_tx),
    )
    .await
    .expect("an abandoned entry is not an error");

    assert_eq!(typed.len(), 0, "no RESET_ENTITIES to the replacing session");
    assert_untouched(&connected, addr);
    assert_entry_entity_removed(&entity_manager, &cell, other).await;
}

/// A takeover during the RESET_ENTITIES send: the store that follows is
/// skipped, the reset is not registered in the replacing session's reliable
/// channel, and the entry's entity goes. Fails if the owner check before the
/// store is removed (the old character's entity, world and ids land on the
/// new session), if the reliable registration moves back before that check,
/// or if the abandoned entry keeps its entity.
#[tokio::test]
async fn play_character_leaves_a_session_replaced_during_the_send() {
    let addr: SocketAddr = "127.0.0.1:55604".parse().unwrap();
    let connected: super::test_fakes::Connected = Arc::new(Mutex::new(HashMap::new()));
    connected
        .lock()
        .unwrap()
        .insert(addr, make_connected_state(0xABCE));
    let transport: Arc<dyn Transport> = Arc::new(super::test_fakes::ReplacingTransport {
        connected: Arc::clone(&connected),
        addr,
        account_id: 0xABCE,
    });
    let (cell_tx, cell) = recording_cell();
    let (entity_manager, other) = manager_with_another_player();

    handle_play_character(
        &transport,
        addr,
        [0xCDu8; 32],
        0xABCE,
        9,
        &connected,
        &None,
        &entity_manager,
        &Some(cell_tx),
    )
    .await
    .expect("an abandoned entry is not an error");

    assert_untouched(&connected, addr);
    {
        let map = connected.lock().unwrap();
        let channel = map[&addr].channel.lock().unwrap();
        let registered: Vec<u32> = channel
            .tx_window
            .iter()
            .chain(channel.unsent_packets.iter())
            .map(|entry| entry.packet.sequence)
            .collect();
        assert!(
            !registered.contains(&OLD_SESSION_RESET_SEQ),
            "the old entry's RESET_ENTITIES is in the replacing session's channel: {registered:?}"
        );
        assert!(registered.is_empty(), "{registered:?}");
    }
    assert_entry_entity_removed(&entity_manager, &cell, other).await;
}

/// The RESET_ENTITIES send fails: the handler errors before the store, so
/// no session holds the entity, and it goes like an abandoned entry's. Fails
/// if only the abandon paths remove it.
#[tokio::test]
async fn play_character_that_fails_to_send_removes_its_entity() {
    let addr: SocketAddr = "127.0.0.1:55605".parse().unwrap();
    let connected: super::test_fakes::Connected = Arc::new(Mutex::new(HashMap::new()));
    connected
        .lock()
        .unwrap()
        .insert(addr, make_connected_state(0xABCF));
    let transport: Arc<dyn Transport> = Arc::new(FailingTransport);
    let (cell_tx, cell) = recording_cell();
    let (entity_manager, other) = manager_with_another_player();

    let result = handle_play_character(
        &transport,
        addr,
        [0xCDu8; 32],
        0xABCF,
        9,
        &connected,
        &None,
        &entity_manager,
        &Some(cell_tx),
    )
    .await;

    assert!(result.is_err(), "the send failure reaches the caller");
    assert_eq!(
        connected.lock().unwrap()[&addr].player_entity_id,
        None,
        "the session never took the entity"
    );
    assert_entry_entity_removed(&entity_manager, &cell, other).await;
}

/// The `NO_ENTITY_ID` bail on a session replaced during the database wait:
/// the replacing session latched its own entry, and the late bail must not
/// reopen it. The fake database replaces the session when the entry
/// connects, then drops the connection, so the replacement comes before the
/// query fails. Fails if the bail resets the latch by address alone.
#[tokio::test]
async fn a_failed_entry_leaves_the_latch_of_the_session_that_replaced_it() {
    let addr: SocketAddr = "127.0.0.1:55606".parse().unwrap();
    let connected: super::test_fakes::Connected = Arc::new(Mutex::new(HashMap::new()));
    connected
        .lock()
        .unwrap()
        .insert(addr, make_connected_state(0xABD0));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let replaced = Arc::new(AtomicBool::new(false));
    {
        let connected = Arc::clone(&connected);
        let replaced = Arc::clone(&replaced);
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                if !replaced.swap(true, Ordering::SeqCst) {
                    let mut fresh = crate::test_support::test_default_connected_client_state();
                    fresh.account_id = 0xABD0;
                    fresh.world_entry_sent = true;
                    connected.lock().unwrap().insert(addr, fresh);
                }
                drop(socket);
            }
        });
    }
    let options = sqlx::postgres::PgConnectOptions::new()
        .host("127.0.0.1")
        .port(port)
        .username("nobody")
        .database("none")
        .ssl_mode(sqlx::postgres::PgSslMode::Disable);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_lazy_with(options);
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));

    handle_play_character(
        &make_transport(),
        addr,
        [0xCDu8; 32],
        0xABD0,
        9,
        &connected,
        &Some(Arc::new(pool)),
        &entity_manager,
        &None,
    )
    .await
    .expect("the bail is not an error");

    assert!(
        replaced.load(Ordering::SeqCst),
        "the session was replaced inside the entry's database wait"
    );
    assert!(
        connected.lock().unwrap()[&addr].world_entry_sent,
        "the replacing session's latch stays set"
    );
    assert_eq!(entity_manager.lock().unwrap().entity_count(), 0);
}
