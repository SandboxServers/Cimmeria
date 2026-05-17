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
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Register an outgoing reliable packet's sequence number AND its
/// encrypted on-wire bytes with the per-session
/// [`Channel`](cimmeria_mercury::channel::Channel) (issue #308).
///
/// The Channel records the entry in its TX window for two purposes:
/// 1. **ACK tracking** — when the client acks this seq, the entry
///    drains and an RTT sample feeds the per-peer adaptive RTO.
/// 2. **Retransmit** — if the RTO fires before the ack arrives, the
///    tick driver re-sends `raw_bytes` verbatim (no re-encryption).
///
/// Callers should invoke this AFTER `socket.send_to` succeeds, so a
/// failed send never appears as in-flight in the TX window.
///
/// `raw_bytes` should be the exact encrypted datagram that just went
/// on the wire. Pass `cimmeria_mercury::packet::Bytes::new()` if you
/// only want shadow-mode observability (ACK consumption + RTO sampling)
/// without retransmit support — the channel silently skips bytes-empty
/// entries during the retransmit scan.
pub(crate) fn shadow_register_reliable_send(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    seq: u32,
    raw_bytes: cimmeria_mercury::packet::Bytes,
) {
    use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags};

    let pkt = Packet::new(PacketFlags::default(), seq, Bytes::new());
    let Ok(clients) = connected.lock() else {
        return;
    };
    let Some(state) = clients.get(&addr) else {
        return;
    };
    let Ok(mut channel) = state.channel.lock() else {
        return;
    };
    if let Err(e) = channel.register_sent_packet(pkt, raw_bytes) {
        tracing::trace!(
            %addr,
            seq,
            error = %e,
            "channel.register_sent_packet failed (likely TX window full) — shadow mode tolerates this",
        );
    }
}

/// Drain the per-session [`Channel`]'s retransmit queue: scan the TX
/// window for entries past the adaptive RTO and return the encrypted
/// bytes to re-send. Issue #308.
///
/// Called from `tick_sync`'s per-session loop every 100 ms. The Channel
/// applies the per-tick budget (`RETRANSMIT_BUDGET_PER_TICK = 5`, issue
/// #292 finding #6) and Karn's exponential backoff internally; the
/// caller just iterates the returned bytes and `socket.send_to`s each.
///
/// Returns an empty vec on any lock-acquisition failure or missing
/// session — the next tick will try again.
///
/// [`Channel`]: cimmeria_mercury::channel::Channel
pub(crate) fn collect_pending_retransmits(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Vec<cimmeria_mercury::packet::Bytes> {
    let Ok(clients) = connected.lock() else {
        return Vec::new();
    };
    let Some(state) = clients.get(&addr) else {
        return Vec::new();
    };
    let Ok(mut channel) = state.channel.lock() else {
        return Vec::new();
    };
    channel.check_timeouts()
}

/// Drain pending ACKs and allocate the next sequence number.
pub(crate) fn drain_acks_and_seq(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<(Vec<u32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
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

/// Read the currently active entity ID for a connected client.
///
/// After world entry, the Account entity is destroyed and replaced by the
/// SGWPlayer entity. Protocol messages like `onVersionInfo` must be addressed
/// to whichever entity the client currently owns, otherwise the client
/// silently drops the response.
pub(crate) fn get_active_entity_id(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients.get(&addr).ok_or("addr not in connected map")?;
    Ok(c.player_entity_id.unwrap_or(c.account_entity_id))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// `to_hex` formats each byte as two uppercase hex digits, separated
    /// by single spaces. Pin the format so a refactor that swaps to
    /// lowercase or drops the separator doesn't silently change every
    /// trace log.
    #[test]
    fn to_hex_formats_bytes_as_uppercase_with_space_separator() {
        assert_eq!(to_hex(&[]), "");
        assert_eq!(to_hex(&[0x00]), "00");
        assert_eq!(to_hex(&[0xAB, 0xCD]), "AB CD");
        assert_eq!(
            to_hex(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]),
            "12 34 56 78 9A BC DE F0"
        );
    }

    /// `to_hex` zero-pads single-digit byte values. A regression that
    /// drops the `:02X` width specifier would emit "1 2" for [0x01, 0x02]
    /// instead of "01 02", breaking deterministic log diffs.
    #[test]
    fn to_hex_zero_pads_single_digit_bytes() {
        assert_eq!(to_hex(&[0x01, 0x02, 0x0F]), "01 02 0F");
    }
}
