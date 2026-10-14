//! One-datagram client method send bound to the entry that accepted a letter turn-in.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_mercury::consts::PACKET_MAX_SIZE;
use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags, SEQUENCE_MASK};
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::{
    LetterTurnInEntryEpoch, LetterTurnInRequest, LetterTurnInToken,
};
use cimmeria_wire::mercury::build_player_entity_method_packet;
use sha2::{Digest, Sha256};

use super::ConnectedClientState;

const TOKEN_DOMAIN: &[u8] = b"cimmeria:letter-turn-in:v1";

/// Derive the same opaque token sent at world entry. Never log its bytes.
pub fn derive_letter_turn_in_token(key: &[u8; 32]) -> LetterTurnInToken {
    let mut digest = Sha256::new();
    digest.update(TOKEN_DOMAIN);
    digest.update(key);
    let hash = digest.finalize();
    let mut prefix = [0; 16];
    prefix.copy_from_slice(&hash[..16]);
    LetterTurnInToken::from_bytes(prefix)
}

/// Exact connected entry captured at synchronous request acceptance.
#[derive(Clone)]
pub struct ExpectedLetterTurnInEntry {
    pub addr: SocketAddr,
    pub entity_id: u32,
    pub player_id: i32,
    pub cancelled: Arc<AtomicBool>,
    pub key: [u8; 32],
    pub generation: Instant,
    pub epoch: LetterTurnInEntryEpoch,
    pub token: LetterTurnInToken,
}

impl std::fmt::Debug for ExpectedLetterTurnInEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExpectedLetterTurnInEntry")
            .field("addr", &self.addr)
            .field("entity_id", &self.entity_id)
            .field("player_id", &self.player_id)
            .field("generation", &self.generation)
            .field("epoch", &self.epoch)
            .field("token", &self.token)
            .finish_non_exhaustive()
    }
}

impl ExpectedLetterTurnInEntry {
    /// Called while both maps are locked, before the route yields.
    pub fn capture(
        addr: SocketAddr,
        state: &ConnectedClientState,
        request: &LetterTurnInRequest,
    ) -> Option<Self> {
        let epoch = *state.extensions.get::<LetterTurnInEntryEpoch>()?;
        let expected = Self {
            addr,
            entity_id: request.entity_id,
            player_id: request.player_id,
            cancelled: Arc::clone(&state.cancelled),
            key: state.key,
            generation: request.generation,
            epoch: request.epoch,
            token: request.token,
        };
        (epoch == request.epoch && expected.matches_state(state)).then_some(expected)
    }

    fn matches_state(&self, state: &ConnectedClientState) -> bool {
        Arc::ptr_eq(&self.cancelled, &state.cancelled)
            && !state.cancelled.load(Ordering::Acquire)
            && state.player_entity_id == Some(self.entity_id)
            && state.active_player_id == Some(self.player_id)
            && state.connected_at == self.generation
            && state.extensions.get::<LetterTurnInEntryEpoch>() == Some(&self.epoch)
            && state.world_name.as_deref() == Some("Harset_CmdCenter")
            && state.key == self.key
            && derive_letter_turn_in_token(&state.key) == self.token
    }

    /// Check the state while the caller holds its connected-map lock.
    pub fn matches_connected(&self, state: Option<&ConnectedClientState>) -> bool {
        state.is_some_and(|state| self.matches_state(state))
    }

    pub fn is_current(
        &self,
        connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) -> bool {
        let Ok(mappings) = entity_to_addr.lock() else {
            return false;
        };
        if mappings.get(&self.entity_id) != Some(&self.addr) {
            return false;
        }
        let Ok(clients) = connected.lock() else {
            return false;
        };
        clients
            .get(&self.addr)
            .is_some_and(|state| self.matches_state(state))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LetterTurnInSendOutcome {
    Sent,
    Stale,
    Oversized,
    TransportError,
    ChannelError,
}

/// Admit exactly one small method on the captured key and channel.
pub async fn send_letter_turn_in_method(
    expected: &ExpectedLetterTurnInEntry,
    method_index: u16,
    args: &[u8],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> LetterTurnInSendOutcome {
    send_with_probe_hook(
        expected,
        method_index,
        args,
        transport,
        connected,
        entity_to_addr,
        || {},
    )
    .await
}

pub(super) async fn send_with_probe_hook<F: FnOnce()>(
    expected: &ExpectedLetterTurnInEntry,
    method_index: u16,
    args: &[u8],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    after_probe: F,
) -> LetterTurnInSendOutcome {
    if !expected.is_current(connected, entity_to_addr) {
        return LetterTurnInSendOutcome::Stale;
    }
    let version = {
        let clients = connected.lock().unwrap();
        clients.get(&expected.addr).map(|state| state.enc_version)
    };
    let Some(version) = version else {
        return LetterTurnInSendOutcome::Stale;
    };
    let probe = build_player_entity_method_packet(
        &expected.key,
        1,
        &[],
        expected.entity_id,
        method_index,
        args,
        version,
    );
    if probe.len() > PACKET_MAX_SIZE {
        return LetterTurnInSendOutcome::Oversized;
    }
    after_probe();

    // The mapping and session stay locked through sequence allocation. No
    // replacement can lend its key or sequence to the accepted old entry.
    let reserved = {
        let Ok(mappings) = entity_to_addr.lock() else {
            return LetterTurnInSendOutcome::Stale;
        };
        if mappings.get(&expected.entity_id) != Some(&expected.addr) {
            return LetterTurnInSendOutcome::Stale;
        }
        let Ok(clients) = connected.lock() else {
            return LetterTurnInSendOutcome::Stale;
        };
        clients.get(&expected.addr).and_then(|state| {
            expected.matches_state(state).then(|| {
                (
                    state.next_seq.fetch_add(1, Ordering::Relaxed) & SEQUENCE_MASK,
                    state.enc_version,
                )
            })
        })
    };
    let Some((seq, version)) = reserved else {
        return LetterTurnInSendOutcome::Stale;
    };
    let packet = build_player_entity_method_packet(
        &expected.key,
        seq,
        &[],
        expected.entity_id,
        method_index,
        args,
        version,
    );
    if packet.len() > PACKET_MAX_SIZE {
        return LetterTurnInSendOutcome::Oversized;
    }
    if transport.send_to(&packet, expected.addr).await.is_err() {
        return LetterTurnInSendOutcome::TransportError;
    }

    // Registration is an atomic session check and channel mutation. A
    // replacement never inherits retransmits encrypted under the old key.
    let Ok(mappings) = entity_to_addr.lock() else {
        return LetterTurnInSendOutcome::Stale;
    };
    if mappings.get(&expected.entity_id) != Some(&expected.addr) {
        return LetterTurnInSendOutcome::Stale;
    }
    let Ok(clients) = connected.lock() else {
        return LetterTurnInSendOutcome::Stale;
    };
    let Some(state) = clients
        .get(&expected.addr)
        .filter(|s| expected.matches_state(s))
    else {
        return LetterTurnInSendOutcome::Stale;
    };
    let Ok(mut channel) = state.channel.lock() else {
        return LetterTurnInSendOutcome::ChannelError;
    };
    let sent = Packet::new(PacketFlags::default(), seq, Bytes::new());
    if channel
        .register_sent_packet(sent, Bytes::copy_from_slice(&packet))
        .is_err()
    {
        return LetterTurnInSendOutcome::ChannelError;
    }
    LetterTurnInSendOutcome::Sent
}
