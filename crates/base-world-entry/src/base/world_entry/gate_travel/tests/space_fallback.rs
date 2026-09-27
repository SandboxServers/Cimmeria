//! The space fallback taken when the cell does not answer `CreateEntity`.
//!
//! For most worlds a lost reply falls back to a hardcoded space id. The
//! historical CellBlock worlds (1201–1207) have no safe one: the table's
//! unknown-world default is the stock `Castle_CellBlock` space. A transfer to
//! one of them must end the session instead of sending a world entry that
//! binds the client to the stock space.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::TestTransport;
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 42;

struct Run {
    sent: Arc<TestTransport>,
    addr: SocketAddr,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    /// Everything the base told the cell after the dropped reply.
    after_create: Vec<BaseToCellMsg>,
}

/// Gate-travel `ENTITY_ID` to `world`, with the cell dropping the
/// `CreateEntity` reply the way it does when the create fails.
async fn travel_with_failed_cell_create(world: &'static str, port: u16) -> Run {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, make_state())])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let sent = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = sent.clone();
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let task_connected = Arc::clone(&connected);
    let handle = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            world,
            [-334.231, 73.472, -228.026],
            [0.0; 3],
            None,
            None,
            &transport,
            &task_connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
        )
        .await
    });

    let msg = timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity {
        world_name,
        reply_tx,
        ..
    } = msg
    else {
        panic!("expected CreateEntity as the first base->cell message");
    };
    assert_eq!(world_name, world);
    drop(reply_tx);

    timeout(Duration::from_secs(2), handle)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel returns Ok");

    let mut after_create = Vec::new();
    while let Ok(msg) = cell_rx.try_recv() {
        after_create.push(msg);
    }
    Run {
        sent,
        addr,
        connected,
        after_create,
    }
}

/// Regression shape: with `resolve_space_id_fallback` answering the stock
/// default for `CellBlock43`, the transfer carries on, sends
/// `RESET_ENTITIES` and queues a world entry for space 65552 while the cell
/// holds the entity in no space.
#[tokio::test]
async fn historical_cellblock_transfer_fails_closed_when_the_cell_create_fails() {
    let run = travel_with_failed_cell_create("CellBlock43", 55801).await;

    assert!(
        run.connected.lock().unwrap().get(&run.addr).is_none(),
        "the session must end rather than enter a space the entity is not in"
    );
    assert!(
        run.sent.is_empty(),
        "no RESET_ENTITIES or world entry may reach the client"
    );
    assert!(
        matches!(
            run.after_create.as_slice(),
            [BaseToCellMsg::DisconnectEntity {
                entity_id: ENTITY_ID
            }]
        ),
        "the cell must only hear the session teardown"
    );
}

/// Control: a stock world still takes its fallback space, so the refusal is
/// scoped to the worlds that have no safe one.
#[tokio::test]
async fn stock_cellblock_transfer_keeps_its_fallback_space_when_the_cell_create_fails() {
    let run = travel_with_failed_cell_create("Castle_CellBlock", 55802).await;

    let map = run.connected.lock().unwrap();
    let entry = map
        .get(&run.addr)
        .expect("session stays up")
        .pending_world_entry
        .as_ref()
        .expect("world entry queued");
    assert_eq!(entry.space_id, crate::mercury::DEFAULT_SPACE_ID);
    assert_eq!(entry.world_name, "Castle_CellBlock");
}
