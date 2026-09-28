//! #898: gate travel moves `ConnectedClientState.world_name` to the
//! destination. Before the fix only `playCharacter` wrote it, so after any
//! trip the admin online-player `zone`, the next trip's Discord world-exit
//! origin (snapshotted from it at the top of `handle_gate_travel`) and the
//! crafting completion's world guard all saw the session's first world.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 42;

/// One trip through `handle_gate_travel`, answering the cell's
/// `CreateEntity` with a space id.
async fn travel(
    world: &'static str,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(crate::test_support::TestTransport::new());
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);
    let (c, e) = (Arc::clone(connected), Arc::clone(entity_to_addr));
    let task = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            world,
            [1.0, 2.0, 3.0],
            [0.0; 3],
            None,
            None,
            &transport,
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
    let _ = reply_tx.send(0x0001_0001);
    timeout(Duration::from_secs(2), task)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel completes");
}

/// Agnos -> Castle_CellBlock -> Harset. After each trip the session's
/// `world_name` is the destination, already when the handler returns, which
/// is before the client can answer RESET_ENTITIES and send `onClientReady`
/// for the new world (the pending world entry is what it will enter).
/// Fails with the update removed: `world_name` stays "Agnos".
#[tokio::test]
async fn gate_travel_moves_the_session_world_name_to_the_destination() {
    let addr: SocketAddr = "127.0.0.1:55795".parse().unwrap();
    let state = make_state();
    assert_eq!(state.world_name.as_deref(), Some("Agnos"));
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));

    for world in ["Castle_CellBlock", "Harset"] {
        travel(world, &connected, &entity_to_addr).await;
        let clients = connected.lock().unwrap();
        let c = &clients[&addr];
        assert_eq!(
            c.world_name.as_deref(),
            Some(world),
            "after the trip to {world}"
        );
        assert_eq!(
            c.pending_world_entry
                .as_ref()
                .map(|e| e.world_name.as_str()),
            Some(world),
            "the client has not entered {world} yet"
        );
    }
}
