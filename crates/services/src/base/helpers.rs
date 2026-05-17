//! Per-session UDP send helpers and witness-routing utilities.
//!
//! # Two-counter sequencing model
//!
//! Every `ConnectedClientState` owns **two independent sequence counters**:
//!
//! - **`next_seq`** — reliable stream. Used by [`send_to_witness_reliable`]
//!   and the `tick_sync` loop. Each packet is also mirrored into the
//!   per-session [`Channel`]'s TX window so the adaptive-RTO retransmit
//!   driver can recover loss. The client tracks this stream via `inSeqAt`
//!   at struct offset `+0x50` and **requires it to be contiguous** —
//!   gaps stall the connection.
//!
//! - **`next_seq_unreliable`** — unreliable stream, accessed via
//!   [`ConnectedClientState::next_unreliable_seq`]. Used by
//!   [`send_to_witness`] for fire-and-forget AoI position relays. The
//!   client deduplicates these via a separate structure at `+0x128` and
//!   does NOT expect contiguity. Lost packets are simply dropped — the
//!   next position frame supersedes them.
//!
//! **Critical invariant:** unreliable packets must NOT consume slots in
//! the reliable seq stream. If they do, the reliable stream gets
//! permanent holes the client can never fill, and every reliable packet
//! after the first hole gets buffered indefinitely (root cause of #317).
//!
//! # Which helper to use
//!
//! | Packet type | Helper | Reason |
//! |---|---|---|
//! | Entity spawn / destroy | [`send_to_witness_reliable`] | Client state depends on it |
//! | Entity method call | [`send_to_witness_reliable`] | Must execute exactly once |
//! | Property update | [`send_to_witness_reliable`] | Client state depends on it |
//! | Dialog / mission update | [`send_to_witness_reliable`] | UI-visible, can't be lost |
//! | Tick sync | sent from `tick_sync.rs` (reliable) | See #317 — reliable for ordering |
//! | AoI position update | [`send_to_witness`] (unreliable) | Superseded by next frame |
//!
//! **Default to reliable.** Only use [`send_to_witness`] (unreliable) if
//! the data is genuinely fire-and-forget AND the client tolerates loss.
//!
//! See `spec.protocol.mercury-wire-format` §1.7 for the wire-level
//! receiver model.
//!
//! [`Channel`]: cimmeria_mercury::channel::Channel

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
/// [`Channel`](cimmeria_mercury::channel::Channel).
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
///
/// **Failure mode — TX window full.** When `Channel::register_sent_packet`
/// returns `Err` (typically because the 32-slot TX window already holds
/// the spec-mandated maximum of in-flight reliable packets), the packet
/// is already on the wire but cannot be tracked here for ACK
/// processing or retransmit. The reliable-delivery contract is
/// effectively downgraded to best-effort for this single packet.
///
/// The current behavior logs at `warn` and continues — the alternative
/// (returning `Result` to ~30 callers so they can disconnect or apply
/// backpressure) is a meaningful API surface change worth its own PR.
/// In a healthy session this is unreachable: the cap is hit only when
/// the client has stopped acking for many ticks, in which case the
/// channel's inactivity / max-retries detection will kill the session
/// shortly anyway. Watch for repeated TX-window-full warns in
/// production logs as a precursor to that signal.
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
        // TX window full (or invalid seq) — the packet is on the wire
        // but won't be tracked for ACK / retransmit. Warn so this is
        // observable in production logs as a precursor to the
        // channel-dead detection (`is_timed_out` / max-retries).
        tracing::warn!(
            %addr,
            seq,
            error = %e,
            "shadow_register_reliable_send: packet sent on wire but NOT tracked \
             (TX window full or invalid seq); reliable-delivery downgraded to \
             best-effort for this packet"
        );
    }
}

/// Drain the per-session [`Channel`]'s retransmit queue: scan the TX
/// window for entries past the adaptive RTO and return the encrypted
/// bytes to re-send.
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

