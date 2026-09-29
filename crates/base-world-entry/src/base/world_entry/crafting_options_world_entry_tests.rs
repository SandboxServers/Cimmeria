//! `playCharacter` starts a character's crafting session from nothing: the
//! stations, tools and "craft anywhere" of whatever this connection played
//! before do not carry over, and no crafting options go out until the new
//! entry's login send.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_crafting::base::crafting::options::CraftingSessionOptions;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::crafting::CraftingOptions;
use tokio::sync::mpsc;

use super::play_character::handle_play_character;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_base_crafting::base::crafting::options::CraftingOptionsExt;

/// Regression shape: a GM's `.allcraft` on one character (craft anywhere,
/// the player as its own machine) and that character's last stations would
/// otherwise reach the next character played on the same connection, in its
/// login bundle and its station gate.
#[tokio::test]
async fn play_character_starts_the_crafting_session_from_nothing() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let addr: SocketAddr = "127.0.0.1:55791".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.plugins = cimmeria_base_session::base::plugin::BasePlugins::build(&[
        &cimmeria_base_crafting::CraftingPlugin,
    ])
    .unwrap();
    state.crafting_options_mut().stations = [Some(900), None, None, None];
    state.crafting_options_mut().craft_anywhere = true;
    state.crafting_options_mut().armed = true;
    state.crafting_options_mut().last_sent = Some(CraftingOptions::default());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

    handle_play_character(
        &transport,
        addr,
        [0u8; 32],
        0xCCDE,
        8,
        &connected,
        &None,
        &entity_manager,
        &cell_tx,
    )
    .await
    .expect("no-DB world entry returns Ok");

    assert_eq!(
        connected.lock().unwrap()[&addr].crafting_options(),
        CraftingSessionOptions::default()
    );
}
