//! Concurrency and wire guards for one captured Base recipient.

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cimmeria_mercury::consts::PACKET_MAX_SIZE;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_FRAGMENTED};
use cimmeria_mercury::transport::Transport;
use tokio::sync::{oneshot, Semaphore};

use super::{
    send_to_session_bound_reliable, snapshot_recipient, ConnectedClientState, WitnessSendOutcome,
};
use crate::mercury::build_player_entity_method_packet;
use crate::test_support::TestTransport;

const ENTITY: u32 = 77;
const PLAYER: i32 = 1;
const METHOD: u16 = cimmeria_wire::cell::dispatch::CLIENT_MG_ON_START_MINIGAME;

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityMap = Arc<Mutex<HashMap<u32, SocketAddr>>>;

fn setup() -> (Connected, EntityMap, SocketAddr) {
    let addr = "127.0.0.1:46000".parse().unwrap();
    let mut owner = crate::test_support::test_default_connected_client_state();
    owner.player_entity_id = Some(ENTITY);
    owner.active_player_id = Some(PLAYER);
    owner.key = [3; 32];
    owner.enc = MercuryEncryption::from_session_key(owner.key);
    owner.pending_acks.lock().unwrap().push(41);
    (
        Arc::new(Mutex::new(HashMap::from([(addr, owner)]))),
        Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
        addr,
    )
}

fn replacement() -> ConnectedClientState {
    let mut state = crate::test_support::test_default_connected_client_state();
    state.player_entity_id = Some(ENTITY);
    state.active_player_id = Some(PLAYER);
    state.key = [9; 32];
    state.enc = MercuryEncryption::from_session_key(state.key);
    state.pending_acks.lock().unwrap().push(91);
    state
}

fn url_args(url: &str) -> Vec<u8> {
    let utf16: Vec<u16> = url.encode_utf16().collect();
    let mut args = Vec::with_capacity(4 + utf16.len() * 2);
    args.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
    for unit in utf16 {
        args.extend_from_slice(&unit.to_le_bytes());
    }
    args
}

fn build(
    key: &[u8; 32],
    version: cimmeria_mercury::encryption::EncryptionVersion,
    seq: u32,
    acks: &[u32],
    args: &[u8],
) -> Vec<u8> {
    build_player_entity_method_packet(key, seq, acks, ENTITY, METHOD, args, version)
}

fn assert_untouched(state: &ConnectedClientState) {
    assert_eq!(state.next_seq.load(Ordering::Relaxed), 0);
    assert_eq!(*state.pending_acks.lock().unwrap(), [91]);
    assert!(state.channel.lock().unwrap().tx_window.is_empty());
    assert!(state.channel.lock().unwrap().unsent_packets.is_empty());
}

#[tokio::test]
async fn session_bound_takeover_before_reservation_never_uses_replacement_key() {
    let (connected, entities, addr) = setup();
    let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
    let transport = Arc::new(TestTransport::new());
    let first = AtomicBool::new(true);
    let args = url_args("http://unused/127.0.0.1/9339/Livewire/77/ticket");
    let outcome = send_to_session_bound_reliable(
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entities,
        snapshot,
        "minigame_start",
        |key, version, seq, acks| {
            if first.swap(false, Ordering::Relaxed) {
                connected.lock().unwrap().insert(addr, replacement());
            }
            build(key, version, seq, acks, &args)
        },
    )
    .await;
    assert_eq!(outcome, WitnessSendOutcome::ClientDisconnected);
    assert!(transport.is_empty());
    assert_untouched(&connected.lock().unwrap()[&addr]);
}

#[tokio::test]
async fn session_bound_takeover_before_preparation_refuses_the_new_session() {
    let (connected, entities, addr) = setup();
    let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
    connected.lock().unwrap().insert(addr, replacement());
    let transport = Arc::new(TestTransport::new());
    let outcome = send_to_session_bound_reliable(
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entities,
        snapshot,
        "minigame_start",
        |key, version, seq, acks| build(key, version, seq, acks, &url_args("old")),
    )
    .await;
    assert_eq!(outcome, WitnessSendOutcome::ClientDisconnected);
    assert!(transport.is_empty());
    assert_untouched(&connected.lock().unwrap()[&addr]);
}

