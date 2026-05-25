//! Per-session UDP send helpers and witness-routing utilities.
//!
//! # Two-counter sequencing model
//!
//! Every `ConnectedClientState` owns **two independent sequence counters**:
//!
//! - **`next_seq`** — reliable stream. Used by [`send_to_witness_reliable`]
//!   and other reliable application-packet paths. Each packet is also mirrored into the
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
//! | Tick sync | sent from `tick_sync.rs` (unreliable, own counter) | 10 Hz emit rate would saturate the 32-slot reliable TX window if it shared the reliable counter; loss is self-correcting (next tick 100 ms later supersedes) |
//! | AoI position update | [`send_to_witness`] (unreliable) | Superseded by next frame |
//!
//! **Default to reliable.** Only use [`send_to_witness`] (unreliable) if
//! the data is genuinely fire-and-forget AND the client tolerates loss.
//!
//! See `spec.protocol.mercury-wire-format` §1.7 for the wire-level
//! receiver model.
//!
//! # Negative-logging convention
//!
//! All three witness-send helpers below emit structured `warn!`
//! (entity-to-addr miss — player-visible drop) and `debug!`
//! (client-disconnected — transient race) events with a stable
//! `reason` field. Regression guards live in this file's `mod tests`
//! using `LogCapture`. See
//! [`docs/architecture/negative-logging-convention.md`] for the field
//! naming rules and level discipline that other negative-log seams
//! must also follow.
//!
//! [`Channel`]: cimmeria_mercury::channel::Channel
//! [`docs/architecture/negative-logging-convention.md`]: ../../../../docs/architecture/negative-logging-convention.md

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

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
/// Callers should invoke this AFTER `transport.send_to` succeeds, so a
/// failed send never appears as in-flight in the TX window.
///
/// `raw_bytes` should be the exact encrypted datagram that just went
/// on the wire. Pass `cimmeria_mercury::packet::Bytes::new()` if you
/// only want shadow-mode observability (ACK consumption + RTO sampling)
/// without retransmit support — the channel silently skips bytes-empty
/// entries during the retransmit scan.
///
/// **Overflow behavior.** When the TX window is full, the Channel queues
/// the entry in its per-session [`unsent_packets`] deque rather than
/// rejecting it (or — as a prior, broken implementation did — silently
/// downgrading the packet's reliable-delivery contract to best-effort).
/// Queued entries are dispatched on the wire at register time but the
/// retransmit scan only walks the TX window, so a queued entry becomes
/// eligible for retransmit only once an ACK frees a window slot and
/// promotion moves it across. ACKs that cover a still-queued seq drain
/// it from the queue directly without going through promotion.
///
/// The only remaining error condition routed through this helper is the
/// unsent-packets queue hitting its [`MAX_UNSENT_PACKETS`] cap, which
/// indicates the peer has stopped acking entirely and the channel is on
/// its way to the inactivity-timeout reap. That is surfaced at WARN so
/// it remains observable as a precursor to the channel-dead detection.
///
/// [`unsent_packets`]: cimmeria_mercury::channel::Channel::unsent_packets
/// [`MAX_UNSENT_PACKETS`]: cimmeria_mercury::consts::MAX_UNSENT_PACKETS
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
        // After the deferred-send queue landed, the only paths that
        // return Err from here are: out-of-range sequence (a programming
        // bug — the seq should have come from the masked counter), and
        // the unsent-packets queue cap. Both are channel-dead-class
        // signals, so WARN remains the right level.
        tracing::warn!(
            %addr,
            seq,
            error = %e,
            "shadow_register_reliable_send: packet bookkeeping rejected \
             (invalid seq or unsent-queue cap exceeded); reliability cannot \
             be tracked for this packet"
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
/// caller just iterates the returned bytes and `transport.send_to`s each.
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
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) where
    F: FnOnce(&[u8; 32], u32, &[u32]) -> Vec<u8>,
{
    // Extract all data from locks in a sync block so no MutexGuard crosses an await.
    let send_data = {
        // Read addr AND map size in one lock scope so the guard is
        // dropped before we re-enter any tracing path. Calling
        // `.lock()` again inside the match `None` arm would deadlock —
        // the scrutinee guard's lifetime extends through the match
        // body (regression caught by the negative-logging helper tests).
        //
        // `map_size` is a SNAPSHOT taken at this read. By the time the
        // warn! below fires, another thread may have added or removed
        // entries; the logged count is for ballpark-scope diagnosis,
        // not a load-bearing invariant.
        let (addr_opt, map_size) = {
            let m = entity_to_addr.lock().unwrap();
            (m.get(&witness_id).copied(), m.len())
        };
        let addr = match addr_opt {
            Some(a) => a,
            None => {
                // entity gone from address map mid-send is a
                // player-visible bug (witness sees stale state). warn! so
                // ops can grep this; `entity_count_in_map` is the
                // snapshot taken above.
                tracing::warn!(
                    witness_id,
                    reason = "entity_to_addr_miss",
                    entity_count_in_map = map_size,
                    "AoI: no client addr for witness -- packet dropped"
                );
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
                // Transient disconnect: client closed mid-AoI-update.
                // debug! (not warn) — happens during normal logoff races
                // but should remain queryable when investigating
                // missing-update bug reports.
                tracing::debug!(
                    witness_id,
                    %addr,
                    reason = "client_disconnected",
                    "AoI: client disconnected mid-send -- packet dropped"
                );
                None
            }
        }
    };

    if let Some((addr, key, seq, acks)) = send_data {
        let packet = build_packet(&key, seq, &acks);
        if let Err(e) = transport.send_to(&packet, addr).await {
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
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) where
    F: FnOnce(&[u8; 32], u32, &[u32]) -> Vec<u8>,
{
    let send_data = {
        // Read addr + map_size in one lock scope; see the unreliable
        // variant above for the deadlock-on-re-lock rationale and the
        // map_size snapshot caveat.
        let (addr_opt, map_size) = {
            let m = entity_to_addr.lock().unwrap();
            (m.get(&witness_id).copied(), m.len())
        };
        let addr = match addr_opt {
            Some(a) => a,
            None => {
                // Reliable path. Dropping a reliable AoI packet means
                // the client never sees a state-change (entity create/
                // destroy, method call) — the single biggest blind
                // spot for the world-entry spawn-glitch class.
                tracing::warn!(
                    witness_id,
                    reason = "entity_to_addr_miss",
                    entity_count_in_map = map_size,
                    "AoI reliable: no client addr for witness -- packet dropped"
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
                tracing::debug!(
                    witness_id,
                    %addr,
                    reason = "client_disconnected",
                    "AoI reliable: client disconnected mid-send -- packet dropped"
                );
                None
            }
        }
    };

    if let Some((addr, key, seq, acks)) = send_data {
        let packet = build_packet(&key, seq, &acks);
        if let Err(e) = transport.send_to(&packet, addr).await {
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

/// Send a [`ChannelBundle`] of N messages to a witness's client as a
/// reliable Mercury bundle (one or more fragmented packets).
///
/// Bundles collapse multiple cross-entity AoI / property messages into
/// fewer UDP datagrams, cutting per-packet header overhead AND reducing
/// the number of slots consumed in the per-channel TX window. See the
/// [`cimmeria_mercury::channel_bundle`] module doc for the
/// "one bundle == one client frame" rule (CRITICAL: do not combine
/// `CREATE_ENTITY(X)` with same-entity-X messages in one bundle).
///
/// The helper:
/// 1. Resolves `witness_id` → `addr` and reads the session key.
/// 2. Drains the session's pending ACKs into the bundle (ACKs ride only
///    the first finalized packet — bundle handles this internally).
/// 3. Atomically reserves `bundle.estimated_packet_count()` consecutive
///    reliable sequence numbers from the session counter, masked to the
///    28-bit space.
/// 4. Finalizes the bundle through the session AES-256-CBC encrypt path.
/// 5. Sends each fragment via the UDP socket.
/// 6. Registers each fragment with the per-session
///    [`Channel`](cimmeria_mercury::channel::Channel) so the retransmit
///    driver in `tick_sync.rs` can re-send on RTO expiry.
///
/// `estimated_packet_count` is the contract: it equals
/// `finalize().packets.len()` for this implementation (assert pinned in
/// the bundle tests), so the seq reservation matches actual emission
/// without a TOCTOU window.
///
/// Empty bundle (no messages, no acks) is a no-op — no seq is allocated,
/// no UDP traffic flows. Use [`ChannelBundle::is_empty`] on the caller
/// side if you want to skip the lookup overhead entirely.
///
/// [`ChannelBundle`]: cimmeria_mercury::channel_bundle::ChannelBundle
pub(crate) async fn send_bundle_to_witness_reliable(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    mut bundle: cimmeria_mercury::channel_bundle::ChannelBundle,
) {
    use cimmeria_mercury::packet::{FLAG_ON_CHANNEL, FLAG_RELIABLE, SEQUENCE_MASK};

    let send_data = {
        // Read addr + map_size in one lock scope; see the unreliable
        // variant for the deadlock-on-re-lock rationale and the
        // map_size snapshot caveat.
        let (addr_opt, map_size) = {
            let m = entity_to_addr.lock().unwrap();
            (m.get(&witness_id).copied(), m.len())
        };
        let addr = match addr_opt {
            Some(a) => a,
            None => {
                // Bundle path. Dropping a bundle drops a whole batch of
                // AoI messages — usually worse than the single-message
                // path. See unreliable/reliable variants above for the
                // rationale on warn-level.
                tracing::warn!(
                    witness_id,
                    reason = "entity_to_addr_miss",
                    entity_count_in_map = map_size,
                    "AoI bundle: no client addr for witness -- bundle dropped"
                );
                return;
            }
        };

        let clients = connected.lock().unwrap();
        let c = match clients.get(&addr) {
            Some(c) => c,
            None => {
                tracing::debug!(
                    witness_id,
                    %addr,
                    reason = "client_disconnected",
                    "AoI bundle: client disconnected mid-send -- bundle dropped"
                );
                return;
            }
        };

        // Drain pending ACKs into the bundle so they ride the first
        // finalized packet. Done under the same lock window as the seq
        // reservation so a concurrent ACK-pumping send doesn't race.
        let drained_acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
        bundle.add_acks(&drained_acks);

        // Now that ACKs are in, estimated_packet_count reflects the true
        // emit count (empty body + empty acks → 0; empty body + acks → 1;
        // otherwise ceil(body / FRAGMENT_BODY_SIZE)).
        let packet_count = bundle.estimated_packet_count();
        if packet_count == 0 {
            return;
        }

        // Atomically reserve `packet_count` consecutive sequence numbers.
        // Mask the base to the 28-bit Mercury space; per-fragment seqs
        // (base+1, base+2, ...) inherit the contiguous reservation and are
        // re-masked by build_fragmented_bundle internally.
        let base_seq = c.next_seq.fetch_add(packet_count as u32, Ordering::Relaxed) & SEQUENCE_MASK;
        let key = c.key;
        Some((addr, key, base_seq, packet_count))
    };

    let Some((addr, key, base_seq, packet_count)) = send_data else {
        return;
    };

    let num_messages = bundle.num_messages();
    let body_len = bundle.body_len();

    // Finalize through the session encrypt closure. Use FLAG_RELIABLE +
    // FLAG_ON_CHANNEL as base flags — the bundle adds FLAG_HAS_SEQUENCE,
    // FLAG_FRAGMENTED, FLAG_HAS_ACKS internally as needed per fragment.
    let base_flags = FLAG_RELIABLE | FLAG_ON_CHANNEL;
    let (packets, seqs_consumed) = bundle.finalize(base_flags, base_seq, |plaintext| {
        crate::mercury::encrypt_packet(plaintext, &key)
    });

    debug_assert_eq!(
        seqs_consumed as usize, packet_count,
        "estimated_packet_count contract violated — seq reservation overshoots finalize"
    );

    tracing::info!(
        %addr,
        witness_id,
        messages = num_messages,
        body_bytes = body_len,
        packets = packets.len(),
        base_seq,
        "AoI bundle: flushed {num_messages} messages in {} packet(s)",
        packets.len()
    );

    for (i, pkt) in packets.iter().enumerate() {
        let frag_seq = base_seq.wrapping_add(i as u32) & SEQUENCE_MASK;
        if let Err(e) = transport.send_to(pkt, addr).await {
            // Abort the rest of the bundle on the first send failure.
            // Continuing would push the trailing fragments onto the wire
            // with no chance of client-side reassembly (the failed
            // fragment's seq is already a gap in the reliable stream and
            // the bundle's frag_begin/frag_end footers expect every
            // fragment in [base_seq..base_seq+packet_count) to arrive).
            // The retransmit driver in tick_sync re-sends the registered
            // fragments [0..i); the unsent fragments [i..packet_count)
            // remain a permanent gap until the inactivity timer reaps
            // the channel — an outcome no worse than continuing, with
            // less wasted bandwidth.
            tracing::error!(
                witness_id,
                %addr,
                frag_seq,
                fragment = i + 1,
                total = packets.len(),
                already_sent = i,
                "AoI bundle: failed to send fragment; aborting remainder of bundle. \
                 Reliable seq stream now has gaps at [{}..{}); channel will be reaped \
                 on inactivity timeout: {e}",
                frag_seq,
                base_seq.wrapping_add(packet_count as u32) & SEQUENCE_MASK,
            );
            return;
        }
        shadow_register_reliable_send(
            connected,
            addr,
            frag_seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(pkt),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::Level;
    // TestTransport needed for the negative-logging log-capture guards below.
    use crate::test_support::TestTransport;

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

    /// TX-window pressure regression guard. The split-counter design only
    /// helps if the tick-sync emit path also avoids registering its
    /// packets in the reliable Channel's TX window. Without this guard,
    /// a refactor could re-introduce a `shadow_register_reliable_send`
    /// call on the tickSync path (the way `send_to_witness_reliable`
    /// does for actual reliable packets) and silently start filling
    /// the 32-slot window again.
    ///
    /// The test calls [`tick_sync_packet`] — the same function `run_tick_loop`
    /// delegates to — so any registration logic added *inside that function*
    /// will cause the assertion to fire. Note the guard's scope: a
    /// `shadow_register_reliable_send` call added *at the call site in
    /// `run_tick_loop`* (outside the helper) would still slip past this test.
    ///
    /// [`tick_sync_packet`]: crate::base::tick_sync::tick_sync_packet
    #[test]
    fn tick_sync_emission_does_not_consume_reliable_tx_window_slots() {
        use crate::base::tick_sync::tick_sync_packet;
        use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags};

        let state = crate::test_support::test_default_connected_client_state();

        // Burst: fill the TX window with 30 reliable application packets.
        // Mirrors the world-entry shape (charList + versionInfo +
        // resourceFragments + createBasePlayer + mapLoaded fragments).
        {
            let mut ch = state.channel.lock().unwrap();
            for seq in 0..30u32 {
                let pkt = Packet::new(PacketFlags::default(), seq, Bytes::new());
                ch.register_sent_packet(pkt, Bytes::new())
                    .expect("register_sent_packet must succeed under window cap");
            }
            assert_eq!(ch.tx_window.len(), 30, "TX window seeded with 30 reliable");
        }

        // Run 10 tick iterations via the same function `run_tick_loop` calls.
        // The UDP send is omitted — the TX-window-pressure failure mode is
        // about register_sent_packet calls, not socket I/O.
        for tick in 0..10u32 {
            let (_seq_id, _pkt) =
                tick_sync_packet(&state.next_seq_unreliable, &state.key, tick, &[]);
        }

        // The reliable TX window must be unchanged. If a future refactor
        // accidentally registers tickSync seqs into the window, this
        // assertion will fire (`tx_window.len() == 40` instead of 30).
        let ch = state.channel.lock().unwrap();
        assert_eq!(
            ch.tx_window.len(),
            30,
            "tickSync emission must not consume reliable TX window slots — \
             the split-counter design's invariant. If this fires, check that \
             `tick_sync_packet` still avoids calling `shadow_register_reliable_send`."
        );
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

    // ──────────────────────────────────────────────────────────────────
    // Negative-logging regression guards.
    //
    // These tests fail if the warn/debug log level on the witness-miss
    // and disconnect paths is reverted to the original `trace!`. Per
    // TESTING.md, a regression guard must fail when the fix is reverted;
    // these do that by asserting on both the level AND the `reason`
    // structured field — so a generic level-only revert (e.g. someone
    // demoting back to `trace`) AND a field-removing revert both trip.
    //
    // The bug shape this guards: a witness AoI packet silently dropped
    // at `trace!` was the single biggest blind spot for the missing-
    // entity-update class of bugs (#288 spawn glitches). Promoting to
    // warn! with a stable `reason` field makes the drop greppable in
    // ops.
    // ──────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn send_to_witness_emits_warn_when_entity_to_addr_misses() {
        use crate::test_support::LogCapture;
        use std::collections::HashMap;
        use std::net::SocketAddr;
        use std::sync::{Arc, Mutex};

        let capture = LogCapture::install();

        let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
            Arc::new(TestTransport::default());
        let connected = Arc::new(Mutex::new(
            HashMap::<SocketAddr, ConnectedClientState>::new(),
        ));
        // Deliberately empty — the witness has no addr mapping.
        let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

        send_to_witness(
            &transport,
            &connected,
            &entity_to_addr,
            999, // witness_id not in map
            |_key, _seq, _acks| vec![],
        )
        .await;

        let found = capture.find_event(
            Level::WARN,
            "no client addr for witness",
            "entity_to_addr_miss",
        );
        assert!(
            found.is_some(),
            "negative-logging convention: AoI witness-miss must emit WARN with reason=entity_to_addr_miss; \
             reverting to trace!/debug! breaks ops visibility of the #288-class spawn glitches. \
             Captured events: {:#?}",
            capture.all()
        );
    }

    #[tokio::test]
    async fn send_to_witness_reliable_emits_warn_when_entity_to_addr_misses() {
        use crate::test_support::LogCapture;
        use std::collections::HashMap;
        use std::net::SocketAddr;
        use std::sync::{Arc, Mutex};

        let capture = LogCapture::install();

        let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
            Arc::new(TestTransport::default());
        let connected = Arc::new(Mutex::new(
            HashMap::<SocketAddr, ConnectedClientState>::new(),
        ));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

        send_to_witness_reliable(
            &transport,
            &connected,
            &entity_to_addr,
            42,
            |_key, _seq, _acks| vec![],
        )
        .await;

        assert!(
            capture
                .find_event(
                    Level::WARN,
                    "no client addr for witness",
                    "entity_to_addr_miss"
                )
                .is_some(),
            "negative-logging convention: reliable AoI witness-miss must emit WARN with reason=entity_to_addr_miss"
        );
    }

    #[tokio::test]
    async fn send_bundle_to_witness_reliable_emits_warn_when_entity_to_addr_misses() {
        use crate::test_support::LogCapture;
        use cimmeria_mercury::channel_bundle::ChannelBundle;
        use std::collections::HashMap;
        use std::net::SocketAddr;
        use std::sync::{Arc, Mutex};

        let capture = LogCapture::install();

        let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
            Arc::new(TestTransport::default());
        let connected = Arc::new(Mutex::new(
            HashMap::<SocketAddr, ConnectedClientState>::new(),
        ));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::<u32, SocketAddr>::new()));

        let bundle = ChannelBundle::new(true);
        send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, 7, bundle).await;

        assert!(
            capture
                .find_event(
                    Level::WARN,
                    "no client addr for witness",
                    "entity_to_addr_miss"
                )
                .is_some(),
            "negative-logging convention: bundle AoI witness-miss must emit WARN with reason=entity_to_addr_miss"
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Disconnect/debug-path guards: addr is mapped but the
    // client is not in `connected` (the post-handshake disconnect race
    // window). The unreliable / reliable / bundle helpers all log this
    // at `debug!` with `reason="client_disconnected"`. Promoting back
    // to `trace!` would silence the path; the guards pin both the
    // level AND the reason.
    // ──────────────────────────────────────────────────────────────────

    /// Helper: build an (entity_to_addr, connected) pair where the
    /// witness has an addr mapping but no `ConnectedClientState`. This
    /// is exactly the post-handshake disconnect window the debug log
    /// guards.
    fn staged_disconnect_maps(
        witness_id: u32,
        addr: SocketAddr,
    ) -> (
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        let connected = Arc::new(Mutex::new(
            HashMap::<SocketAddr, ConnectedClientState>::new(),
        ));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(witness_id, addr)])));
        (connected, entity_to_addr)
    }

    #[tokio::test]
    async fn send_to_witness_emits_debug_when_client_disconnected() {
        use crate::test_support::LogCapture;

        let capture = LogCapture::install();
        let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
            Arc::new(TestTransport::default());
        let witness_addr: SocketAddr = "127.0.0.1:55101".parse().unwrap();
        let (connected, entity_to_addr) = staged_disconnect_maps(111, witness_addr);

        send_to_witness(
            &transport,
            &connected,
            &entity_to_addr,
            111,
            |_key, _seq, _acks| vec![],
        )
        .await;

        assert!(
            capture
                .find_event(
                    Level::DEBUG,
                    "client disconnected mid-send",
                    "client_disconnected"
                )
                .is_some(),
            "negative-logging convention: unreliable disconnect path must emit DEBUG with reason=client_disconnected"
        );
    }

    #[tokio::test]
    async fn send_to_witness_reliable_emits_debug_when_client_disconnected() {
        use crate::test_support::LogCapture;

        let capture = LogCapture::install();
        let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
            Arc::new(TestTransport::default());
        let witness_addr: SocketAddr = "127.0.0.1:55102".parse().unwrap();
        let (connected, entity_to_addr) = staged_disconnect_maps(222, witness_addr);

        send_to_witness_reliable(
            &transport,
            &connected,
            &entity_to_addr,
            222,
            |_key, _seq, _acks| vec![],
        )
        .await;

        assert!(
            capture
                .find_event(
                    Level::DEBUG,
                    "client disconnected mid-send",
                    "client_disconnected"
                )
                .is_some(),
            "negative-logging convention: reliable disconnect path must emit DEBUG with reason=client_disconnected"
        );
    }

    #[tokio::test]
    async fn send_bundle_to_witness_reliable_emits_debug_when_client_disconnected() {
        use crate::test_support::LogCapture;
        use cimmeria_mercury::channel_bundle::ChannelBundle;

        let capture = LogCapture::install();
        let transport: Arc<dyn cimmeria_mercury::transport::Transport> =
            Arc::new(TestTransport::default());
        let witness_addr: SocketAddr = "127.0.0.1:55103".parse().unwrap();
        let (connected, entity_to_addr) = staged_disconnect_maps(333, witness_addr);

        let bundle = ChannelBundle::new(true);
        send_bundle_to_witness_reliable(&transport, &connected, &entity_to_addr, 333, bundle).await;

        assert!(
            capture
                .find_event(
                    Level::DEBUG,
                    "client disconnected mid-send",
                    "client_disconnected"
                )
                .is_some(),
            "negative-logging convention: bundle disconnect path must emit DEBUG with reason=client_disconnected"
        );
    }
}
