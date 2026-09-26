//! Client→server reliable ordering on the live receive path (NA38).
//!
//! `handle_encrypted_datagram` runs every decrypted client packet through
//! the session Channel's receive gate before dispatch. These guards drive
//! real encrypted datagrams through it and watch what reaches the cell.
//! `REQUEST_ENTITY_UPDATE` (msg 0x07) is the probe: the base forwards it
//! to the cell verbatim, so the cell receiver shows exactly which client
//! messages were dispatched, and in what order.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::test_default_connected_client_state;

use super::handle_encrypted_datagram;

const WITNESS_ID: u32 = 7;

struct Rig {
    addr: SocketAddr,
    state_key: [u8; 32],
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pending_acks: Arc<Mutex<Vec<u32>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// A logged-in, in-world session whose channel is anchored at seq 0, the
/// way `handle_login` leaves it.
fn rig() -> Rig {
    let addr: SocketAddr = "127.0.0.1:40001".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(WITNESS_ID);
    state.channel.lock().unwrap().anchor_rx_seq(0);
    let state_key = state.key;
    let pending_acks = Arc::clone(&state.pending_acks);
    let mut map = HashMap::new();
    map.insert(addr, state);
    let (cell_tx, cell_rx) = mpsc::channel(64);
    Rig {
        addr,
        state_key,
        transport: Arc::new(TestTransport::new()),
        connected: Arc::new(Mutex::new(map)),
        pending_acks,
        entity_manager: Arc::new(Mutex::new(EntityManager::new())),
        cell_tx: Some(cell_tx),
        cell_rx,
        entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
    }
}

/// One encrypted reliable client packet carrying a single
/// `REQUEST_ENTITY_UPDATE` for `probe_id`.
fn reliable_probe(rig: &Rig, seq: u32, probe_id: u32) -> Vec<u8> {
    let mut payload = 0u32.to_le_bytes().to_vec(); // header word
    payload.extend_from_slice(&probe_id.to_le_bytes());
    let mut body = vec![0x07];
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);
    let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
    let plain = build_outgoing(flags, &body, Some(seq), &[], None);
    let enc = rig.connected.lock().unwrap()[&rig.addr].enc.clone();
    enc.encrypt(&plain).unwrap()
}

async fn deliver(rig: &Rig, datagram: &[u8]) {
    let enc = rig.connected.lock().unwrap()[&rig.addr].enc.clone();
    handle_encrypted_datagram(
        &rig.transport,
        rig.addr,
        datagram,
        enc,
        rig.state_key,
        0,
        &rig.pending_acks,
        &rig.connected,
        &None,
        &None,
        &rig.entity_manager,
        &rig.cell_tx,
        &rig.entity_to_addr,
    )
    .await
    .unwrap();
}

/// Every probe id the cell has been handed so far, in dispatch order.
fn dispatched(rig: &mut Rig) -> Vec<u32> {
    let mut ids = Vec::new();
    while let Ok(msg) = rig.cell_rx.try_recv() {
        if let BaseToCellMsg::RequestEntityUpdate { entity_ids, .. } = msg {
            ids.extend(entity_ids);
        }
    }
    ids
}

#[tokio::test]
async fn a_client_packet_behind_a_gap_waits_for_the_retransmit() {
    let mut rig = rig();

    deliver(&rig, &reliable_probe(&rig, 0, 100)).await;
    assert_eq!(dispatched(&mut rig), vec![100]);

    // seq 1 is lost; seq 2 must not jump ahead of it.
    deliver(&rig, &reliable_probe(&rig, 2, 102)).await;
    assert_eq!(
        dispatched(&mut rig),
        Vec::<u32>::new(),
        "seq 2 dispatched before the missing seq 1"
    );

    // The client's retransmit of seq 1 releases both, in order.
    deliver(&rig, &reliable_probe(&rig, 1, 101)).await;
    assert_eq!(dispatched(&mut rig), vec![101, 102]);

    assert_eq!(
        *rig.pending_acks.lock().unwrap(),
        vec![0, 2, 1],
        "every accepted reliable packet is acked, the buffered one included"
    );
}

#[tokio::test]
async fn a_retransmitted_duplicate_is_acked_but_not_dispatched_twice() {
    let mut rig = rig();

    // Our ACK for seq 0 was lost, so the client sends it again.
    let packet = reliable_probe(&rig, 0, 200);
    deliver(&rig, &packet).await;
    deliver(&rig, &packet).await;

    assert_eq!(
        dispatched(&mut rig),
        vec![200],
        "a retransmitted client message must run exactly once"
    );
    assert_eq!(
        *rig.pending_acks.lock().unwrap(),
        vec![0, 0],
        "the duplicate is re-acked so the client stops resending it"
    );
}
