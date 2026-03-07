use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;

use cimmeria_common::EntityId;
use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;

use super::ConnectedClientState;

/// Format a byte slice as a hex string for trace logging.
pub(crate) fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}

/// Drain pending ACKs and allocate the next sequence number.
pub(crate) fn drain_acks_and_seq(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<(Vec<u32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients
        .get_mut(&addr)
        .ok_or("addr not in connected map")?;
    let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
    let seq = c.next_seq.fetch_add(1, Ordering::Relaxed);
    Ok((acks, seq))
}

/// Read the dynamically allocated account entity ID for a connected client.
pub(crate) fn get_account_entity_id(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients.get(&addr).ok_or("addr not in connected map")?;
    Ok(c.account_entity_id)
}

/// Destroy all entities associated with a disconnecting client and remove it from the map.
///
/// Safe to call multiple times for the same address -- returns silently if the
/// session was already removed (e.g. DISCONNECT handler cleaned up, then the
/// tick-sync inactivity timeout fires on the now-absent session).
///
/// Always sets `cancelled` on the session before removal so the tick-sync loop
/// exits promptly instead of running until the 60-second inactivity timeout.
pub(crate) fn destroy_client_entities(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let (account_eid, player_eid) = {
        let mut clients = match connected.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let Some(c) = clients.get(&addr) else {
            tracing::debug!(%addr, "destroy_client_entities: no session, already cleaned up");
            return;
        };
        // Signal the tick-sync loop to exit before we remove the session.
        c.cancelled.store(true, Ordering::Relaxed);
        let account_eid = c.account_entity_id;
        let player_eid = c.player_entity_id;
        clients.remove(&addr);
        (account_eid, player_eid)
    };

    let mut mgr = entity_manager.lock().unwrap();
    if account_eid != 0 {
        tracing::debug!(%addr, account_entity_id = account_eid, "Destroying Account entity");
        mgr.destroy_entity(EntityId(account_eid as i32));
    }
    if let Some(player_eid) = player_eid {
        tracing::debug!(%addr, player_entity_id = player_eid, "Destroying Player entity");
        mgr.destroy_entity(EntityId(player_eid as i32));

        // Remove from entity->addr reverse index
        entity_to_addr.lock().unwrap().remove(&player_eid);

        // Notify CellService to disconnect and destroy the cell entity
        if let Some(tx) = cell_tx {
            let _ = tx.try_send(BaseToCellMsg::DisconnectEntity {
                entity_id: player_eid,
            });
        }
    }
    tracing::info!(%addr, "Client entities cleaned up");
}

/// Send an AoI packet to a specific witness's client.
///
/// Looks up the witness entity_id -> SocketAddr, then finds the client state
/// to get encryption key and sequence number. Calls the packet builder closure
/// and sends the result via UDP.
pub(crate) async fn send_to_witness<F>(
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) where
    F: FnOnce(&[u8; 32], u32, &[u32]) -> Vec<u8>,
{
    // Extract all data from locks in a sync block so no MutexGuard crosses an await.
    let send_data = {
        let addr = match entity_to_addr.lock().unwrap().get(&witness_id).copied() {
            Some(a) => a,
            None => {
                tracing::trace!(witness_id, "AoI: no client addr for witness -- skipping");
                return;
            }
        };

        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => {
                let key = c.key;
                let seq = c.next_seq.fetch_add(1, Ordering::Relaxed);
                let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
                Some((addr, key, seq, acks))
            }
            None => {
                tracing::trace!(witness_id, %addr, "AoI: client disconnected -- skipping");
                None
            }
        }
    };

    if let Some((addr, key, seq, acks)) = send_data {
        let packet = build_packet(&key, seq, &acks);
        if let Err(e) = socket.send_to(&packet, addr).await {
            tracing::warn!(witness_id, %addr, "AoI: failed to send packet: {e}");
        }
    }
}