/// Drain pending ACKs and allocate the next sequence number, masked to
/// the 28-bit Mercury valid range.
///
/// The session-local `AtomicU32` counter monotonically increments past
/// `u32::MAX / SEQUENCE_MASK` cycles over a long-lived session; without
/// masking, an allocated seq could land inside the `NULL_SEQUENCE`
/// sentinel range or above the 28-bit space, get rejected by the
/// peer's parser (R4 drop), and silently break ACK draining. Masking
/// at allocation keeps every emitted seq inside the spec'd space.
pub(crate) fn drain_acks_and_seq(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<(Vec<u32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
    let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
    let seq = c.next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
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

/// Send an AoI packet to a specific witness's client — **unreliable**
/// variant. Use for self-correcting / ephemeral traffic where loss
/// recovers naturally on the next emit (currently only avatar position
/// updates fit this profile). Most callers want
/// [`send_to_witness_reliable`] instead.
///
/// Looks up the witness entity_id -> SocketAddr, then finds the client state
/// to get encryption key and sequence number. Calls the packet builder closure
/// and sends the result via UDP. No Channel registration — packets sent via
/// this path are NOT tracked for retransmit.
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
                // Unreliable counter — kept separate from `next_seq` so the
                // reliable seq stream remains contiguous. The receiver's
                // `inSeqAt` only advances for reliable arrivals; sharing the
                // counter creates gaps the client cannot fill. See
                // `ConnectedClientState::next_unreliable_seq` for the
                // encapsulated fetch-add + mask.
                let seq = c.next_unreliable_seq();
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

/// Send an AoI packet to a specific witness's client — **reliable**
/// variant. After the UDP send succeeds, registers the encrypted bytes
/// with the per-session [`Channel`]'s TX window so the retransmit
/// driver in `tick_sync.rs` re-sends on RTO expiry.
///
/// Use for **every** state-change AoI emit: entity create/destroy,
/// entity method calls (90%+ of server→client traffic — quest updates,
/// NPC spawns, interaction triggers, content engine events, inventory
/// changes, mission state, dialog opens), entity-invisible, entity-leave.
/// The wire format already sets `FLAG_RELIABLE` for these via
/// `REPLY_FLAGS_RELIABLE`; this helper closes the loop on the server's
/// send-window tracking so the FLAG_RELIABLE promise is kept.
///
/// **Do NOT** use for `build_avatar_update` (position relay) — those
/// are unreliable on the wire and should NOT be in the TX window.
/// Use plain [`send_to_witness`] for that case.
///
/// [`Channel`]: cimmeria_mercury::channel::Channel
pub(crate) async fn send_to_witness_reliable<F>(
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) where
    F: FnOnce(&[u8; 32], u32, &[u32]) -> Vec<u8>,
{
    let send_data = {
        let addr = match entity_to_addr.lock().unwrap().get(&witness_id).copied() {
            Some(a) => a,
            None => {
                tracing::trace!(
                    witness_id,
                    "AoI reliable: no client addr for witness -- skipping"
                );
                return;
            }
        };

        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => {
                let key = c.key;
                let seq = c.next_seq.fetch_add(1, Ordering::Relaxed)
                    & cimmeria_mercury::packet::SEQUENCE_MASK;
                let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
                Some((addr, key, seq, acks))
            }
            None => {
                tracing::trace!(witness_id, %addr, "AoI reliable: client disconnected -- skipping");
                None
            }
        }
    };

    if let Some((addr, key, seq, acks)) = send_data {
        let packet = build_packet(&key, seq, &acks);
        if let Err(e) = socket.send_to(&packet, addr).await {
            tracing::warn!(witness_id, %addr, "AoI reliable: failed to send packet: {e}");
            return;
        }
        // Register the encrypted bytes with the per-session Channel so
        // the retransmit driver in tick_sync re-sends on RTO expiry.
        shadow_register_reliable_send(
            connected,
            addr,
            seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(&packet),
        );
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

    /// Regression guard for the reliable / unreliable seq stream split
    /// (PR #317). The bug: a single shared counter meant every unreliable
    /// emission consumed a slot in the reliable stream the client expects
    /// to be contiguous, leaving permanent holes that stalled the session.
    ///
    /// This test asserts the wire-format-correct shape: bumping the
    /// unreliable counter does NOT advance the reliable one, and vice
    /// versa. A future refactor that re-merges them (a tempting
    /// "simplification") will fail this test before it ships.
    ///
    /// See `spec.protocol.mercury-wire-format` §1.7 and the disassembly
    /// of `UnAckedHandler::queueAckForPacket` for why this invariant is
    /// load-bearing on the client side.
    #[test]
    fn reliable_and_unreliable_seq_counters_are_independent() {
        let state = crate::test_support::test_default_connected_client_state();

        // Both counters start at the same value (0). They live in separate
        // dedup state on the receiver (`inSeqAt` at +0x50 vs the unreliable
        // structure at +0x128), so a shared starting value does not collide.
        let r0 = state.next_seq.load(Ordering::Relaxed);
        let u0 = state.next_seq_unreliable.load(Ordering::Relaxed);
        assert_eq!(r0, 0);
        assert_eq!(u0, 0);

        // Bumping the unreliable counter must NOT advance the reliable one.
        let u_first = state.next_unreliable_seq();
        assert_eq!(u_first, 0, "first unreliable seq is the initial value");
        assert_eq!(
            state.next_seq.load(Ordering::Relaxed),
            0,
            "reliable counter must NOT advance when an unreliable packet is sent",
        );

        // Bumping the reliable counter must NOT advance the unreliable one.
        let r_first = state.next_seq.fetch_add(1, Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;
        assert_eq!(r_first, 0, "first reliable seq is the initial value");
        assert_eq!(
            state.next_seq_unreliable.load(Ordering::Relaxed),
            1,
            "unreliable counter must NOT advance when a reliable packet is sent",
        );

        // Interleaved sequence: R, U, R, U, R. Each stream is monotonic
        // independently, regardless of interleave order — this is exactly
        // the shape that broke before the fix.
        let _r_second = state.next_seq.fetch_add(1, Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;
        let _u_second = state.next_unreliable_seq();
        let r_third = state.next_seq.fetch_add(1, Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;
        let u_third = state.next_unreliable_seq();
        let r_fourth = state.next_seq.fetch_add(1, Ordering::Relaxed)
            & cimmeria_mercury::packet::SEQUENCE_MASK;

        assert_eq!(r_third, 2, "reliable stream stays contiguous (0,1,2,...)");
        assert_eq!(
            r_fourth, 3,
            "reliable stream stays contiguous (0,1,2,3,...)"
        );
        assert_eq!(u_third, 2, "unreliable stream stays contiguous (0,1,2,...)");
    }

    /// The encapsulating accessor [`ConnectedClientState::next_unreliable_seq`]
    /// must mask its return value to the 28-bit Mercury sequence space.
    /// A regression that drops the `SEQUENCE_MASK` would let the counter
    /// roll into the reserved high 4 bits and corrupt the flags byte on
    /// the wire (the failure shape from issue #292).
    #[test]
    fn next_unreliable_seq_masks_to_28_bit_space() {
        let state = crate::test_support::test_default_connected_client_state();
        // Pre-load the counter near the wrap point.
        state
            .next_seq_unreliable
            .store(cimmeria_mercury::packet::SEQUENCE_MASK, Ordering::Relaxed);

        let seq = state.next_unreliable_seq();
        assert_eq!(
            seq,
            cimmeria_mercury::packet::SEQUENCE_MASK,
            "last value before wrap is the mask itself"
        );

        let wrapped = state.next_unreliable_seq();
        assert_eq!(
            wrapped, 0,
            "next call after wrap masks back to 0 — the 4 reserved high \
             bits must never leak into the seq footer"
        );
    }
}
