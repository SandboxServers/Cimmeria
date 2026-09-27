//! SS-00: the online name index across world entry and reanchor.
//!
//! World entry lists a character at `onClientReady`, not at
//! `play_character`. Reanchor keeps the session, its name and its
//! `player_id`, so the listing must survive it, exactly once. The session-removing teardowns are guarded in
//! `cimmeria-base-session` (`player_index::tests`), logoff in `cimmeria-base`
//! and the gate-travel abandon in `gate_travel::tests`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::player_index::{
    lookup_online, NameLookup, OnlinePlayer, OnlinePlayerIndex,
};
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::handle_on_client_ready;
use super::play_character::handle_play_character;
use super::reanchor_player::handle_reanchor_player;
use crate::base::PendingClientReadyInfo;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

/// World entry lists the character at `onClientReady`, not at
/// `playCharacter`: until the client has created its player entity a tell or
/// duel challenge must not find it (Copilot on #880). Regression shapes:
/// the listing moved back to `play_character` (the pre-ready assertions
/// fail), dropped from `handle_on_client_ready` (the post-ready ones fail),
/// or a gate-travel re-run of `onClientReady` logging a second insert.
#[tokio::test]
async fn play_character_lists_the_character_in_the_online_index() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let addr: SocketAddr = "127.0.0.1:55680".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
    const PLAYER_ID: i32 = 7;
    let capture = LogCapture::install();
    let inserts = || {
        capture
            .all()
            .into_iter()
            .filter(|c| c.target == "online_index" && c.has_field("event", "online_index.insert"))
            .collect::<Vec<_>>()
    };

    handle_play_character(
        &transport,
        addr,
        [0u8; 32],
        0xCCDD,
        PLAYER_ID,
        &connected,
        &None,
        &entity_manager,
        &cell_tx,
    )
    .await
    .expect("no-DB world entry returns Ok");

    let name = connected.lock().unwrap()[&addr]
        .player_name
        .clone()
        .expect("world entry names the session");
    assert_eq!(
        lookup_online(&connected, &name),
        NameLookup::NotFound,
        "not listed before the client is ready"
    );
    assert!(inserts().is_empty(), "no insert before client-ready");

    // mapLoaded stages the finalization; onClientReady consumes it.
    let stage_client_ready = || {
        let mut clients = connected.lock().unwrap();
        let c = clients.get_mut(&addr).unwrap();
        c.pending_client_ready = Some(PendingClientReadyInfo {
            entity_id: c.player_entity_id.expect("world entry sets the entity id"),
            player_id: PLAYER_ID,
            world_name: "CombatSim".to_string(),
            appearance_args: Vec::new(),
            tint_args: Vec::new(),
            first_login: 0,
        });
    };
    let client_ready = || {
        handle_on_client_ready(
            addr,
            [0u8; 32],
            &connected,
            &cell_tx,
            &transport,
            &entity_to_addr,
            &None,
        )
    };
    stage_client_ready();
    client_ready().await.expect("no-DB client-ready returns Ok");

    assert_eq!(
        lookup_online(&connected, &name),
        NameLookup::Found(OnlinePlayer {
            addr,
            player_id: PLAYER_ID
        })
    );
    assert_eq!(
        lookup_online(&connected, &name.to_uppercase()),
        NameLookup::Found(OnlinePlayer {
            addr,
            player_id: PLAYER_ID
        }),
        "and by case fold"
    );
    // Telemetry: one DEBUG `online_index.insert` with the ids and the path.
    let inserted = inserts();
    assert_eq!(inserted.len(), 1);
    assert!(inserted[0].has_field("path", "world_entry"));
    assert!(inserted[0].has_field("player_id", "7"));
    let account_id = connected.lock().unwrap()[&addr].account_id;
    assert!(inserted[0].has_field("account_id", &account_id.to_string()));

    // Gate travel re-runs mapLoaded / onClientReady for a character that is
    // still listed: still one listing, and no second insert event.
    stage_client_ready();
    client_ready()
        .await
        .expect("second client-ready returns Ok");
    assert_eq!(
        OnlinePlayerIndex::new(&connected.lock().unwrap())
            .entries()
            .count(),
        1
    );
    assert_eq!(
        inserts().len(),
        1,
        "an already-listed character is not re-inserted"
    );
}

/// A reanchor (respawn) re-creates the client's pawn on the same session:
/// the character stays listed, once, at the same address and `player_id`.
#[tokio::test]
async fn reanchor_keeps_exactly_one_listing() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let addr: SocketAddr = "127.0.0.1:55681".parse().unwrap();
    let entity_id = 4242u32;
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.player_name = Some("Lomiada".to_string());
    state.active_player_id = Some(7);
    state.listed_online = true;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));

    handle_reanchor_player(
        entity_id,
        0x0001_0042,
        [1.0, 2.0, 3.0],
        [0.0; 3],
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await
    .expect("reanchor of a connected player succeeds");

    assert_eq!(
        lookup_online(&connected, "Lomiada"),
        NameLookup::Found(OnlinePlayer { addr, player_id: 7 })
    );
    let clients = connected.lock().unwrap();
    assert_eq!(OnlinePlayerIndex::new(&clients).entries().count(), 1);
}
