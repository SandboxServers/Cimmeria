//! Live-DB fixtures shared by the crafting verb tests: one account and one
//! player row per sentinel pair, removed by exact id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::sync::CraftClient;
use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, TestTransport};

/// Delete the player's expertise rows, the player and the account.
pub(crate) async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert an account and a level-1 player that leaves every crafting
/// column to its schema default.
pub(crate) async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("craft-cr-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("craft-cr-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// The `account_id` every [`OneSession`] carries, so tests can assert the
/// identity fields on crafting events.
pub(crate) const SESSION_ACCOUNT_ID: u32 = 4242;

/// One connected session for `entity_id`, with the transport that
/// records what it is sent.
pub(crate) struct OneSession {
    pub(crate) addr: SocketAddr,
    pub(crate) typed: Arc<TestTransport>,
    pub(crate) transport: Arc<dyn Transport>,
    pub(crate) connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(crate) entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl OneSession {
    pub(crate) fn new(entity_id: u32, port: u16) -> Self {
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        let typed = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed.clone();
        let mut state = test_default_connected_client_state();
        state.account_id = SESSION_ACCOUNT_ID;
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        Self {
            addr,
            typed,
            transport,
            connected,
            entity_to_addr,
        }
    }

    pub(crate) fn client(&self) -> CraftClient<'_> {
        CraftClient {
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }
}
