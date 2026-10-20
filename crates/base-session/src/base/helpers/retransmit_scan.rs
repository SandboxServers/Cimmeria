//! Retransmit scan for one session's reliable channel: collects the timed-out
//! encrypted datagrams together with the origin that proves them.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::SendOrigin;

use super::ConnectedClientState;

/// Cached encrypted datagram and the origin of the channel that yielded it.
/// Test fixtures without a Phase 3 seed retain the legacy untagged path.
pub struct PendingRetransmit {
    /// Exactly the bytes cached by the reliable channel.
    pub bytes: cimmeria_mercury::packet::Bytes,
    /// Immutable origin copied while the matching channel was locked.
    pub origin: Option<SendOrigin>,
}

impl std::ops::Deref for PendingRetransmit {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

/// Scan the current channel's timed-out TX entries for encrypted resend bytes.
/// The Channel keeps its own retransmit budget and exponential backoff.
/// Missing sessions and poisoned locks produce an empty batch.
pub fn collect_pending_retransmits(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Vec<PendingRetransmit> {
    collect_pending_retransmits_for_session(connected, addr, None)
}

/// Scan only the session that started a tick loop, never a same-peer replacement.
pub(crate) fn collect_pending_retransmits_for_session(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    expected_cancelled: Option<&Arc<std::sync::atomic::AtomicBool>>,
) -> Vec<PendingRetransmit> {
    let Ok(clients) = connected.lock() else {
        return Vec::new();
    };
    let Some(state) = clients.get(&addr) else {
        return Vec::new();
    };
    if expected_cancelled.is_some_and(|expected| !Arc::ptr_eq(expected, &state.cancelled)) {
        return Vec::new();
    }
    let Ok(mut channel) = state.channel.lock() else {
        return Vec::new();
    };
    let origin = state.wire_send_origin();
    // The channel's rows name the session (TG-MER-03); resynced each tick
    // because login and character select change it.
    channel.set_log_identity(crate::base::session_identity::channel_log_identity(state));
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
    if let Some(stall) = channel.check_tx_hole_named(|entry| {
        super::reliable_send::name_stalled_entry(&clients, &state.enc, entry)
    }) {
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
        .into_iter()
        .map(|bytes| {
            let matching: Vec<_> = channel
                .tx_window
                .iter()
                .filter(|entry| entry.raw_bytes == bytes)
                .collect();
            let proven = !matching.is_empty()
                && matching.iter().all(|entry| {
                    !entry
                        .send_kind
                        .is_some_and(|kind| kind.ends_with("_unproven"))
                });
            PendingRetransmit {
                bytes,
                origin: proven.then(|| origin.clone()).flatten(),
            }
        })
        .collect()
}
