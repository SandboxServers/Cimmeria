//! The base plugin seams in `cimmeria-base` (#962 step 5): the base-method
//! router asks the session's plugin registry before its static arms, and
//! `logOff` and the disconnect teardown fire their session hooks.
//!
//! The production plugin-owned list is empty until a base feature with a
//! base method moves, so the routing tests register against their own
//! ownership list (`BasePlugins::build_with`).

use std::sync::Mutex as StdMutex;

use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_base_session::base::plugin::{
    BaseMethodCall, BasePlugin, BasePluginBuilder, BasePlugins, BoxFuture, PluginOwnership,
    SessionEvent, SessionHookPoint, PLUGIN_OWNED_BASE_METHODS,
};
use tracing::Level;

/// `minigameCallRequest` (0xD3): a real base method with no static arm.
const MINIGAME_CALL_REQUEST: u8 = 0xD3;

const OWNED: PluginOwnership = PluginOwnership {
    base_methods: &[
        MINIGAME_CALL_REQUEST - BASE_METHOD_ID,
        sgw_player_base::PERF_STATS - BASE_METHOD_ID,
    ],
    cell_messages: &[],
};

/// `(address port or entity id, what)` rows the test handlers and hooks
/// write, keyed so parallel tests never read each other's rows.
static CALLS: StdMutex<Vec<(u32, String)>> = StdMutex::new(Vec::new());

fn calls_for(key: u32) -> Vec<String> {
    CALLS
        .lock()
        .unwrap()
        .iter()
        .filter(|(k, _)| *k == key)
        .map(|(_, w)| w.clone())
        .collect()
}

fn recording_handler(call: BaseMethodCall<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        CALLS.lock().unwrap().push((
            u32::from(call.addr.port()),
            format!("method {} args {}", call.method_index, call.args.len()),
        ));
    })
}

fn log_off_hook(event: SessionEvent) {
    CALLS
        .lock()
        .unwrap()
        .push((event.entity_id, format!("log_off {}", event.cause)));
}

fn disconnect_hook(event: SessionEvent) {
    CALLS
        .lock()
        .unwrap()
        .push((event.entity_id, format!("disconnect {}", event.cause)));
}

struct RoutingPlugin;
impl BasePlugin for RoutingPlugin {
    fn name(&self) -> &'static str {
        "routing"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        for &index in OWNED.base_methods {
            plugin.base_method(index, recording_handler);
        }
    }
}

struct LifecyclePlugin;
impl BasePlugin for LifecyclePlugin {
    fn name(&self) -> &'static str {
        "lifecycle"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .session_hook(SessionHookPoint::LogOffAfterEntityUnmapped, log_off_hook)
            .session_hook(
                SessionHookPoint::DisconnectAfterEntityUnmapped,
                disconnect_hook,
            );
    }
}

struct Fixture {
    addr: SocketAddr,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
}

fn fixture(port: u16, plugins: BasePlugins, entity_id: Option<u32>) -> Fixture {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.plugins = plugins;
    state.player_entity_id = entity_id;
    let entity_to_addr = entity_id.map(|e| (e, addr)).into_iter().collect();
    Fixture {
        addr,
        transport: Arc::new(TestTransport::default()),
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        entity_to_addr: Arc::new(Mutex::new(entity_to_addr)),
        entity_manager: Arc::new(Mutex::new(EntityManager::new())),
    }
}

async fn dispatch(f: &Fixture, msg_id: u8, payload: &[u8]) {
    dispatch_sgw_player_base_method(
        msg_id,
        payload,
        &None,
        f.addr,
        &f.transport,
        [0u8; 32],
        &f.connected,
        &f.entity_manager,
        &None,
        &f.entity_to_addr,
        &None,
    )
    .await
    .expect("dispatch returns Ok");
}

fn unhandled_warned(capture: &crate::test_support::LogCaptureGuard, index: u8) -> bool {
    capture.all().iter().any(|e| {
        e.level == Level::WARN
            && e.message_contains("Unhandled SGWPlayer base method")
            && e.has_field("base_method_index", &index.to_string())
    })
}

