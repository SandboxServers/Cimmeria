//! `logOff` drops the player's queued crafting inductions on the server's
//! crafting sessions, before the entity id can be reused.

use super::super::*;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use cimmeria_base_session::base::crafting::session::{
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
async fn log_off_drops_the_crafting_queue() {
    // Private ids: the server-wide crafting sessions are shared by every
    // test in the process.
    const ENTITY: u32 = 4293;
    const PLAYER_ID: i32 = 4294;
    let addr: SocketAddr = "127.0.0.1:54331".parse().unwrap();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(ENTITY);
    state.active_player_id = Some(PLAYER_ID);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let env = InductionEnv {
        db_pool: None,
        cell_tx: None,
        transport: transport.clone(),
        connected: connected.clone(),
        entity_to_addr: entity_to_addr.clone(),
    };
    let sessions = crafting_sessions();
    assert_eq!(
        sessions
            .submit(ENTITY, PLAYER_ID, Box::new(Parked), &env)
            .await,
        SubmitOutcome::Started
    );
    sessions
        .submit(ENTITY, PLAYER_ID, Box::new(Parked), &env)
        .await;
    assert_eq!(sessions.pending(ENTITY), 2);

    // Return to character select (`Disconnect = 0`).
    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[0u8],
        &None,
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
    )
    .await
    .expect("logOff");

    assert_eq!(sessions.pending(ENTITY), 0, "logOff drops the queue");
}
