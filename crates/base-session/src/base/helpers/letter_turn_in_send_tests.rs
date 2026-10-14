use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::{
    LetterTurnInAttempt, LetterTurnInEntryEpoch, LetterTurnInRequest,
};

use super::letter_turn_in_send::send_with_probe_hook;
use super::{
    derive_letter_turn_in_token, send_letter_turn_in_method, ExpectedLetterTurnInEntry,
    LetterTurnInSendOutcome,
};
use crate::test_support::{test_default_connected_client_state, TestTransport};

const ENTITY: u32 = 6541;
const PLAYER: i32 = 6542;

struct Rig {
    addr: SocketAddr,
    connected: Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>,
    mapping: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    expected: ExpectedLetterTurnInEntry,
    recorder: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
}

fn rig() -> Rig {
    let addr = "127.0.0.1:56541".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.key = [9; 32];
    state.enc = MercuryEncryption::from_session_key(state.key);
    state.player_entity_id = Some(ENTITY);
    state.active_player_id = Some(PLAYER);
    state.world_name = Some("Harset_CmdCenter".to_owned());
    state.extensions.insert(LetterTurnInEntryEpoch::new(1));
    let request = LetterTurnInRequest {
        entity_id: ENTITY,
        player_id: PLAYER,
        attempt: LetterTurnInAttempt::from_bytes([3; 16]),
        token: derive_letter_turn_in_token(&state.key),
        generation: state.connected_at,
        epoch: LetterTurnInEntryEpoch::new(1),
    };
    let expected = ExpectedLetterTurnInEntry::capture(addr, &state, &request).unwrap();
    let recorder = Arc::new(TestTransport::default());
    Rig {
        addr,
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        mapping: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
        expected,
        transport: recorder.clone(),
        recorder,
    }
}

#[test]
fn fixed_domain_matches_world_entry_vector_and_debug_redacts() {
    let token = derive_letter_turn_in_token(&[9; 32]);
    assert_eq!(
        token.bytes(),
        [
            0xc7, 0xd4, 0xa0, 0x1c, 0x23, 0x52, 0x49, 0x2e, 0x4b, 0xe6, 0xb9, 0x8e, 0xa0, 0xa0,
            0x89, 0xec
        ]
    );
    assert_eq!(format!("{token:?}"), "LetterTurnInToken([REDACTED])");
    assert_eq!(
        format!("{:?}", LetterTurnInAttempt::from_bytes([3; 16])),
        "LetterTurnInAttempt([REDACTED])"
    );
}

#[test]
fn sender_registers_only_on_the_captured_session() {
    let rig = rig();
    let sent = send_letter_turn_in_method(
        &rig.expected,
        71,
        &[1, 0, 0, 0, 3, 0, 0, 0],
        &rig.transport,
        &rig.connected,
        &rig.mapping,
    );
    assert_eq!(sent, LetterTurnInSendOutcome::Sent);
    let packets = rig.recorder.filter_to(rig.addr);
    assert_eq!(packets.len(), 1);
    let clients = rig.connected.lock().unwrap();
    let channel = clients[&rig.addr].channel.lock().unwrap();
    assert_eq!(channel.tx_window.len(), 1);
    assert_eq!(channel.tx_window[0].raw_bytes.as_ref(), packets[0]);
}

#[test]
fn replacement_epoch_and_mapping_never_receive_old_method() {
    for change in 0..5 {
        let rig = rig();
        match change {
            0 => {
                rig.connected
                    .lock()
                    .unwrap()
                    .get_mut(&rig.addr)
                    .unwrap()
                    .key = [8; 32]
            }
            1 => {
                rig.connected
                    .lock()
                    .unwrap()
                    .get_mut(&rig.addr)
                    .unwrap()
                    .extensions
                    .insert(LetterTurnInEntryEpoch::new(2));
            }
            2 => {
                rig.connected
                    .lock()
                    .unwrap()
                    .get_mut(&rig.addr)
                    .unwrap()
                    .active_player_id = Some(PLAYER + 1)
            }
            3 => {
                rig.mapping.lock().unwrap().remove(&ENTITY);
            }
            _ => {
                let mut replacement = test_default_connected_client_state();
                replacement.key = [9; 32];
                replacement.connected_at = rig.expected.generation;
                replacement.player_entity_id = Some(ENTITY);
                replacement.active_player_id = Some(PLAYER);
                replacement.world_name = Some("Harset_CmdCenter".to_owned());
                replacement
                    .extensions
                    .insert(LetterTurnInEntryEpoch::new(1));
                rig.connected.lock().unwrap().insert(rig.addr, replacement);
            }
        }
        assert_eq!(
            send_letter_turn_in_method(
                &rig.expected,
                71,
                &[0; 8],
                &rig.transport,
                &rig.connected,
                &rig.mapping
            ),
            LetterTurnInSendOutcome::Stale,
            "change {change}"
        );
        assert!(rig.recorder.filter_to(rig.addr).is_empty());
    }
}

