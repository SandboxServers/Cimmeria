//! PR #1246 review, finding 3a: a gate transfer still in flight when a
//! relaunched client takes its address over must not touch the new session.
//!
//! The SGW client binds a fixed UDP port, so a client killed mid-transfer
//! and relaunched comes back on the same address, and its fresh login
//! replaces the session there (`relaunch_takeover`). Gate travel resolved
//! the address before awaiting the cell, so after the await the session at
//! that address can be the new one. These pin that the abandon path and
//! the world-entry store leave it alone. Each fails with its ownership
//! check (`session_owner`) removed.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::TestTransport;
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 42;

/// The relaunched client's session: logged in, no character yet.
fn relaunched_session() -> ConnectedClientState {
    let mut s = make_state();
    s.player_entity_id = None;
    s.pending_player_entity_id = None;
    s.pending_world_entry = None;
    s.world_name = None;
    s
}

#[tokio::test]
async fn abandon_leaves_a_session_that_replaced_the_travelling_one() {
    let addr: SocketAddr = "127.0.0.1:55801".parse().unwrap();
    let new_session = relaunched_session();
    let new_flag = Arc::clone(&new_session.cancelled);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, new_session)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    abandon_unspaced_session(addr, ENTITY_ID, &connected, &entity_to_addr, &None).await;

    assert!(
        connected.lock().unwrap().contains_key(&addr),
        "the relaunched client's session must not be removed"
    );
    assert!(
        !new_flag.load(Ordering::Relaxed),
        "nor its tick loop cancelled"
    );
}

/// The session is replaced while gate travel waits on the cell's
/// `CreateEntity`. The transfer must finish without writing the old
/// character's world entry onto the new session or sending it the old
/// session's RESET_ENTITIES.
#[tokio::test]
async fn a_transfer_finishing_after_its_session_was_replaced_writes_nothing_onto_it() {
    let addr: SocketAddr = "127.0.0.1:55802".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, make_state())])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let test_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = test_transport.clone();
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let (c, e) = (Arc::clone(&connected), Arc::clone(&entity_to_addr));
    let task = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            "Harset",
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

    // The relaunch takes the address over while the cell is busy.
    connected.lock().unwrap().insert(addr, relaunched_session());
    let _ = reply_tx.send(0x0001_0001);
    timeout(Duration::from_secs(2), task)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel returns cleanly");

    let clients = connected.lock().unwrap();
    let s = &clients[&addr];
    assert!(
        s.pending_world_entry.is_none() && s.pending_player_entity_id.is_none(),
        "the old character's world entry must not land on the new session"
    );
    assert_eq!(s.world_name, None, "nor its destination world");
    assert!(
        test_transport.filter_to(addr).is_empty(),
        "the old session's RESET_ENTITIES must not reach the relaunched client"
    );
}
