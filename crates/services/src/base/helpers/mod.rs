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

/// Read the session's account access level (from `account.accesslevel`,
/// loaded at login). Returns 0 (Player) when the addr isn't connected or
/// the lock is poisoned — a missing session must never be treated as
/// privileged. Used by `createCharacter` to stamp the new character's
/// `access_level` from the account so it persists into world entry.
pub(crate) fn get_access_level(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> u32 {
    connected
        .lock()
        .ok()
        .and_then(|clients| clients.get(&addr).map(|c| c.access_level))
        .unwrap_or(0)
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
///
/// `reason` is a short, stable label naming why the disconnect fired
/// (`"client_disconnect"`, `"inactivity_timeout"`, `"send_error"`,
/// `"duplicate_login"`, `"logoff"`). Pin it across every call site
/// so SigNoz can pivot on `disconnect_reason` to answer "what kind
/// of disconnect am I looking at?" without inferring from message
/// text.
pub(crate) fn destroy_client_entities(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    reason: &'static str,
) {
    let (account_eid, player_eid, account_id, player_name, session_secs) = {
        let mut clients = match connected.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let Some(c) = clients.get(&addr) else {
            tracing::debug!(%addr, disconnect_reason = reason, "destroy_client_entities: no session, already cleaned up");
            return;
        };
        // Signal the tick-sync loop to exit before we remove the session.
        c.cancelled.store(true, Ordering::Relaxed);
        let account_eid = c.account_entity_id;
        let player_eid = c.player_entity_id;
        // Snapshot identity + session length for the Discord disconnect emit
        // before `remove` drops the state.
        let account_id = c.account_id;
        let player_name = c.player_name.clone();
        let session_secs = c.connected_at.elapsed().as_secs();
        clients.remove(&addr);
        (
            account_eid,
            player_eid,
            account_id,
            player_name,
            session_secs,
        )
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
    tracing::info!(
        %addr,
        disconnect_reason = reason,
        account_entity_id = account_eid,
        player_entity_id = ?player_eid,
        "Client entities cleaned up"
    );

    // Discord auth-channel: every teardown path funnels through here, so this
    // is the one place that reports *why* a player dropped. The stable
    // `reason` label maps to a typed `DisconnectReason` for the embed.
    cimmeria_discord::emit_player_disconnect(
        Some(account_id),
        player_name,
        addr,
        cimmeria_discord::DisconnectReason::from_label(reason),
        session_secs,
    );
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
mod tests;