#[tokio::test]
async fn session_bound_character_switch_before_reservation_sends_no_old_url() {
    let (connected, entities, addr) = setup();
    let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
    let transport = Arc::new(TestTransport::new());
    let first = AtomicBool::new(true);
    let outcome = send_to_session_bound_reliable(
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entities,
        snapshot,
        "minigame_start",
        |key, version, seq, acks| {
            if first.swap(false, Ordering::Relaxed) {
                connected
                    .lock()
                    .unwrap()
                    .get_mut(&addr)
                    .unwrap()
                    .active_player_id = Some(2);
            }
            build(key, version, seq, acks, &url_args("old"))
        },
    )
    .await;
    assert_eq!(outcome, WitnessSendOutcome::ClientDisconnected);
    assert!(transport.is_empty());
    let clients = connected.lock().unwrap();
    let owner = &clients[&addr];
    assert_eq!(owner.next_seq.load(Ordering::Relaxed), 0);
    assert_eq!(*owner.pending_acks.lock().unwrap(), [41]);
    assert!(owner.channel.lock().unwrap().tx_window.is_empty());
}

#[tokio::test]
async fn session_bound_character_switch_before_close_reservation_sends_no_close() {
    let (connected, entities, addr) = setup();
    let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
    let transport = Arc::new(TestTransport::new());
    let first = AtomicBool::new(true);
    let outcome = send_to_session_bound_reliable(
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entities,
        snapshot,
        "minigame_end",
        |key, version, seq, acks| {
            if first.swap(false, Ordering::Relaxed) {
                connected
                    .lock()
                    .unwrap()
                    .get_mut(&addr)
                    .unwrap()
                    .active_player_id = Some(2);
            }
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                ENTITY,
                cimmeria_wire::cell::dispatch::CLIENT_MG_ON_END_MINIGAME,
                &[],
                version,
            )
        },
    )
    .await;
    assert_eq!(outcome, WitnessSendOutcome::ClientDisconnected);
    assert!(transport.is_empty());
    let clients = connected.lock().unwrap();
    let owner = &clients[&addr];
    assert_eq!(owner.next_seq.load(Ordering::Relaxed), 0);
    assert_eq!(*owner.pending_acks.lock().unwrap(), [41]);
    assert!(owner.channel.lock().unwrap().tx_window.is_empty());
}

#[derive(Debug)]
struct PausedTransport {
    sent: Mutex<Vec<Vec<u8>>>,
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Semaphore,
    pause_at: usize,
    send_count: AtomicUsize,
}

impl Transport for PausedTransport {
    fn send_to<'a, 'b, 'f>(
        &'a self,
        bytes: &'b [u8],
        _addr: SocketAddr,
    ) -> Pin<Box<dyn Future<Output = io::Result<usize>> + Send + 'f>>
    where
        'a: 'f,
        'b: 'f,
        Self: 'f,
    {
        Box::pin(async move {
            if self.send_count.fetch_add(1, Ordering::Relaxed) == self.pause_at {
                if let Some(entered) = self.entered.lock().unwrap().take() {
                    let _ = entered.send(());
                }
                self.release.acquire().await.unwrap().forget();
            }
            self.sent.lock().unwrap().push(bytes.to_vec());
            Ok(bytes.len())
        })
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

#[tokio::test]
async fn session_bound_takeover_during_send_keeps_old_ciphertext_and_new_window_empty() {
    let (connected, entities, addr) = setup();
    let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
    let args = url_args("http://unused/127.0.0.1/9339/Livewire/77/ticket");
    let (entered_tx, entered_rx) = oneshot::channel();
    let transport = Arc::new(PausedTransport {
        sent: Mutex::new(Vec::new()),
        entered: Mutex::new(Some(entered_tx)),
        release: Semaphore::new(0),
        pause_at: 0,
        send_count: AtomicUsize::new(0),
    });
    let task = tokio::spawn({
        let connected = connected.clone();
        let entities = entities.clone();
        let transport = transport.clone();
        let args = args.clone();
        async move {
            send_to_session_bound_reliable(
                &(transport as Arc<dyn Transport>),
                &connected,
                &entities,
                snapshot,
                "minigame_start",
                |key, version, seq, acks| build(key, version, seq, acks, &args),
            )
            .await
        }
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), entered_rx)
        .await
        .unwrap()
        .unwrap();
    connected.lock().unwrap().insert(addr, replacement());
    transport.release.add_permits(1);
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap(),
        WitnessSendOutcome::ClientDisconnected
    ));
    assert_untouched(&connected.lock().unwrap()[&addr]);
    let sent = transport.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    let plaintext = MercuryEncryption::from_session_key([3; 32])
        .decrypt(&sent[0])
        .unwrap();
    let parsed = parse_incoming(&plaintext).unwrap();
    assert_eq!(parsed.body, expected_body(&args).as_slice());
    assert!(MercuryEncryption::from_session_key([9; 32])
        .decrypt(&sent[0])
        .is_err());
}

