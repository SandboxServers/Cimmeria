//! The three seam hook kinds `cimmeria-base-methods` fires (#962 step 5):
//! item use (the first plugin that handles it decides), the inventory
//! resync and the applied-science total, plus `entity_plugins`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use super::*;
use crate::test_support::{test_default_connected_client_state, TestTransport};

/// `(key, what)` rows the test hooks write, keyed by entity (or item) id so
/// parallel tests never read each other's rows.
static LOG: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());

fn log_for(key: u32) -> Vec<String> {
    LOG.lock()
        .unwrap()
        .iter()
        .filter(|(k, _)| *k == key)
        .map(|(_, w)| w.clone())
        .collect()
}

fn record(key: u32, what: String) {
    LOG.lock().unwrap().push((key, what));
}

fn declines(call: ItemUseCall<'_>) -> BoxFuture<'_, ItemUseOutcome> {
    Box::pin(async move {
        record(call.entity_id, "declines".into());
        ItemUseOutcome::NotHandled
    })
}
fn refuses(call: ItemUseCall<'_>) -> BoxFuture<'_, ItemUseOutcome> {
    Box::pin(async move {
        record(call.entity_id, "refuses".into());
        ItemUseOutcome::Refused
    })
}
fn consumes(call: ItemUseCall<'_>) -> BoxFuture<'_, ItemUseOutcome> {
    Box::pin(async move {
        record(call.entity_id, "consumes".into());
        ItemUseOutcome::Consumed(ItemConsumed {
            removed_all: true,
            outbox: None,
        })
    })
}
fn rows_seen(call: InventoryCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move { record(call.entity_id, format!("rows {:?}", call.rows)) })
}
fn asp_seen(call: AppliedScienceCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move { record(call.entity_id, format!("asp {}", call.total)) })
}

/// A plugin registering the given item-use hook at both points.
struct ItemUser(&'static str, ItemUseHook);
impl BasePlugin for ItemUser {
    fn name(&self) -> &'static str {
        self.0
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .item_use_hook(ItemUseHookPoint::CraftingItem, self.1)
            .item_use_hook(ItemUseHookPoint::InstanceNotFound, self.1);
    }
}

struct Watcher;
impl BasePlugin for Watcher {
    fn name(&self) -> &'static str {
        "watcher"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .inventory_hook(InventoryHookPoint::AfterFullInventoryUpdate, rows_seen)
            .progression_hook(ProgressionHookPoint::AfterAppliedScienceEarned, asp_seen);
    }
}

struct World {
    db_pool: Option<Arc<sqlx::PgPool>>,
    pool: Arc<sqlx::PgPool>,
    cell_tx: Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl World {
    fn new() -> Self {
        // Never connected: the hooks here do not touch the database.
        let pool = Arc::new(
            sqlx::postgres::PgPoolOptions::new()
                .connect_lazy("postgres://nobody@127.0.0.1:1/none")
                .expect("a lazy pool"),
        );
        Self {
            db_pool: Some(Arc::clone(&pool)),
            pool,
            cell_tx: None,
            transport: Arc::new(TestTransport::new()),
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    fn item_use(&self, entity_id: u32) -> ItemUseCall<'_> {
        ItemUseCall {
            entity_id,
            player_id: 1,
            item_id: 2,
            pool: &self.pool,
            ctx: BaseCtx {
                db_pool: &self.db_pool,
                cell_tx: &self.cell_tx,
                transport: &self.transport,
                connected: &self.connected,
                entity_to_addr: &self.entity_to_addr,
            },
        }
    }
}

/// Hooks run in table order until one handles the use: a declining plugin
/// lets the next one decide, and the one after a decision never runs.
#[tokio::test]
async fn the_first_plugin_that_handles_an_item_use_decides() {
    let world = World::new();
    let plugins = BasePlugins::build(&[
        &ItemUser("a", declines),
        &ItemUser("b", refuses),
        &ItemUser("c", consumes),
    ])
    .unwrap();
    let outcome = plugins
        .run_item_use_hook(ItemUseHookPoint::CraftingItem, world.item_use(4_901))
        .await;
    assert!(matches!(outcome, ItemUseOutcome::Refused), "{outcome:?}");
    assert_eq!(log_for(4_901), vec!["declines", "refuses"]);

    let consumed = BasePlugins::build(&[&ItemUser("a", declines), &ItemUser("c", consumes)])
        .unwrap()
        .run_item_use_hook(ItemUseHookPoint::InstanceNotFound, world.item_use(4_902))
        .await;
    assert!(
        matches!(
            consumed,
            ItemUseOutcome::Consumed(ItemConsumed {
                removed_all: true,
                outbox: None
            })
        ),
        "{consumed:?}"
    );
}

/// With no plugin, or only declining ones, an item use is `NotHandled`, so
/// core runs its own line (a WARN).
#[tokio::test]
async fn an_item_use_nobody_handles_is_not_handled() {
    let world = World::new();
    for plugins in [
        BasePlugins::empty(),
        BasePlugins::build(&[&ItemUser("a", declines)]).unwrap(),
    ] {
        let outcome = plugins
            .run_item_use_hook(ItemUseHookPoint::CraftingItem, world.item_use(4_903))
            .await;
        assert!(matches!(outcome, ItemUseOutcome::NotHandled), "{outcome:?}");
    }
}

#[tokio::test]
async fn the_inventory_and_progression_hooks_get_their_values() {
    let world = World::new();
    let plugins = BasePlugins::build(&[&Watcher]).unwrap();
    let rows = [(10, 20, 15), (11, 21, 1)];
    plugins
        .run_inventory_hook(
            InventoryHookPoint::AfterFullInventoryUpdate,
            InventoryCall {
                entity_id: 4_904,
                player_id: 1,
                pool: &world.pool,
                rows: &rows,
                transport: &world.transport,
                connected: &world.connected,
                entity_to_addr: &world.entity_to_addr,
            },
        )
        .await;
    plugins
        .run_progression_hook(
            ProgressionHookPoint::AfterAppliedScienceEarned,
            AppliedScienceCall {
                entity_id: 4_904,
                player_id: 1,
                total: 7,
                transport: &world.transport,
                connected: &world.connected,
                entity_to_addr: &world.entity_to_addr,
            },
        )
        .await;
    assert_eq!(
        log_for(4_904),
        vec!["rows [(10, 20, 15), (11, 21, 1)]", "asp 7"]
    );
}

/// `entity_plugins` finds the registry of the session playing the entity,
/// and an empty one for an entity with no session.
#[test]
fn entity_plugins_follows_the_entity_to_its_session() {
    let addr: SocketAddr = "127.0.0.1:54905".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.plugins = BasePlugins::build(&[&Watcher]).unwrap();
    let connected = Mutex::new(HashMap::from([(addr, state)]));
    let entity_to_addr = Mutex::new(HashMap::from([(4_905u32, addr)]));
    assert_eq!(
        entity_plugins(&connected, &entity_to_addr, 4_905).plugin_names(),
        &["watcher"]
    );
    assert!(entity_plugins(&connected, &entity_to_addr, 4_906)
        .plugin_names()
        .is_empty());
}