#[test]
fn oversized_method_is_rejected_before_sequence_admission() {
    let rig = rig();
    assert_eq!(
        send_letter_turn_in_method(
            &rig.expected,
            72,
            &[0; 1600],
            &rig.transport,
            &rig.connected,
            &rig.mapping
        ),
        LetterTurnInSendOutcome::Oversized
    );
    assert!(rig.recorder.filter_to(rig.addr).is_empty());
    assert!(rig.connected.lock().unwrap()[&rig.addr]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
}

#[test]
fn same_key_epoch_invalidation_before_admission_sends_and_registers_nothing() {
    let rig = rig();
    let outcome = send_with_probe_hook(
        &rig.expected,
        71,
        &[0; 8],
        &rig.transport,
        &rig.connected,
        &rig.mapping,
        || {
            // The same address/entity/player with a new epoch must not lend
            // its reliable sequence to a packet built for the old entry.
            rig.connected
                .lock()
                .unwrap()
                .get_mut(&rig.addr)
                .unwrap()
                .extensions
                .insert(LetterTurnInEntryEpoch::new(2));
        },
    );
    assert_eq!(outcome, LetterTurnInSendOutcome::Stale);
    assert!(rig.recorder.filter_to(rig.addr).is_empty());
    let clients = rig.connected.lock().unwrap();
    assert_eq!(
        clients[&rig.addr]
            .next_seq
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
    assert!(clients[&rig.addr]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
}

struct WouldBlockOnceTransport {
    blocked: AtomicBool,
    recorder: Arc<TestTransport>,
}

impl Transport for WouldBlockOnceTransport {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        bytes: &'life1 [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.recorder.send_to(bytes, _addr).await })
    }

    fn try_send_to(&self, bytes: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        if self.blocked.swap(false, Ordering::AcqRel) {
            return Err(std::io::ErrorKind::WouldBlock.into());
        }
        self.recorder.try_send_to(bytes, addr)
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

#[test]
fn would_block_has_no_packet_or_registration_and_same_attempt_can_retry() {
    let rig = rig();
    let transport: Arc<dyn Transport> = Arc::new(WouldBlockOnceTransport {
        blocked: AtomicBool::new(true),
        recorder: rig.recorder.clone(),
    });
    assert_eq!(
        send_letter_turn_in_method(
            &rig.expected,
            71,
            &[0; 8],
            &transport,
            &rig.connected,
            &rig.mapping
        ),
        LetterTurnInSendOutcome::TransportError
    );
    assert!(rig.recorder.is_empty());
    assert!(rig.connected.lock().unwrap()[&rig.addr]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
    assert_eq!(
        send_letter_turn_in_method(
            &rig.expected,
            71,
            &[0; 8],
            &transport,
            &rig.connected,
            &rig.mapping
        ),
        LetterTurnInSendOutcome::Sent
    );
    assert_eq!(rig.recorder.len(), 1);
    assert_eq!(
        rig.connected.lock().unwrap()[&rig.addr]
            .channel
            .lock()
            .unwrap()
            .tx_window
            .len(),
        1
    );
}

struct AsyncOnlyTransport;

impl Transport for AsyncOnlyTransport {
    fn send_to<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _bytes: &'life1 [u8],
        _addr: SocketAddr,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::io::Result<usize>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async { panic!("async send must not be used by the fenced method") })
    }

    fn local_addr(&self) -> std::io::Result<SocketAddr> {
        Ok("127.0.0.1:1".parse().unwrap())
    }
}

#[test]
fn unsupported_sync_transport_fails_closed_without_registration() {
    let rig = rig();
    let transport: Arc<dyn Transport> = Arc::new(AsyncOnlyTransport);
    assert_eq!(
        send_letter_turn_in_method(
            &rig.expected,
            71,
            &[0; 8],
            &transport,
            &rig.connected,
            &rig.mapping
        ),
        LetterTurnInSendOutcome::TransportError
    );
    assert!(rig.connected.lock().unwrap()[&rig.addr]
        .channel
        .lock()
        .unwrap()
        .tx_window
        .is_empty());
}
