//! A world change drops the player's queued crafting inductions. The
//! session keeps its entity id and its recorded world name through gate
//! travel, so this drop is the only thing that stops a queued craft from
//! completing in the new world.

use super::*;
use cimmeria_base_crafting::base::crafting::session::{
    crafting_sessions, Completion, InductionEnv, InductionJob, JobFuture, JobOutcome, SubmitOutcome,
};

/// A job that must never complete in this test.
struct Parked;

impl InductionJob for Parked {
    fn verb(&self) -> &'static str {
        "parked"
    }

    fn timer_id(&self) -> i32 {
        0
    }

    fn complete<'a>(self: Box<Self>, _done: Completion<'a>) -> JobFuture<'a> {
        Box::pin(async { JobOutcome::Failed })
    }
}

#[tokio::test]
async fn gate_travel_drops_the_crafting_queue() {
    // A private entity id: the server-wide crafting sessions are shared by
    // every test in the process, and the other gate-travel tests use 42.
    // `make_state` plays character 7.
    const ENTITY: u32 = 4295;
    let transport = make_socket().await;
    let addr: SocketAddr = "127.0.0.1:55711".parse().unwrap();
    let mut state = make_state();
    state.plugins = cimmeria_base_session::base::plugin::BasePlugins::build(&[
        &cimmeria_base_crafting::CraftingPlugin,
    ])
    .unwrap();
    state.player_entity_id = Some(ENTITY);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    let env = InductionEnv {
        db_pool: None,
        cell_tx: None,
        transport: transport.clone(),
        connected: connected.clone(),
        entity_to_addr: entity_to_addr.clone(),
    };
    let sessions = crafting_sessions();
    assert_eq!(
        sessions.submit(ENTITY, 7, Box::new(Parked), &env).await,
        SubmitOutcome::Started
    );
    assert_eq!(sessions.pending(ENTITY), 1);

    // No cached character: the transfer aborts right after the drop,
    // before any cell round trip.
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .active_player_id = None;
    let _ = handle_gate_travel(
        ENTITY,
        "Castle",
        [0.0; 3],
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

    assert_eq!(
        sessions.pending(ENTITY),
        0,
        "a world change drops the queue"
    );
}
