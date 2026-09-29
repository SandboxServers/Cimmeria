//! `BasePlugins`: the startup checks, the envelope routing and the hook
//! firing order.
//!
//! The production ownership lists are empty until a base feature moves, so
//! these tests validate against their own ([`OWNED`]) through
//! `BasePlugins::build_with`. Hooks report into a process-wide log keyed by
//! a per-test entity id, so tests running in parallel never read each
//! other's rows.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::base::duel::SEND_DUEL_CHALLENGE;
use tracing::Level;

use super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

/// `sendDuelChallenge` (25) and `perfStats` (29), flattened.
const DUEL: u8 = SEND_DUEL_CHALLENGE - 0xC0;
const PERF: u8 = 0xDD - 0xC0;

struct Ping(u32);
struct Pong(u32);
struct Undeclared;

const MESSAGES: &[PluginMsgKind] = &[PluginMsgKind::of::<Ping>(), PluginMsgKind::of::<Pong>()];
const OWNED: PluginOwnership = PluginOwnership {
    base_methods: &[DUEL, PERF],
    cell_messages: MESSAGES,
};

// ── The hook log ─────────────────────────────────────────────────────────

static LOG: Mutex<Vec<(u32, &'static str)>> = Mutex::new(Vec::new());

fn record(entity_id: u32, what: &'static str) {
    LOG.lock().unwrap().push((entity_id, what));
}

fn log_for(entity_id: u32) -> Vec<&'static str> {
    LOG.lock()
        .unwrap()
        .iter()
        .filter(|(e, _)| *e == entity_id)
        .map(|(_, w)| *w)
        .collect()
}

// ── Handlers ─────────────────────────────────────────────────────────────

fn noop_method(_call: BaseMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async {})
}

fn ping_consumer(msg: PluginMsg, _ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        let Ping(entity_id) = msg.downcast::<Ping>().expect("routed by type");
        record(entity_id, "ping");
    })
}

fn pong_consumer(msg: PluginMsg, _ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        let Pong(entity_id) = msg.downcast::<Pong>().expect("routed by type");
        record(entity_id, "pong");
    })
}

fn session_a(event: SessionEvent) {
    record(event.entity_id, "session_a");
}
fn session_b(event: SessionEvent) {
    record(event.entity_id, "session_b");
}
fn state_hook(state: &mut ConnectedClientState) {
    state.account_id += 1;
}
fn world_entry_a(call: WorldEntryCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move { record(call.entity_id, "world_a") })
}
fn world_entry_b(call: WorldEntryCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move { record(call.entity_id, "world_b") })
}

// ── Plugins ──────────────────────────────────────────────────────────────

/// A plugin that registers the given base methods and nothing else.
struct Methods(&'static str, &'static [u8]);
impl BasePlugin for Methods {
    fn name(&self) -> &'static str {
        self.0
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        for &i in self.1 {
            plugin.base_method(i, noop_method);
        }
    }
}

struct PingPlugin;
impl BasePlugin for PingPlugin {
    fn name(&self) -> &'static str {
        "ping"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin.on_cell_message::<Ping>(ping_consumer);
    }
}

struct PongPlugin;
impl BasePlugin for PongPlugin {
    fn name(&self) -> &'static str {
        "pong"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin.on_cell_message::<Pong>(pong_consumer);
    }
}

struct UndeclaredPlugin;
impl BasePlugin for UndeclaredPlugin {
    fn name(&self) -> &'static str {
        "undeclared"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin.on_cell_message::<Undeclared>(ping_consumer);
    }
}

/// Registers one hook of every kind, tagged `a`.
struct HooksA;
impl BasePlugin for HooksA {
    fn name(&self) -> &'static str {
        "hooks_a"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .session_hook(SessionHookPoint::LogOffAfterEntityUnmapped, session_a)
            .session_hook(
                SessionHookPoint::GateTravelBeforeActiveCharacterCheck,
                session_a,
            )
            .session_state_hook(
                SessionStateHookPoint::GateTravelBeforeCreateEntity,
                state_hook,
            )
            .world_entry_hook(
                WorldEntryHookPoint::ClientReadyAfterOrgRestore,
                world_entry_a,
            );
    }
}