/// The inversion guard (plugin ADR §3.3): no static arm may still claim a
/// plugin-owned index, or it would shadow a plugin that forgot to register
/// and hide the gap. With an empty registry, every plugin-owned index must
/// reach the router's unhandled WARN. The two controls prove the check can
/// fail: `perfStats` has a static arm (no WARN), `minigameCallRequest` has
/// none (WARN).
#[tokio::test]
async fn plugin_owned_base_methods_have_no_static_arm() {
    let capture = LogCapture::install();
    let f = fixture(54_401, BasePlugins::empty(), None);

    for &index in PLUGIN_OWNED_BASE_METHODS {
        dispatch(&f, BASE_METHOD_ID + index, &[]).await;
        assert!(
            unhandled_warned(&capture, index),
            "plugin-owned base method {index} is still handled by a static arm"
        );
    }

    dispatch(&f, sgw_player_base::PERF_STATS, &[0u8; 48]).await;
    assert!(!unhandled_warned(
        &capture,
        sgw_player_base::PERF_STATS - BASE_METHOD_ID
    ));
    dispatch(&f, MINIGAME_CALL_REQUEST, &[]).await;
    assert!(unhandled_warned(
        &capture,
        MINIGAME_CALL_REQUEST - BASE_METHOD_ID
    ));
}

/// A registered base method reaches its plugin with the flattened index and
/// the argument bytes, before the static arms: `perfStats`' own DEBUG sink
/// does not run, and an index with no static arm no longer warns.
#[tokio::test]
async fn a_registered_base_method_reaches_its_plugin_before_the_static_arms() {
    let capture = LogCapture::install();
    let plugins = BasePlugins::build_with(&[&RoutingPlugin], OWNED).unwrap();
    plugins.check_complete().unwrap();
    let f = fixture(54_402, plugins, None);

    dispatch(&f, MINIGAME_CALL_REQUEST, &[1, 2, 3]).await;
    dispatch(&f, sgw_player_base::PERF_STATS, &[0u8; 48]).await;

    assert_eq!(
        calls_for(54_402),
        vec![
            "method 19 args 3".to_string(),
            "method 29 args 48".to_string()
        ]
    );
    assert!(!unhandled_warned(&capture, 19), "{:#?}", capture.all());
    assert!(
        capture
            .find_message(Level::DEBUG, "SGWPlayer.perfStats")
            .is_none(),
        "the static perfStats arm must not run for a plugin-owned index"
    );
}

/// The registry is the session's: a session admitted without the plugin
/// (an empty registry) falls through to the static router's WARN, the
/// runtime face of a missing registration (#311).
#[tokio::test]
async fn a_session_without_the_plugin_falls_through_to_the_unhandled_warn() {
    let capture = LogCapture::install();
    let f = fixture(54_403, BasePlugins::empty(), None);
    dispatch(&f, MINIGAME_CALL_REQUEST, &[]).await;
    assert!(calls_for(54_403).is_empty());
    assert!(unhandled_warned(&capture, 19));
}

/// `logOff` fires `LogOffAfterEntityUnmapped` once, with the player entity
/// and cause `log_off`, after the entity left `entity_to_addr`.
#[tokio::test]
async fn log_off_fires_the_session_hook_after_the_entity_is_unmapped() {
    const ENTITY: u32 = 4_404;
    let plugins = BasePlugins::build(&[&LifecyclePlugin]).unwrap();
    let f = fixture(54_404, plugins, Some(ENTITY));

    dispatch(&f, sgw_player_base::LOG_OFF, &[0u8]).await;

    assert_eq!(calls_for(ENTITY), vec!["log_off log_off".to_string()]);
    assert!(f.entity_to_addr.lock().unwrap().get(&ENTITY).is_none());
}

/// The disconnect teardown fires `DisconnectAfterEntityUnmapped` with the
/// disconnect reason, although the session has already left the map.
#[tokio::test]
async fn the_disconnect_teardown_fires_the_session_hook_with_its_reason() {
    const ENTITY: u32 = 4_405;
    let plugins = BasePlugins::build(&[&LifecyclePlugin]).unwrap();
    let f = fixture(54_405, plugins, Some(ENTITY));

    cimmeria_base_session::base::helpers::destroy_client_entities(
        &f.connected,
        &f.entity_manager,
        f.addr,
        &None,
        &f.entity_to_addr,
        &f.transport,
        &None,
        "inactivity_timeout",
    );

    assert_eq!(
        calls_for(ENTITY),
        vec!["disconnect inactivity_timeout".to_string()]
    );
    assert!(f.connected.lock().unwrap().is_empty());
}
