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

use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::Transport;

// Test modules reach these through `use super::*`.
#[cfg(test)]
use crate::cell::messages::BaseToCellMsg;
#[cfg(test)]
use cimmeria_entity::manager::EntityManager;

use super::ConnectedClientState;

/// Outcome of a single-packet witness send ([`send_to_witness`] /
/// [`send_to_witness_reliable`]).
///
/// Both helpers already emit their own structured `warn!`/`debug!` on the
/// failure arms; this return value exists so a *caller* that wants
/// success-side visibility (the AoI entity-introduction emit path —
/// `aoi.create_emit` / `aoi.create_send_failed`) can log per-packet
/// `seq`/`bytes`/`addr_resolved` without re-resolving the address itself.
///
/// Returning a value is purely additive: existing call sites use these
/// helpers as `...await;` statements and ignore the result. The type is
/// intentionally NOT `#[must_use]` so those sites compile unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessSendOutcome {
    /// Packet hit the wire. `seq` is the reliable/unreliable sequence
    /// number consumed; `bytes` is the encrypted datagram length.
    Sent {
        addr: SocketAddr,
        seq: u32,
        bytes: usize,
    },
    /// `witness_id` had no entry in `entity_to_addr` — packet dropped.
    /// Mirrors the helper's `reason = "entity_to_addr_miss"` warn.
    AddrUnresolved,
    /// The session was gone from `connected` mid-send (logoff race) —
    /// packet dropped. Mirrors the helper's `reason = "client_disconnected"`
    /// debug.
    ClientDisconnected,
    /// Address resolved and session present, but `transport.send_to`
    /// returned an I/O error. Mirrors the helper's send-failure warn.
    SendError,
}

impl WitnessSendOutcome {
    /// `true` only for [`WitnessSendOutcome::Sent`]. Test-only inspector
    /// (the seams match the variant directly).
    #[cfg(test)]
    pub(crate) fn is_sent(&self) -> bool {
        matches!(self, WitnessSendOutcome::Sent { .. })
    }

    /// `true` when the address was successfully resolved from
    /// `entity_to_addr` — i.e. anything other than
    /// [`WitnessSendOutcome::AddrUnresolved`]. Used as the `addr_resolved`
    /// field on the AoI create-emit seam.
    pub fn addr_resolved(&self) -> bool {
        !matches!(self, WitnessSendOutcome::AddrUnresolved)
    }

    /// Stable `reason` token for the failure arms, or `None` on success.
    /// Pinned by the negative-logging regression guards — treat as API.
    pub fn failure_reason(&self) -> Option<&'static str> {
        match self {
            WitnessSendOutcome::Sent { .. } => None,
            WitnessSendOutcome::AddrUnresolved => Some("entity_to_addr_miss"),
            WitnessSendOutcome::ClientDisconnected => Some("client_disconnected"),
            WitnessSendOutcome::SendError => Some("send_error"),
        }
    }
}

/// Outcome of a bundle witness send ([`send_bundle_to_witness_reliable`]).
///
/// Same rationale as [`WitnessSendOutcome`] but carries the multi-fragment
/// shape: a bundle finalizes to `packets` fragments starting at `base_seq`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleSendOutcome {
    /// All fragments hit the wire. `base_seq` is the first reserved seq;
    /// `packets` is the fragment count; `bytes` is the total accumulated
    /// body length.
    Sent {
        addr: SocketAddr,
        base_seq: u32,
        packets: usize,
        bytes: usize,
    },
    /// `witness_id` had no entry in `entity_to_addr` — bundle dropped.
    AddrUnresolved,
    /// Session gone from `connected` mid-send — bundle dropped.
    ClientDisconnected,
    /// Empty bundle (no messages, no acks) — nothing reserved, no-op.
    Empty,
    /// A fragment failed to send mid-bundle; the reliable seq stream now
    /// has a gap and the channel will be reaped on inactivity timeout.
    SendError,
}

impl BundleSendOutcome {
    /// `true` only for [`BundleSendOutcome::Sent`]. Test-only inspector
    /// (the seams match the variant directly).
    #[cfg(test)]
    pub(crate) fn is_sent(&self) -> bool {
        matches!(self, BundleSendOutcome::Sent { .. })
    }

    /// `true` when the address resolved from `entity_to_addr`.
    pub fn addr_resolved(&self) -> bool {
        !matches!(self, BundleSendOutcome::AddrUnresolved)
    }

