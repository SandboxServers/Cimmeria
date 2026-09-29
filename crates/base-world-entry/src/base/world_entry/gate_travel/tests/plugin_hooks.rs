//! Gate travel fires the base plugins' two gate-travel hooks (#962 step 5)
//! where crafting's inline calls sit: the session hook before the
//! fail-closed active-character check, so it runs for a refused transfer
//! too, and the state hook before the cell is asked for the destination
//! entity.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use cimmeria_base_session::base::plugin::{
    BasePlugin, BasePluginBuilder, BasePlugins, SessionEvent, SessionHookPoint,
    SessionStateHookPoint,
};
use std::sync::Mutex as StdMutex;
use std::time::Duration;
use tokio::time::timeout;

/// `(entity id, cause)` rows from the session hook.
static EVENTS: StdMutex<Vec<(u32, &'static str)>> = StdMutex::new(Vec::new());

/// What the state hook leaves in the session's extensions.
struct LeftOriginWorld;

fn on_travel(event: SessionEvent) {
    EVENTS.lock().unwrap().push((event.entity_id, event.cause));
}

fn on_state(state: &mut ConnectedClientState) {
    state.extensions.insert(LeftOriginWorld);
}

struct TravelPlugin;
impl BasePlugin for TravelPlugin {
    fn name(&self) -> &'static str {
        "travel"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin
            .session_hook(
                SessionHookPoint::GateTravelBeforeActiveCharacterCheck,
                on_travel,
            )
            .session_state_hook(
                SessionStateHookPoint::GateTravelBeforeCreateEntity,
                on_state,
            );
    }
}

fn events_for(entity_id: u32) -> Vec<&'static str> {
    EVENTS
        .lock()
        .unwrap()
        .iter()
        .filter(|(e, _)| *e == entity_id)
        .map(|(_, c)| *c)
        .collect()
}

#[tokio::test]
async fn gate_travel_fires_both_hooks_before_the_cell_create() {
    const ENTITY_ID: u32 = 4_501;
    let addr: SocketAddr = "127.0.0.1:55801".parse().unwrap();
    let mut state = make_state();
    state.plugins = BasePlugins::build(&[&TravelPlugin]).unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(crate::test_support::TestTransport::new());
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let (c, e, t) = (
        Arc::clone(&connected),
        Arc::clone(&entity_to_addr),
        Arc::clone(&transport),
    );
    let travel = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            "Castle_CellBlock",
            [1.0, 2.0, 3.0],
            [0.0; 3],
            None,
            None,
            &t,
            &c,
            &e,
            &Some(cell_tx),
            &None,
        )
        .await
    });

    let msg = timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg else {
        panic!("expected CreateEntity as the first base->cell message");
    };
    // Read at the moment the cell is asked for the entity.
    let hooked_at_create = connected.lock().unwrap()[&addr]
        .extensions
        .contains::<LeftOriginWorld>();
    let events_at_create = events_for(ENTITY_ID);
    let _ = reply_tx.send(0x0001_0001);
    timeout(Duration::from_secs(2), travel)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel completes");

    assert!(hooked_at_create, "the state hook ran before CreateEntity");
    assert_eq!(events_at_create, vec!["gate_travel"]);
    assert_eq!(events_for(ENTITY_ID), vec!["gate_travel"], "fired once");
}

/// A transfer refused for a missing active character still fires the
/// session hook (crafting's queue is dropped on that path today), but not
/// the state hook, which sits after the check.
#[tokio::test]
async fn a_refused_transfer_fires_only_the_session_hook() {
    const ENTITY_ID: u32 = 4_502;
    let addr: SocketAddr = "127.0.0.1:55802".parse().unwrap();
    let mut state = make_state();
    state.active_player_id = None;
    state.plugins = BasePlugins::build(&[&TravelPlugin]).unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(crate::test_support::TestTransport::new());

    let _ = handle_gate_travel(
        ENTITY_ID,
        "Castle_CellBlock",
        [1.0, 2.0, 3.0],
        [0.0; 3],
        None,
        None,
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
    )
    .await;

    assert_eq!(events_for(ENTITY_ID), vec!["gate_travel"]);
    // The refusal ends the session; if it survived, the state hook must
    // not have touched it.
    let clients = connected.lock().unwrap();
    if let Some(c) = clients.get(&addr) {
        assert!(!c.extensions.contains::<LeftOriginWorld>());
    }
}
