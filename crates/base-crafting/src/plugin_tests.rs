//! `CraftingPlugin`: its registrations, an envelope routed through the
//! registry to a crafting handler, and the session-state hooks.
//!
//! The other hooks are pinned where core fires them: the teardown drop here
//! in `session::tests`, `logOff` in `cimmeria-base`'s
//! `dispatch::tests::crafting_teardown`, gate travel and `playCharacter` in
//! `cimmeria-base-world-entry`, the item-use, tool-refresh and ASP seams in
//! `cimmeria-base-methods`' live-DB tests.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::plugin::{
    BaseCtx, BasePlugins, PluginMsg, SessionStateHookPoint, PLUGIN_CELL_MESSAGES,
};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::crafting::{CraftingStations, StationChangeCause};

use crate::base::crafting::options::{session_options, CraftingOptionsExt};
use crate::test_support::{test_default_connected_client_state, TestTransport};
use crate::CraftingPlugin;

fn crafting() -> BasePlugins {
    BasePlugins::build(&[&CraftingPlugin]).expect("the crafting plugin builds")
}

/// Crafting alone completes the production table: it consumes every
/// declared envelope payload, in the declared order, and registers no base
/// method (its verbs are cell methods).
#[test]
fn crafting_consumes_every_declared_payload_and_no_base_method() {
    let plugins = crafting();
    plugins
        .check_complete()
        .expect("crafting completes the base table");
    assert_eq!(plugins.plugin_names(), &["crafting"]);
    assert_eq!(plugins.base_method_indices().count(), 0);
    let declared: Vec<&str> = PLUGIN_CELL_MESSAGES.iter().map(|k| k.type_name()).collect();
    assert_eq!(plugins.cell_message_types().collect::<Vec<_>>(), declared);
}

/// A station report in the envelope reaches `options::handle_station_report`
/// through the registry: the session's stations are the report's.
#[tokio::test]
async fn a_station_report_envelope_reaches_the_crafting_options() {
    const ENTITY: u32 = 4_801;
    let addr: SocketAddr = "127.0.0.1:55801".parse().unwrap();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    let report = CraftingStations {
        entity_id: ENTITY,
        player_id: 4_802,
        stations: [Some(900), None, Some(901), None],
        cause: StationChangeCause::Moved,
    };

    let consumed = crafting()
        .dispatch_cell_message(
            PluginMsg::new(report),
            BaseCtx {
                db_pool: &None,
                cell_tx: &None,
                transport: &transport,
                connected: &connected,
                entity_to_addr: &entity_to_addr,
            },
        )
        .await;

    assert!(consumed);
    let stations = connected.lock().unwrap()[&addr].crafting_options().stations;
    assert_eq!(stations, [Some(900), None, Some(901), None]);
}

/// `playCharacter` resets the options: nothing carries over from what the
/// connection played before.
#[test]
fn the_play_character_hook_resets_the_crafting_options() {
    let mut state = test_default_connected_client_state();
    state.crafting_options_mut().craft_anywhere = true;
    state.crafting_options_mut().stations = [Some(900), None, None, None];
    crafting().run_session_state_hook(
        SessionStateHookPoint::PlayCharacterAfterEntryLatch,
        &mut state,
    );
    assert!(session_options(&state).is_none());
    assert_eq!(state.crafting_options(), Default::default());
}

/// Gate travel forgets the origin world's stations and disarms the change
/// sends, keeping the tools and "craft anywhere"; a session crafting never
/// touched stays untouched.
#[test]
fn the_gate_travel_hook_forgets_the_stations_only() {
    let mut state = test_default_connected_client_state();
    state.crafting_options_mut().craft_anywhere = true;
    state.crafting_options_mut().stations = [Some(900), None, None, Some(901)];
    state.crafting_options_mut().armed = true;
    crafting().run_session_state_hook(
        SessionStateHookPoint::GateTravelBeforeCreateEntity,
        &mut state,
    );
    let options = state.crafting_options();
    assert_eq!(options.stations, [None; 4]);
    assert!(!options.armed);
    assert!(
        options.craft_anywhere,
        "craft anywhere carries over a world change"
    );

    let mut fresh = test_default_connected_client_state();
    crafting().run_session_state_hook(
        SessionStateHookPoint::GateTravelBeforeCreateEntity,
        &mut fresh,
    );
    assert!(
        session_options(&fresh).is_none(),
        "nothing stored for an untouched session"
    );
}