    /// Stable `reason` token for the non-sent arms, or `None` on success.
    pub fn failure_reason(&self) -> Option<&'static str> {
        match self {
            BundleSendOutcome::Sent { .. } => None,
            BundleSendOutcome::AddrUnresolved => Some("entity_to_addr_miss"),
            BundleSendOutcome::ClientDisconnected => Some("client_disconnected"),
            BundleSendOutcome::Empty => Some("empty_bundle"),
            BundleSendOutcome::SendError => Some("send_error"),
        }
    }
}

/// The packet hex formatter for trace logs; shared with the wire firehose.
pub use cimmeria_wire::hex::to_hex;

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
/// A capped entry that reached its cap unacked (the login reply and
/// time-sync, #842) is dropped by the scan instead of resent; each one
/// gets a single WARN here, `event = "reliable_resend_abandoned"`, with
/// the session's `account_id` and the `seq`.
///
/// [`Channel`]: cimmeria_mercury::channel::Channel
pub fn collect_pending_retransmits(
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
    // Receive-stall watchdog, on the same 100 ms tick as the retransmit
    // scan: a client reliable packet the client never resent is blocking
    // everything it sent after it (NA38). The WARN is logged inside.
    if let Some(stall) = channel.check_rx_stall() {
        if stall.first_warning {
            cimmeria_observability::counter!("mercury_rx_stalls_total");
        }
    }
    // Transmit-hole watchdog, the other direction: the client has acked
    // reliable packets we sent after one it never acked, so it is holding
    // everything behind that one (no entity creates, leaves or method
    // calls reach it) until a resend lands. The WARN is logged inside.
    if let Some(stall) = channel
        .check_tx_hole_named(|entry| reliable_send::name_stalled_entry(&clients, &state.enc, entry))
    {
        if stall.first_warning {
            cimmeria_observability::counter!("mercury_tx_hole_stalls_total");
        }
    }
    let retransmits = channel.check_timeouts();
    for dropped in channel.take_abandoned() {
        // One row per packet, once: the channel has stopped resending it.
        // For the login handshake this is a client that ignored both the
        // original and every resend of seq 1 or 2 (#842).
        tracing::warn!(
            %addr,
            account_id = state.account_id,
            account_name = state.account_name.as_deref(),
            seq = dropped.seq,
            retransmit_count = dropped.retransmit_count,
            event = "reliable_resend_abandoned",
            reason = "retransmit_cap_reached",
            "reliable packet reached its retransmit cap unacked; the channel stopped resending it"
        );
    }
    retransmits
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
pub fn drain_acks_and_seq(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<(Vec<u32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients.get_mut(&addr).ok_or("addr not in connected map")?;
    let acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
        &mut c.pending_acks.lock().unwrap(),
        c.enc_version,
    );
    let seq = c.next_seq.fetch_add(1, Ordering::Relaxed) & cimmeria_mercury::packet::SEQUENCE_MASK;
    Ok((acks, seq))
}

/// Read the session's wire-encryption version for a connected client.
///
/// Returns [`EncryptionVersion::default`] (V1) when the addr isn't connected
/// or the lock is poisoned — a missing session falls back to the legacy v1
/// cipher rather than failing the send. Used by the direct-key outbound
/// handlers (char list, version info, resource fragments) that need the
/// session's version to build their packets but don't otherwise hold a
/// `ConnectedClientState` reference.
pub fn get_enc_version(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> EncryptionVersion {
    connected
        .lock()
        .ok()
        .and_then(|clients| clients.get(&addr).map(|c| c.enc_version))
        .unwrap_or_default()
}

/// Read the dynamically allocated account entity ID for a connected client.
pub fn get_account_entity_id(
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
pub fn get_access_level(
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
pub async fn send_to_witness<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) -> WitnessSendOutcome
where
    F: FnOnce(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
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
                // DEBUG for a witness whose session just ended (the
                // teardown race), WARN otherwise: `departed_witnesses`.
                departed_witnesses::log_addr_miss(
                    witness_id,
                    map_size,
                    departed_witnesses::AddrMissPath::Unreliable,
                    connected,
                );
                return WitnessSendOutcome::AddrUnresolved;
            }
        };

        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => {
                let key = c.key;
                let version = c.enc_version;
                // Unreliable counter — kept separate from `next_seq` so the
                // reliable seq stream remains contiguous. The receiver's
                // `inSeqAt` only advances for reliable arrivals; sharing the
                // counter creates gaps the client cannot fill. See
                // `ConnectedClientState::next_unreliable_seq` for the
                // encapsulated fetch-add + mask.
                let seq = c.next_unreliable_seq();
                let acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
                    &mut c.pending_acks.lock().unwrap(),
                    c.enc_version,
                );
                Some((addr, key, version, seq, acks))
            }
            None => {
                // Transient disconnect: client closed mid-AoI-update.
                // debug! (not warn) — happens during normal logoff races
                // but should remain queryable when investigating
                // missing-update bug reports.
                tracing::debug!(
                    witness_id, // nt:id-only the session left mid-send, so nothing names it
                    %addr,
                    reason = "client_disconnected",
                    "AoI: client disconnected mid-send -- packet dropped"
                );
                None
            }
        }
    };

    let Some((addr, key, version, seq, acks)) = send_data else {
        return WitnessSendOutcome::ClientDisconnected;
    };
    let packet = build_packet(&key, version, seq, &acks);
    let bytes = packet.len();
    if let Err(e) = transport.send_to(&packet, addr).await {
        tracing::warn!(
            witness_id,
            witness_name = crate::base::session_identity::entity_name_for(
                connected,
                entity_to_addr,
                witness_id
            ),
            %addr,
            "AoI: failed to send packet: {e}"
        );
        return WitnessSendOutcome::SendError;
    }
    WitnessSendOutcome::Sent { addr, seq, bytes }
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
/// `build_packet` is called twice, once to measure the packet and once to
/// send it, and a packet whose body cannot fit one 1472-byte datagram goes
/// out as a fragmented bundle instead: see [`send_reliable_to_addr`].
///
/// **Do NOT** use for `build_avatar_update` (position relay) — those
/// are unreliable on the wire and should NOT be in the TX window.
/// Use plain [`send_to_witness`] for that case.
///
/// [`Channel`]: cimmeria_mercury::channel::Channel
pub async fn send_to_witness_reliable<F>(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) -> WitnessSendOutcome
where
    F: Fn(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
{
    // Read addr + map_size in one lock scope; see the unreliable variant
    // above for the deadlock-on-re-lock rationale and the map_size
    // snapshot caveat.
    let (addr_opt, map_size) = {
        let m = entity_to_addr.lock().unwrap();
        (m.get(&witness_id).copied(), m.len())
    };
    let Some(addr) = addr_opt else {
        // DEBUG for a witness whose session just ended (the teardown race),
        // WARN otherwise: `departed_witnesses`.
        departed_witnesses::log_addr_miss(
            witness_id,
            map_size,
            departed_witnesses::AddrMissPath::Reliable,
            connected,
        );
        return WitnessSendOutcome::AddrUnresolved;
    };
    send_reliable_to_addr(
        transport,
        connected,
        addr,
        witness_id,
        "witness_single",
        build_packet,
    )
    .await
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
pub async fn send_bundle_to_witness_reliable(
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    bundle: cimmeria_mercury::channel_bundle::ChannelBundle,
) -> BundleSendOutcome {
    // Read addr + map_size in one lock scope; see the unreliable
    // variant for the deadlock-on-re-lock rationale and the
    // map_size snapshot caveat.
    let (addr_opt, map_size) = {
        let m = entity_to_addr.lock().unwrap();
        (m.get(&witness_id).copied(), m.len())
    };
    let Some(addr) = addr_opt else {
        // DEBUG for a witness whose session just ended (the
        // teardown race), WARN otherwise: `departed_witnesses`.
        departed_witnesses::log_addr_miss(
            witness_id,
            map_size,
            departed_witnesses::AddrMissPath::Bundle,
            connected,
        );
        return BundleSendOutcome::AddrUnresolved;
    };
    reliable_fit::send_bundle_reliable_to_addr(
        transport,
        connected,
        addr,
        witness_id,
        bundle,
        "witness_bundle",
    )
    .await
}

mod reliable_fit;
pub use reliable_fit::send_reliable_to_addr;

mod reliable_send;
pub use reliable_send::shadow_register_reliable_send;

mod departed_witnesses;
pub use departed_witnesses::{
    note_witness_departed, unmap_departed_witness, witness_recently_departed, DEPARTED_WITNESS_TTL,
};

mod witness_broadcast;
pub use witness_broadcast::broadcast_to_witnesses;

mod session_teardown;
pub use session_teardown::{destroy_client_entities, destroy_owned_client_entities};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod disconnect_teardown;

#[cfg(test)]
mod departed_witnesses_tests;

#[cfg(test)]
mod reliable_fit_tests;

#[cfg(test)]
mod sent_message_head_tests;

#[cfg(test)]
mod session_end_names_tests;

#[cfg(test)]
mod session_teardown_tests;
