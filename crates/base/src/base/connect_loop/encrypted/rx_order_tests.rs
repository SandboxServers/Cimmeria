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
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::packet::{build_outgoing, FLAG_HAS_SEQUENCE, FLAG_ON_CHANNEL, FLAG_RELIABLE};
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use tracing::Level;

use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{test_default_connected_client_state, LogCapture};
use cimmeria_base_session::base::helpers::{
    derive_letter_turn_in_token, send_letter_turn_in_method, ExpectedLetterTurnInEntry,
    LetterTurnInSendOutcome,
};
use cimmeria_wire::cell::messages::{
    LetterTurnInAttempt, LetterTurnInEntryEpoch, LetterTurnInRequest,
};

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

/// A logged-in, in-world session whose channel is built exactly as
/// `handle_login` builds it (`new_client_channel`).
fn rig() -> Rig {
    let addr: SocketAddr = "127.0.0.1:40001".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(WITNESS_ID);
    state.channel = Mutex::new(crate::base::login::new_client_channel(addr));
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
///
/// Wire (corrected 2026-09-28, issue #838): `[u32 entityId][N × u32
/// cacheStamp]`, N = 0 here -- exactly what the real client sends.
fn reliable_probe(rig: &Rig, seq: u32, probe_id: u32) -> Vec<u8> {
    let payload = probe_id.to_le_bytes().to_vec();
    let mut body = vec![0x07];
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(&payload);
    let flags = FLAG_ON_CHANNEL | FLAG_HAS_SEQUENCE | FLAG_RELIABLE;
    let plain = build_outgoing(flags, &body, Some(seq), &[], None);
    let enc = rig.connected.lock().unwrap()[&rig.addr].enc.clone();
    enc.encrypt(&plain).unwrap()
}

/// A raw `REQUEST_ENTITY_UPDATE` body shorter than the 4-byte entity id --
/// the byte-exact `payload_too_short` regression guard for issue #838's
/// negative-log requirement.
fn malformed_probe(rig: &Rig, seq: u32) -> Vec<u8> {
    let payload: &[u8] = &[0xAB]; // 1 byte -- too short for a u32 entity id
    let mut body = vec![0x07];
    body.extend_from_slice(&(payload.len() as u16).to_le_bytes());
    body.extend_from_slice(payload);
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

#[tokio::test]
async fn a_client_whose_reliable_stream_starts_at_1234_is_dispatched_normally() {
    // A session channel must adopt the client's first reliable sequence.
    // Pinned at 0, seq 1234 would be 1234 slots ahead: past the 512 window,
    // dropped unacked, and the login would hang.
    let mut rig = rig();

    deliver(&rig, &reliable_probe(&rig, 1234, 300)).await;
    assert_eq!(
        dispatched(&mut rig),
        vec![300],
        "the first reliable packet must run whatever its sequence"
    );
    deliver(&rig, &reliable_probe(&rig, 1236, 302)).await;
    deliver(&rig, &reliable_probe(&rig, 1235, 301)).await;
    assert_eq!(dispatched(&mut rig), vec![301, 302]);
    assert_eq!(*rig.pending_acks.lock().unwrap(), vec![1234, 1236, 1235]);
}

/// #838 negative-log guard: a `REQUEST_ENTITY_UPDATE` body shorter than the
/// 4-byte entity id must log a WARN with `reason = "payload_too_short"`, not
/// a silent DEBUG. This must fail if the arm reverts to the pre-fix
/// `tracing::debug!` no-op.
#[tokio::test]
async fn a_too_short_payload_logs_a_negative_event_not_a_silent_drop() {
    let mut rig = rig();
    let guard = LogCapture::install();

    deliver(&rig, &malformed_probe(&rig, 0)).await;

    let row = guard
        .find_event(
            Level::WARN,
            "payload shorter than the 4-byte entity id",
            "payload_too_short",
        )
        .unwrap_or_else(|| panic!("no payload_too_short row; saw {:#?}", guard.all()));
    assert!(row.has_field("payload_len", "1"));

    assert!(
        dispatched(&mut rig).is_empty(),
        "a malformed payload must not reach the cell"
    );
}

struct PartialWire;

impl Transport for PartialWire {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _: &'life1 [u8],
        _: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async { panic!("letter admission must be synchronous") })
    }

    fn try_send_to(&self, bytes: &[u8], _: SocketAddr) -> std::io::Result<usize> {
        Ok(bytes.len() - 1)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

async fn assert_fatal_letter_admission_blocks_method_and_tears_down(full_queue: bool) {
    let mut rig = rig();
    let player_id = 40_001;
    let (expected, cancelled, unreliable, pending, last_recv, key, version) = {
        let mut clients = rig.connected.lock().unwrap();
        let state = clients.get_mut(&rig.addr).unwrap();
        state.active_player_id = Some(player_id);
        state.world_name = Some("Harset_CmdCenter".to_owned());
        let epoch = LetterTurnInEntryEpoch::new(1);
        state.extensions.insert(epoch);
        let request = LetterTurnInRequest {
            entity_id: WITNESS_ID,
            player_id,
            attempt: LetterTurnInAttempt::from_bytes([4; 16]),
            token: derive_letter_turn_in_token(&state.key),
            generation: state.connected_at,
            epoch,
        };
        (
            ExpectedLetterTurnInEntry::capture(rig.addr, state, &request).unwrap(),
            Arc::clone(&state.cancelled),
            Arc::clone(&state.next_seq_unreliable),
            Arc::clone(&state.pending_acks),
            Arc::clone(&state.last_recv),
            state.key,
            state.enc_version,
        )
    };
    rig.entity_to_addr
        .lock()
        .unwrap()
        .insert(WITNESS_ID, rig.addr);
    if full_queue {
        use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags};
        let clients = rig.connected.lock().unwrap();
        let mut channel = clients[&rig.addr].channel.lock().unwrap();
        for seq in 1..=(cimmeria_mercury::consts::TX_WINDOW_SIZE
            + cimmeria_mercury::consts::MAX_UNSENT_PACKETS)
        {
            channel
                .register_sent_packet(
                    Packet::new(PacketFlags::default(), seq as u32, Bytes::new()),
                    Bytes::new(),
                )
                .unwrap();
        }
    }
    let admission: Arc<dyn Transport> = if full_queue {
        Arc::clone(&rig.transport)
    } else {
        Arc::new(PartialWire)
    };
    assert_eq!(
        send_letter_turn_in_method(
            &expected,
            71,
            &[0; 8],
            &admission,
            &rig.connected,
            &rig.entity_to_addr,
        ),
        LetterTurnInSendOutcome::ChannelError
    );
    assert!(cancelled.load(Ordering::Acquire));
    let prior_recv = Instant::now() - Duration::from_secs(30);
    *last_recv.lock().unwrap() = prior_recv;
    let valid_method = reliable_probe(&rig, 0, 99);
    deliver(&rig, &valid_method).await;
    assert!(
        dispatched(&mut rig).is_empty(),
        "cancelled owner cannot dispatch game method"
    );
    assert!(
        pending.lock().unwrap().is_empty(),
        "receive gate must not ACK it"
    );
    assert_eq!(
        *last_recv.lock().unwrap(),
        prior_recv,
        "cancelled packet cannot refresh liveness"
    );

    tokio::time::timeout(
        Duration::from_secs(2),
        crate::base::tick_sync::run_tick_loop(
            Arc::clone(&rig.transport),
            rig.addr,
            key,
            version,
            unreliable,
            pending,
            last_recv,
            cancelled,
            Arc::clone(&rig.connected),
            Arc::clone(&rig.entity_manager),
            rig.cell_tx.clone(),
            Arc::clone(&rig.entity_to_addr),
            None,
        ),
    )
    .await
    .expect("cancelled owner tick must exit");
    assert!(!rig.connected.lock().unwrap().contains_key(&rig.addr));
    assert!(!rig.entity_to_addr.lock().unwrap().contains_key(&WITNESS_ID));
    let msg = tokio::time::timeout(Duration::from_secs(2), rig.cell_rx.recv())
        .await
        .unwrap()
        .unwrap();
    let BaseToCellMsg::DisconnectEntity {
        entity_id,
        reply_tx,
    } = msg
    else {
        panic!("expected exact cell disconnect");
    };
    assert_eq!(entity_id, WITNESS_ID);
    reply_tx.send(()).unwrap();
}

#[tokio::test]
async fn partial_letter_admission_blocks_encrypted_method_then_tick_tears_down_owner() {
    assert_fatal_letter_admission_blocks_method_and_tears_down(false).await;
}

#[tokio::test]
async fn unregistered_admitted_letter_blocks_encrypted_method_then_tick_tears_down_owner() {
    assert_fatal_letter_admission_blocks_method_and_tears_down(true).await;
}

#[tokio::test]
async fn stale_encrypted_packet_cannot_dispatch_into_same_key_address_replacement() {
    let mut rig = rig();
    let packet = reliable_probe(&rig, 0, 100);
    let (old_enc, old_pending) = {
        let clients = rig.connected.lock().unwrap();
        (
            clients[&rig.addr].enc.clone(),
            Arc::clone(&clients[&rig.addr].pending_acks),
        )
    };
    let mut replacement = test_default_connected_client_state();
    replacement.player_entity_id = Some(WITNESS_ID);
    replacement.channel = Mutex::new(crate::base::login::new_client_channel(rig.addr));
    rig.connected.lock().unwrap().insert(rig.addr, replacement);
    handle_encrypted_datagram(
        &rig.transport,
        rig.addr,
        &packet,
        old_enc,
        rig.state_key,
        0,
        &old_pending,
        &rig.connected,
        &None,
        &None,
        &rig.entity_manager,
        &rig.cell_tx,
        &rig.entity_to_addr,
    )
    .await
    .unwrap();
    assert!(dispatched(&mut rig).is_empty());
    assert!(old_pending.lock().unwrap().is_empty());
    assert!(rig.connected.lock().unwrap().contains_key(&rig.addr));
}

#[tokio::test]
async fn cancellation_after_channel_delivery_still_blocks_method_dispatch() {
    let mut rig = rig();
    let raw = reliable_probe(&rig, 0, 321);
    let plain = rig.connected.lock().unwrap()[&rig.addr]
        .enc
        .decrypt(&raw)
        .unwrap();
    let parsed = cimmeria_mercury::packet::parse_incoming(&plain).unwrap();
    let (delivery, class_id) =
        super::receive_in_order(&rig.connected, rig.addr, &rig.pending_acks, parsed, 0).unwrap();
    rig.connected.lock().unwrap()[&rig.addr]
        .cancelled
        .store(true, Ordering::Release);
    super::dispatch_client_bundle(
        &delivery.bundles[0],
        delivery.bundle_seqs[0],
        class_id,
        &rig.transport,
        rig.addr,
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
    assert!(dispatched(&mut rig).is_empty());
}
