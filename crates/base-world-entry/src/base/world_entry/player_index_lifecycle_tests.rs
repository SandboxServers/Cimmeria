//! SS-00: the online name index across world entry and reanchor.
//!
//! World entry (`play_character`) is what lists a character; reanchor keeps
//! the session, its name and its `player_id`, so the listing must survive
//! it, exactly once. The session-removing teardowns are guarded in
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

use super::play_character::handle_play_character;
use super::reanchor_player::handle_reanchor_player;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, TestTransport};

/// `playCharacter` (no-DB path) lists the character under its name.
/// Regression shape: drop `listed_online = true` from the world-entry store
/// block and every tell or duel challenge finds nobody online.
#[tokio::test]
async fn play_character_lists_the_character_in_the_online_index() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let addr: SocketAddr = "127.0.0.1:55680".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
    const PLAYER_ID: i32 = 7;

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
