//! Opt-in reliable Mercury sends bound to one Base connection and character.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;

use super::{reliable_fit, ConnectedClientState, WitnessSendOutcome};

/// Owner observed before registering a minigame or closing its UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipientSnapshot {
    pub entity_id: u32,
    pub player_id: i32,
    pub addr: SocketAddr,
    pub session_instance_id: u64,
}

impl RecipientSnapshot {
    pub(super) fn matches(self, state: &ConnectedClientState) -> bool {
        self.session_instance_id != 0
            && state.session_instance_id == self.session_instance_id
            && state.player_entity_id == Some(self.entity_id)
            && state.active_player_id == Some(self.player_id)
    }
}

/// Capture the current owner; no ticket may be registered without one.
pub fn snapshot_recipient(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    entity_id: u32,
    player_id: i32,
) -> Option<RecipientSnapshot> {
    if player_id <= 0 {
        return None;
    }
    let addr = entity_to_addr.lock().ok()?.get(&entity_id).copied()?;
    let clients = connected.lock().ok()?;
    let state = clients.get(&addr)?;
    (state.session_instance_id != 0
        && state.player_entity_id == Some(entity_id)
        && state.active_player_id == Some(player_id))
    .then_some(RecipientSnapshot {
        entity_id,
        player_id,
        addr,
        session_instance_id: state.session_instance_id,
    })
}

/// Refuse a changed mapping/owner before preparation; `reliable_fit` repeats
/// the owner check in the lock window that consumes ACKs and reliable seqs.
pub async fn send_to_session_bound_reliable<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    recipient: RecipientSnapshot,
    kind: &'static str,
    build_packet: F,
) -> WitnessSendOutcome
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    if entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&recipient.entity_id).copied())
        != Some(recipient.addr)
    {
        return WitnessSendOutcome::AddrUnresolved;
    }
    reliable_fit::send_reliable_to_addr_matching(
        transport,
        connected,
        recipient.addr,
        recipient.entity_id,
        kind,
        Some(recipient),
        build_packet,
    )
    .await
}
