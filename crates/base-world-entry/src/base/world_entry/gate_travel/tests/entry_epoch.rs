//! Gate travel invalidates a letter turn-in entry before destination awaits.

use super::*;
use cimmeria_wire::cell::messages::LetterTurnInEntryEpoch;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn accepted_same_world_gate_travel_invalidates_letter_entry_before_cell_reply() {
    let addr: SocketAddr = "127.0.0.1:55796".parse().unwrap();
    let mut state = make_state();
    state.extensions.insert(LetterTurnInEntryEpoch::new(7));
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(42, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(crate::test_support::TestTransport::new());
    let (tx, mut rx) = mpsc::channel(8);
    let (clients, entities) = (Arc::clone(&connected), Arc::clone(&entity_to_addr));
    let task = tokio::spawn(async move {
        handle_gate_travel(
            42,
            "Agnos",
            [1.0, 2.0, 3.0],
            [0.0; 3],
            None,
            None,
            &transport,
            &clients,
            &entities,
            &Some(tx),
            &None,
        )
        .await
    });
    let msg = timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("CreateEntity awaited")
        .expect("CreateEntity message");
    let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg else {
        panic!("expected destination create");
    };
    assert!(connected.lock().unwrap()[&addr]
        .extensions
        .get::<LetterTurnInEntryEpoch>()
        .is_none());
    let _ = reply_tx.send(0x0001_0001);
    timeout(Duration::from_secs(2), task)
        .await
        .expect("gate travel completion")
        .unwrap()
        .unwrap();
}
