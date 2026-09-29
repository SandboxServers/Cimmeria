//! `onClientReady` fires the base plugins' world-entry hook (#962 step 5)
//! once, with the entering entity and character, where crafting's login
//! sync runs; a session with no pending world entry fires nothing.

use std::sync::Mutex as StdMutex;

use super::*;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_base_session::base::plugin::{
    BasePlugin, BasePluginBuilder, BasePlugins, BoxFuture, WorldEntryCall, WorldEntryHookPoint,
};

/// `(entity id, player id, port)` rows from the hook.
static ENTRIES: StdMutex<Vec<(u32, i32, u16)>> = StdMutex::new(Vec::new());

fn on_entry(call: WorldEntryCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        ENTRIES
            .lock()
            .unwrap()
            .push((call.entity_id, call.player_id, call.addr.port()));
    })
}

struct EntryPlugin;
impl BasePlugin for EntryPlugin {
    fn name(&self) -> &'static str {
        "entry"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin.world_entry_hook(WorldEntryHookPoint::ClientReadyAfterOrgRestore, on_entry);
    }
}

fn entries_for(entity_id: u32) -> Vec<(u32, i32, u16)> {
    ENTRIES
        .lock()
        .unwrap()
        .iter()
        .copied()
        .filter(|(e, _, _)| *e == entity_id)
        .collect()
}

async fn client_ready(addr: SocketAddr, entity_id: u32, pending: bool) {
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(entity_id);
    state.plugins = BasePlugins::build(&[&EntryPlugin]).unwrap();
    if pending {
        state.pending_client_ready = Some(crate::base::PendingClientReadyInfo {
            entity_id,
            player_id: 4_700,
            world_name: "Agnos".to_string(),
            appearance_args: vec![0xAB],
            tint_args: vec![0xCD],
            first_login: 0,
        });
    }
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let _ = handle_on_client_ready(
        addr,
        [0u8; 32],
        &connected,
        &None,
        &transport,
        &entity_to_addr,
        &None,
    )
    .await;
}

#[tokio::test]
async fn client_ready_fires_the_world_entry_hook_once() {
    let addr: SocketAddr = "127.0.0.1:55701".parse().unwrap();
    client_ready(addr, 4_701, true).await;
    assert_eq!(entries_for(4_701), vec![(4_701, 4_700, 55_701)]);
}

#[tokio::test]
async fn client_ready_without_a_pending_entry_fires_nothing() {
    let addr: SocketAddr = "127.0.0.1:55702".parse().unwrap();
    client_ready(addr, 4_702, false).await;
    assert!(entries_for(4_702).is_empty());
}