fn expected_body(args: &[u8]) -> Vec<u8> {
    let wire = build(
        &[3; 32],
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        0,
        &[],
        args,
    );
    let plain = MercuryEncryption::from_session_key([3; 32])
        .decrypt(&wire)
        .unwrap();
    parse_incoming(&plain).unwrap().body.to_vec()
}

#[tokio::test]
async fn session_bound_long_url_fragments_and_reassembles_exactly() {
    let (connected, entities, addr) = setup();
    let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
    let transport = Arc::new(TestTransport::new());
    let args = url_args(&format!("http://unused/{}", "a".repeat(4000)));
    let outcome = send_to_session_bound_reliable(
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entities,
        snapshot,
        "minigame_start",
        |key, version, seq, acks| build(key, version, seq, acks, &args),
    )
    .await;
    let WitnessSendOutcome::Sent { packets, seq, .. } = outcome else {
        panic!("{outcome:?}")
    };
    let sent = transport.filter_to(addr);
    assert_eq!(packets, sent.len());
    assert!(packets > 1);
    let mut body = Vec::new();
    for (i, wire) in sent.iter().enumerate() {
        assert!(wire.len() <= PACKET_MAX_SIZE);
        let plain = MercuryEncryption::from_session_key([3; 32])
            .decrypt(wire)
            .unwrap();
        let fragment = parse_incoming(&plain).unwrap();
        assert_ne!(fragment.flags & FLAG_FRAGMENTED, 0);
        assert_eq!(fragment.seq_id, Some(seq + i as u32));
        body.extend_from_slice(&fragment.body);
    }
    assert_eq!(body, expected_body(&args));
    assert_eq!(
        connected.lock().unwrap()[&addr]
            .channel
            .lock()
            .unwrap()
            .tx_window
            .len(),
        packets
    );
}

#[tokio::test]
async fn session_bound_fragment_send_takeover_never_registers_new_channel() {
    for pause_at in [0, 1] {
        let (connected, entities, addr) = setup();
        let snapshot = snapshot_recipient(&connected, &entities, ENTITY, PLAYER).unwrap();
        let args = url_args(&format!("http://unused/{}", "a".repeat(4000)));
        let (entered_tx, entered_rx) = oneshot::channel();
        let transport = Arc::new(PausedTransport {
            sent: Mutex::new(Vec::new()),
            entered: Mutex::new(Some(entered_tx)),
            release: Semaphore::new(0),
            pause_at,
            send_count: AtomicUsize::new(0),
        });
        let task = tokio::spawn({
            let connected = connected.clone();
            let entities = entities.clone();
            let transport = transport.clone();
            let args = args.clone();
            async move {
                send_to_session_bound_reliable(
                    &(transport as Arc<dyn Transport>),
                    &connected,
                    &entities,
                    snapshot,
                    "minigame_start",
                    |key, version, seq, acks| build(key, version, seq, acks, &args),
                )
                .await
            }
        });
        tokio::time::timeout(std::time::Duration::from_secs(2), entered_rx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            connected.lock().unwrap()[&addr]
                .channel
                .lock()
                .unwrap()
                .tx_window
                .len(),
            pause_at,
            "earlier fragments register before takeover"
        );
        connected.lock().unwrap().insert(addr, replacement());
        transport.release.add_permits(1);
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(outcome, WitnessSendOutcome::ClientDisconnected);
        assert_untouched(&connected.lock().unwrap()[&addr]);
        let sent = transport.sent.lock().unwrap();
        assert_eq!(sent.len(), pause_at + 1, "later fragments must stop");
        for (i, wire) in sent.iter().enumerate() {
            assert!(wire.len() <= PACKET_MAX_SIZE);
            let plain = MercuryEncryption::from_session_key([3; 32])
                .decrypt(wire)
                .unwrap();
            let fragment = parse_incoming(&plain).unwrap();
            assert_eq!(fragment.seq_id, Some(i as u32));
            assert_ne!(fragment.flags & FLAG_FRAGMENTED, 0);
            assert!(MercuryEncryption::from_session_key([9; 32])
                .decrypt(wire)
                .is_err());
        }
    }
}