/// Registers the log-off and world-entry hooks, tagged `b`.
struct HooksB;
impl BasePlugin for HooksB {
    fn name(&self) -> &'static str {
        "hooks_b"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .session_hook(SessionHookPoint::LogOffAfterEntityUnmapped, session_b)
            .session_state_hook(
                SessionStateHookPoint::GateTravelBeforeCreateEntity,
                state_hook,
            )
            .world_entry_hook(
                WorldEntryHookPoint::ClientReadyAfterOrgRestore,
                world_entry_b,
            );
    }
}

// ── A base context over empty maps ───────────────────────────────────────

struct World {
    db_pool: Option<Arc<sqlx::PgPool>>,
    cell_tx: Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl World {
    fn new() -> Self {
        Self {
            db_pool: None,
            cell_tx: None,
            transport: Arc::new(TestTransport::new()),
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    fn ctx(&self) -> BaseCtx<'_> {
        BaseCtx {
            db_pool: &self.db_pool,
            cell_tx: &self.cell_tx,
            transport: &self.transport,
            connected: &self.connected,
            entity_to_addr: &self.entity_to_addr,
        }
    }
}

// ── Startup checks ───────────────────────────────────────────────────────

#[test]
fn the_production_lists_name_crafting_and_an_empty_table_is_incomplete() {
    assert!(PLUGIN_OWNED_BASE_METHODS.is_empty());
    let declared: Vec<&str> = PLUGIN_CELL_MESSAGES.iter().map(|k| k.type_name()).collect();
    assert_eq!(declared.len(), 7, "{declared:?}");
    assert!(
        declared
            .iter()
            .all(|t| t.starts_with("cimmeria_wire::crafting::")),
        "{declared:?}"
    );
    let plugins = BasePlugins::build(&[]).expect("an empty table builds");
    assert!(plugins.plugin_names().is_empty());
    assert_eq!(plugins.base_method_indices().count(), 0);
    // Without crafting nothing consumes its payloads: the orchestrator must
    // refuse to start, and a bare registry says the same.
    for empty in [plugins, BasePlugins::empty()] {
        match empty.check_complete() {
            Err(BasePluginError::MissingCellMessageConsumers { missing }) => {
                assert_eq!(missing, declared);
            }
            other => panic!("an empty table must miss the crafting consumers: {other:?}"),
        }
    }
}

#[test]
fn a_complete_table_registers_every_owned_index_and_message() {
    let plugins = BasePlugins::build_with(
        &[&Methods("m", &[PERF, DUEL]), &PingPlugin, &PongPlugin],
        OWNED,
    )
    .expect("builds");
    plugins.check_complete().expect("complete");
    assert_eq!(plugins.plugin_names(), &["m", "ping", "pong"]);
    assert_eq!(
        plugins.base_method_indices().collect::<Vec<_>>(),
        vec![DUEL, PERF],
        "ascending, whatever the registration order"
    );
    assert!(plugins.base_method(DUEL).is_some());
    assert!(plugins.base_method(0).is_none());
    let types: Vec<&str> = plugins.cell_message_types().collect();
    assert_eq!(types.len(), 2);
    assert!(types[0].ends_with("Ping") && types[1].ends_with("Pong"));
}

#[test]
fn two_plugins_claiming_one_base_method_fail_the_build() {
    let err = BasePlugins::build_with(&[&Methods("a", &[DUEL]), &Methods("b", &[DUEL])], OWNED)
        .unwrap_err();
    assert_eq!(
        err,
        BasePluginError::DuplicateBaseMethod {
            index: DUEL,
            name: "sendDuelChallenge",
            first: "a",
            second: "b",
        }
    );
}

#[test]
fn an_index_past_the_exposed_base_methods_fails_the_build() {
    let err = BasePlugins::build_with(&[&Methods("a", &[30])], OWNED).unwrap_err();
    assert_eq!(
        err,
        BasePluginError::UnknownBaseMethod {
            index: 30,
            plugin: "a"
        }
    );
}

#[test]
fn a_known_index_not_on_the_owned_list_fails_the_build() {
    // chatJoin (0) is a real base method the static router still handles.
    let err = BasePlugins::build_with(&[&Methods("a", &[0])], OWNED).unwrap_err();
    assert_eq!(
        err,
        BasePluginError::NotPluginOwned {
            index: 0,
            name: "chatJoin",
            plugin: "a"
        }
    );
    // And against the production list, every index is not owned yet.
    assert!(matches!(
        BasePlugins::build(&[&Methods("a", &[DUEL])]),
        Err(BasePluginError::NotPluginOwned { index: DUEL, .. })
    ));
}

/// The #962 missing-registration rule: a table that leaves an owned index
/// without a handler must not start. `check_complete` names it, and the
/// orchestrator refuses to start the base.
#[test]
fn an_owned_base_method_with_no_handler_fails_check_complete() {
    let plugins =
        BasePlugins::build_with(&[&Methods("m", &[PERF]), &PingPlugin, &PongPlugin], OWNED)
            .expect("builds");
    assert_eq!(
        plugins.check_complete().unwrap_err(),
        BasePluginError::MissingBaseMethods {
            missing: vec![(DUEL, "sendDuelChallenge")]
        }
    );
}

#[test]
fn two_consumers_of_one_payload_type_fail_the_build() {
    let err = BasePlugins::build_with(&[&PingPlugin, &PingPlugin], OWNED).unwrap_err();
    assert!(
        matches!(
            err,
            BasePluginError::DuplicateCellMessage {
                first: "ping",
                second: "ping",
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn a_consumer_of_an_undeclared_payload_type_fails_the_build() {
    let err = BasePlugins::build_with(&[&UndeclaredPlugin], OWNED).unwrap_err();
    match err {
        BasePluginError::UndeclaredCellMessage { type_name, plugin } => {
            assert!(type_name.ends_with("Undeclared"), "{type_name}");
            assert_eq!(plugin, "undeclared");
        }
        other => panic!("expected UndeclaredCellMessage, got {other:?}"),
    }
}

/// A declared payload type nobody consumes fails `check_complete`, naming
/// the type: the cell would send it and the base would drop it.
#[test]
fn a_declared_payload_type_with_no_consumer_fails_check_complete() {
    let plugins =
        BasePlugins::build_with(&[&Methods("m", &[PERF, DUEL]), &PingPlugin], OWNED).unwrap();
    match plugins.check_complete().unwrap_err() {
        BasePluginError::MissingCellMessageConsumers { missing } => {
            assert_eq!(missing.len(), 1);
            assert!(missing[0].ends_with("Pong"), "{missing:?}");
        }
        other => panic!("expected MissingCellMessageConsumers, got {other:?}"),
    }
}

// ── The envelope ─────────────────────────────────────────────────────────

#[tokio::test]
async fn an_envelope_reaches_the_consumer_of_its_payload_type_only() {
    let world = World::new();
    let plugins = BasePlugins::build_with(&[&PingPlugin, &PongPlugin], OWNED).unwrap();
    assert!(
        plugins
            .dispatch_cell_message(PluginMsg::new(Pong(7_101)), world.ctx())
            .await
    );
    assert!(
        plugins
            .dispatch_cell_message(PluginMsg::new(Ping(7_101)), world.ctx())
            .await
    );
    assert_eq!(log_for(7_101), vec!["pong", "ping"]);
}

/// The runtime half of the missing-registration rule: an envelope nobody
/// consumes is dropped with a WARN naming its payload type and
/// `reason = "no_consumer"`, never silently.
#[tokio::test]
async fn an_envelope_with_no_consumer_warns_and_is_dropped() {
    let capture = LogCapture::install();
    let world = World::new();
    let consumed = BasePlugins::empty()
        .dispatch_cell_message(PluginMsg::new(Ping(7_102)), world.ctx())
        .await;
    assert!(!consumed);
    assert!(log_for(7_102).is_empty());
    let warn = capture
        .find_message(Level::WARN, "cell message has no base plugin consumer")
        .unwrap_or_else(|| panic!("no-consumer WARN missing: {:#?}", capture.all()));
    assert!(
        warn.fields
            .get("reason")
            .is_some_and(|r| r.trim_matches('"') == "no_consumer"),
        "{warn:?}"
    );
    assert!(
        warn.fields
            .get("type_name")
            .is_some_and(|t| t.trim_matches('"').ends_with("Ping")),
        "{warn:?}"
    );
    assert_eq!(warn.target, "base.plugin");
}

// ── Hooks ────────────────────────────────────────────────────────────────

#[test]
fn session_hooks_fire_at_their_point_in_table_order() {
    let plugins = BasePlugins::build(&[&HooksA, &HooksB]).unwrap();
    let event = |entity_id| SessionEvent {
        entity_id,
        cause: "test",
    };
    plugins.run_session_hook(SessionHookPoint::LogOffAfterEntityUnmapped, event(7_103));
    assert_eq!(log_for(7_103), vec!["session_a", "session_b"]);

    plugins.run_session_hook(
        SessionHookPoint::GateTravelBeforeActiveCharacterCheck,
        event(7_104),
    );
    assert_eq!(log_for(7_104), vec!["session_a"]);

    // Nothing subscribes to the disconnect point here.
    plugins.run_session_hook(
        SessionHookPoint::DisconnectAfterEntityUnmapped,
        event(7_105),
    );
    assert!(log_for(7_105).is_empty());

    // Reversing the table reverses the order.
    let reversed = BasePlugins::build(&[&HooksB, &HooksA]).unwrap();
    reversed.run_session_hook(SessionHookPoint::LogOffAfterEntityUnmapped, event(7_106));
    assert_eq!(log_for(7_106), vec!["session_b", "session_a"]);
}

#[test]
fn session_state_hooks_get_the_session_state() {
    let plugins = BasePlugins::build(&[&HooksA, &HooksB]).unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = 40;
    plugins.run_session_state_hook(
        SessionStateHookPoint::GateTravelBeforeCreateEntity,
        &mut state,
    );
    assert_eq!(state.account_id, 42, "both plugins' hooks ran on the state");
    BasePlugins::empty().run_session_state_hook(
        SessionStateHookPoint::GateTravelBeforeCreateEntity,
        &mut state,
    );
    assert_eq!(state.account_id, 42, "an empty registry changes nothing");
}

#[tokio::test]
async fn world_entry_hooks_fire_in_table_order() {
    let world = World::new();
    let plugins = BasePlugins::build(&[&HooksA, &HooksB]).unwrap();
    let call = WorldEntryCall {
        addr: "127.0.0.1:7107".parse().unwrap(),
        entity_id: 7_107,
        player_id: 1,
        ctx: world.ctx(),
    };
    plugins
        .run_world_entry_hook(WorldEntryHookPoint::ClientReadyAfterOrgRestore, call)
        .await;
    assert_eq!(log_for(7_107), vec!["world_a", "world_b"]);
}

// ── The session carries the registry ─────────────────────────────────────

#[test]
fn session_plugins_reads_the_registry_the_session_was_admitted_under() {
    let addr: SocketAddr = "127.0.0.1:7108".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.plugins = BasePlugins::build(&[&HooksA]).unwrap();
    let connected = Mutex::new(HashMap::from([(addr, state)]));
    assert_eq!(
        session_plugins(&connected, addr).plugin_names(),
        &["hooks_a"]
    );

    let absent: SocketAddr = "127.0.0.1:7109".parse().unwrap();
    assert!(session_plugins(&connected, absent)
        .plugin_names()
        .is_empty());

    // A fresh session carries an empty registry and no extensions.
    let fresh = test_default_connected_client_state();
    assert!(fresh.plugins.plugin_names().is_empty());
    assert!(!fresh.extensions.contains::<Ping>());
}

#[test]
fn a_session_keeps_one_extension_slot_per_type() {
    #[derive(Debug, PartialEq)]
    struct Queue(Vec<u32>);

    let mut state = test_default_connected_client_state();
    assert!(state.extensions.insert(Queue(vec![1])).is_none());
    state.extensions.get_mut::<Queue>().unwrap().0.push(2);
    assert_eq!(state.extensions.get::<Queue>(), Some(&Queue(vec![1, 2])));
    assert_eq!(state.extensions.remove::<Queue>(), Some(Queue(vec![1, 2])));
    assert!(!state.extensions.contains::<Queue>());
}
