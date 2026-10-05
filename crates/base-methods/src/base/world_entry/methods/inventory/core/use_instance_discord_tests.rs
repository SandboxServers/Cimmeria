//! The `ItemUsed` Discord target (NT-10 review finding 1): the client sends
//! a cell entity ID, and the embed must never pair a character's name with
//! it.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_discord::Named;

use super::use_instance::discord_item_target;
use crate::base::ConnectedClientState;
use crate::test_support::test_default_connected_client_state;

const BOB_ENTITY: u32 = 4711;

/// Bob (player_id 13) is connected and plays entity 4711.
fn bob_online() -> (
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let addr: SocketAddr = "127.0.0.1:40001".parse().unwrap();
    let mut bob = test_default_connected_client_state();
    bob.player_entity_id = Some(BOB_ENTITY);
    bob.active_player_id = Some(13);
    bob.player_name = Some("Bob".into());
    let e2a = Arc::new(Mutex::new(HashMap::from([(BOB_ENTITY, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, bob)])));
    (e2a, connected)
}

/// **Regression guard.** A player target renders as that player's
/// character pair, `Bob (#13)`, as in every other embed. The old code
/// paired the name with the entity ID: `Bob (#4711)`.
#[test]
fn player_target_pairs_with_player_id_not_entity_id() {
    let (e2a, connected) = bob_online();
    let target = discord_item_target(BOB_ENTITY as i32, &e2a, &connected);
    assert_eq!(target, Some(Named::new(13, Some("Bob".into()))));
}

/// A target with no player session is labelled as an entity, never given
/// a bare `#id` a reader could take for a player or seed ID.
#[test]
fn non_player_target_is_labelled_as_an_entity() {
    let (e2a, connected) = bob_online();
    assert_eq!(
        discord_item_target(9001, &e2a, &connected),
        Some(Named::name_only("entity:9001"))
    );
}

#[test]
fn no_target_is_none() {
    let (e2a, connected) = bob_online();
    assert_eq!(discord_item_target(0, &e2a, &connected), None);
}
