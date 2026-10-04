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

use cimmeria_common::EntityId;
use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;

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
    if let Some(stall) = channel.check_tx_hole() {
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
///
/// A session whose character was still in the world (`listed_online`)
/// also tells its contact-list watchers and organizations it went offline
/// (`session_presence::spawn_offline`, on its own task; audit A-35, ORG-06),
/// with `reason` as the `disconnect_reason`.
///
/// The player entity id is **not** returned to `EntityManager`'s free list
/// until the cell confirms it has torn the mirrored cell entity down (its
/// `DisconnectEntity` reply). Freeing it eagerly let a concurrent login
/// recycle the id via `allocate_id`'s FIFO free list before the cell had
/// even seen the disconnect, so the old session's `DisconnectEntity` — sent
/// or still in flight — could land on and destroy the *new* player's cell
/// entity (issue #999). When the cell send fails outright, or the cell
/// drops the reply without confirming teardown, the id is withheld from
/// reuse permanently rather than reused unconfirmed: an unrecycled id costs
/// nothing (the id space is an `i32` counter), a reused one racing a live
/// cell can destroy another player's session.
///
/// The Base→Cell send and the wait for that reply run on a **spawned
/// task**, not inline. This function is called from the base's single UDP
/// receive loop (`client_disconnect`, `duplicate_login`) and from the
/// per-session tick-sync loop (`inactivity_timeout`); awaiting a cell round
/// trip — which itself does a DB write (`persist_last_position`) before
/// replying — inline there would pause packet intake for every connected
/// player whenever the cell is busy or the shared Base→Cell channel is
/// backpressured. Everything that does not depend on the cell's reply (the
/// session-map removal, the Account entity free, the reverse-index removal,
/// the plugins' teardown hook (the crafting-queue drop), the offline-presence
/// fan-out, the Discord emit)
/// still runs synchronously before this function returns.
pub fn destroy_client_entities(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    transport: &Arc<dyn Transport>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
    reason: &'static str,
) {
    let (
        account_eid,
        player_eid,
        account_id,
        player_name,
        discord_account,
        discord_character,
        session_secs,
        ended,
        plugins,
    ) = {
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
        let discord_account = c.discord_account();
        let discord_character = c.discord_character();
        let player_name = c.player_name.clone();
        let session_secs = c.connected_at.elapsed().as_secs();
        // Snapshot before `remove`: a character still in the world is
        // announced offline once (a `logOff` already unlisted and announced).
        let ended = match (c.listed_online, c.active_player_id, player_eid) {
            (true, Some(player_id), Some(entity_id)) => {
                Some(crate::base::session_presence::EndedSession {
                    account_id,
                    player_id,
                    entity_id,
                    player_name: player_name.clone(),
                })
            }
            _ => None,
        };
        crate::base::player_index::log_unlisted(addr, c, reason);
        crate::base::deferred_aoi::log_discarded_on_teardown(addr, c, reason);
        // The session's plugin registry outlives the session for the
        // disconnect hook below.
        let plugins = clients.remove(&addr).map(|c| c.plugins).unwrap_or_default();
        (
            account_eid,
            player_eid,
            account_id,
            player_name,
            discord_account,
            discord_character,
            session_secs,
            ended,
            plugins,
        )
    };

    if account_eid != 0 {
        tracing::debug!(%addr, account_entity_id = account_eid, "Destroying Account entity");
        // The Account entity has no cell-side mirror, so there is nothing to
        // race: free it immediately.
        entity_manager
            .lock()
            .unwrap()
            .destroy_entity(EntityId(account_eid as i32));
    }
    if let Some(player_eid) = player_eid {
        tracing::debug!(%addr, player_entity_id = player_eid, "Destroying Player entity");
        tracing::info!(
            target: "session.end",
            %addr,
            entity_id = player_eid,
            account_id,
            player_name = ?player_name,
            disconnect_reason = reason,
            session_secs,
            "player session ended"
        );

        // Remove from entity->addr reverse index, and record the witness as
        // departed so the cell's in-flight sends to it log at DEBUG.
        unmap_departed_witness(entity_to_addr, player_eid);

        // The base plugins' teardown (#962 step 5): crafting's queued
        // inductions die with the session here, and nothing they would have
        // consumed is touched.
        plugins.run_session_hook(
            crate::base::plugin::SessionHookPoint::DisconnectAfterEntityUnmapped,
            crate::base::plugin::SessionEvent {
                entity_id: player_eid,
                cause: reason,
            },
        );

        // Every user chat channel this character was in loses it here too:
        // this is the disconnect/timeout/duplicate-login teardown, the
        // counterpart of `handle_log_off`'s own call for the two paths a
        // client-initiated logOff covers. Neither call site fires for gate
        // travel (`base-world-entry/gate_travel`), which reuses the same
        // entity id, so membership survives a world change untouched.
        crate::base::user_channels::user_channel_registry().leave_all(player_eid);

        // Notify CellService to disconnect and destroy the cell entity, and
        // hold `player_eid` out of `EntityManager`'s free list until the
        // cell confirms the teardown finished -- see the function doc and
        // issue #999. Spawned so the caller (the UDP receive loop, or the
        // tick-sync loop) never blocks on the cell's reply.
        match cell_tx {
            Some(tx) => {
                let tx = tx.clone();
                let entity_manager = Arc::clone(entity_manager);
                tokio::spawn(async move {
                    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                    match tx
                        .send(BaseToCellMsg::DisconnectEntity {
                            entity_id: player_eid,
                            reply_tx,
                        })
                        .await
                    {
                        Ok(()) => match reply_rx.await {
                            Ok(()) => {
                                entity_manager
                                    .lock()
                                    .unwrap()
                                    .destroy_entity(EntityId(player_eid as i32));
                            }
                            Err(_) => {
                                tracing::warn!(
                                    entity_id = player_eid,
                                    account_id,
                                    disconnect_reason = reason,
                                    "destroy_client_entities: cell dropped the \
                                     DisconnectEntity reply without confirming \
                                     teardown -- entity id withheld from reuse; \
                                     the cell entity may be leaked in its space"
                                );
                            }
                        },
                        Err(e) => {
                            tracing::warn!(
                                entity_id = player_eid,
                                account_id,
                                disconnect_reason = reason,
                                error = %e,
                                "destroy_client_entities: DisconnectEntity send \
                                 failed -- cell may leak the player's entity in \
                                 its space, and the id is withheld from reuse \
                                 until it does"
                            );
                        }
                    }
                });
            }
            // No cell configured (no-cell test harnesses and the account-only
            // "no character in world yet" teardown): nothing on the other
            // side could be mid-teardown, so free the id immediately.
            None => {
                entity_manager
                    .lock()
                    .unwrap()
                    .destroy_entity(EntityId(player_eid as i32));
            }
        }
    }
    tracing::info!(
        %addr,
        disconnect_reason = reason,
        account_entity_id = account_eid,
        player_entity_id = ?player_eid,
        "Client entities cleaned up"
    );

    if let Some(ended) = ended {
        crate::base::session_presence::spawn_offline(
            ended,
            reason,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        );
    }

    // Discord auth-channel: every teardown path funnels through here, so this
    // is the one place that reports *why* a player dropped. The stable
    // `reason` label maps to a typed `DisconnectReason` for the embed.
    cimmeria_discord::emit_player_disconnect(
        discord_account,
        discord_character,
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
                    witness_id,
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
        tracing::warn!(witness_id, %addr, "AoI: failed to send packet: {e}");
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
    F: FnOnce(&[u8; 32], EncryptionVersion, u32, &[u32]) -> Vec<u8>,
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
                // DEBUG for a witness whose session just ended (the
                // teardown race), WARN otherwise: `departed_witnesses`.
                departed_witnesses::log_addr_miss(
                    witness_id,
                    map_size,
                    departed_witnesses::AddrMissPath::Reliable,
                );
                return WitnessSendOutcome::AddrUnresolved;
            }
        };

        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => {
                let key = c.key;
                let version = c.enc_version;
                let seq = c.next_seq.fetch_add(1, Ordering::Relaxed)
                    & cimmeria_mercury::packet::SEQUENCE_MASK;
                let acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
                    &mut c.pending_acks.lock().unwrap(),
                    c.enc_version,
                );
                Some((addr, key, version, seq, acks))
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

    let Some((addr, key, version, seq, acks)) = send_data else {
        return WitnessSendOutcome::ClientDisconnected;
    };
    let packet = build_packet(&key, version, seq, &acks);
    let bytes = packet.len();
    if let Err(e) = transport.send_to(&packet, addr).await {
        tracing::warn!(witness_id, %addr, "AoI reliable: failed to send packet: {e}");
        return WitnessSendOutcome::SendError;
    }
    // Register the encrypted bytes with the per-session Channel so
    // the retransmit driver in tick_sync re-sends on RTO expiry.
    shadow_register_reliable_send_with_details(
        connected,
        addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&packet),
        ReliableSendDetails {
            kind: "witness_single",
            fragment: None,
            message_count: None,
        },
    );
    WitnessSendOutcome::Sent { addr, seq, bytes }
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
    mut bundle: cimmeria_mercury::channel_bundle::ChannelBundle,
) -> BundleSendOutcome {
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
                // DEBUG for a witness whose session just ended (the
                // teardown race), WARN otherwise: `departed_witnesses`.
                departed_witnesses::log_addr_miss(
                    witness_id,
                    map_size,
                    departed_witnesses::AddrMissPath::Bundle,
                );
                return BundleSendOutcome::AddrUnresolved;
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
                return BundleSendOutcome::ClientDisconnected;
            }
        };

        // Drain pending ACKs into the bundle so they ride the first
        // finalized packet. Done under the same lock window as the seq
        // reservation so a concurrent ACK-pumping send doesn't race.
        let drained_acks: Vec<u32> = cimmeria_mercury::packet::take_piggyback_acks(
            &mut c.pending_acks.lock().unwrap(),
            c.enc_version,
        );
        bundle.add_acks(&drained_acks);

        // Now that ACKs are in, estimated_packet_count reflects the true
        // emit count (empty body + empty acks → 0; empty body + acks → 1;
        // otherwise ceil(body / FRAGMENT_BODY_SIZE)).
        let packet_count = bundle.estimated_packet_count();
        if packet_count == 0 {
            return BundleSendOutcome::Empty;
        }

        // Atomically reserve `packet_count` consecutive sequence numbers.
        // Mask the base to the 28-bit Mercury space; per-fragment seqs
        // (base+1, base+2, ...) inherit the contiguous reservation and are
        // re-masked by build_fragmented_bundle internally.
        let base_seq = c.next_seq.fetch_add(packet_count as u32, Ordering::Relaxed) & SEQUENCE_MASK;
        let key = c.key;
        let version = c.enc_version;
        Some((addr, key, version, base_seq, packet_count))
    };

    let Some((addr, key, version, base_seq, packet_count)) = send_data else {
        // Unreachable: every None path inside the block above early-returns
        // a specific outcome. Defensive fallback keeps the match exhaustive.
        return BundleSendOutcome::Empty;
    };

    let num_messages = bundle.num_messages();
    let body_len = bundle.body_len();
    // Fragment shape for the flush event: body bytes per packet and how many
    // cuts were moved off a message header (the client aborts a bundle whose
    // header straddles two packets).
    let plan = bundle.fragment_plan();
    let packet_bytes = format!("{:?}", plan.packet_sizes());
    let header_guarded_cuts = plan.header_guarded_cuts;

    // Finalize through the session encrypt closure. Use FLAG_RELIABLE +
    // FLAG_ON_CHANNEL as base flags — the bundle adds FLAG_HAS_SEQUENCE,
    // FLAG_FRAGMENTED, FLAG_HAS_ACKS internally as needed per fragment.
    let base_flags = FLAG_RELIABLE | FLAG_ON_CHANNEL;
    let (packets, seqs_consumed) = bundle.finalize(base_flags, base_seq, |plaintext| {
        crate::mercury::encrypt_packet(plaintext, &key, version)
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
        fragmented = packets.len() > 1,
        packet_bytes = %packet_bytes,
        header_guarded_cuts,
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
            return BundleSendOutcome::SendError;
        }
        shadow_register_reliable_send_with_details(
            connected,
            addr,
            frag_seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(pkt),
            ReliableSendDetails {
                kind: "witness_bundle",
                fragment: (packets.len() > 1).then_some((i + 1, packets.len())),
                message_count: Some(num_messages),
            },
        );
    }

    BundleSendOutcome::Sent {
        addr,
        base_seq,
        packets: packets.len(),
        bytes: body_len,
    }
}

mod reliable_send;
pub use reliable_send::shadow_register_reliable_send;
use reliable_send::{shadow_register_reliable_send_with_details, ReliableSendDetails};

mod departed_witnesses;
pub use departed_witnesses::{
    note_witness_departed, unmap_departed_witness, witness_recently_departed, DEPARTED_WITNESS_TTL,
};

mod witness_broadcast;
pub use witness_broadcast::broadcast_to_witnesses;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod disconnect_teardown;

#[cfg(test)]
mod departed_witnesses_tests;
