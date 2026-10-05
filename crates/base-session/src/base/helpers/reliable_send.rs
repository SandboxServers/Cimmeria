//! Registering a reliable send with the session's Mercury channel, and the
//! `mercury.reliable_send` event that records its exact wire size,
//! fingerprint and send site.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::channel::{MessageNames, TxEntry};
use cimmeria_mercury::packet::MessageHead;

use super::ConnectedClientState;

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
#[track_caller]
pub fn shadow_register_reliable_send(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    seq: u32,
    raw_bytes: cimmeria_mercury::packet::Bytes,
) {
    shadow_register_reliable_send_with_details(
        connected,
        addr,
        seq,
        raw_bytes,
        ReliableSendDetails::default(),
    );
}

#[derive(Clone, Copy)]
pub(super) struct ReliableSendDetails {
    pub(super) kind: &'static str,
    pub(super) fragment: Option<(usize, usize)>,
    pub(super) message_count: Option<usize>,
    /// The message the packet's body starts in, for `mercury.tx_hole`.
    pub(super) first_message: Option<MessageHead>,
}

impl Default for ReliableSendDetails {
    fn default() -> Self {
        Self {
            kind: "direct",
            fragment: None,
            message_count: None,
            first_message: None,
        }
    }
}

/// Identify and name the first message of a stalled packet for the
/// `mercury.tx_hole` WARN, which the transport cannot do itself. Called only
/// on a tick that warns, with the session table locked. A later fragment of
/// a bundle carries the head its plan recorded; any other packet starts at a
/// message boundary, so its retained bytes are decrypted with the session key
/// and its head read at body offset 0 (a non-bundle fragment past the first
/// starts mid-stream and stays unnamed).
pub(super) fn name_stalled_entry(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    enc: &cimmeria_mercury::encryption::MercuryEncryption,
    entry: &TxEntry,
) -> MessageNames {
    let head = entry.first_message.or_else(|| {
        let plaintext = enc.decrypt(&entry.raw_bytes).ok()?;
        let packet = cimmeria_mercury::packet::parse_incoming(&plaintext).ok()?;
        let continuation = packet.frag_begin.is_some_and(|b| Some(b) != packet.seq_id);
        if continuation {
            return None;
        }
        cimmeria_mercury::packet::first_message_head(&packet.body)
    });
    head.map_or_else(MessageNames::default, |h| name_stalled_message(clients, &h))
}

/// Name one message head: an entity method on a player (the witness itself,
/// or any entity some session plays) reads the player table; on anything
/// else, the name every in-world type agrees on (a mob's or pet's 27-31 stay
/// unnamed).
pub(super) fn name_stalled_message(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    head: &MessageHead,
) -> MessageNames {
    let method_name = head.method_index.and_then(|index| {
        let is_player = head
            .entity_id
            .is_some_and(|entity| clients.values().any(|c| c.player_entity_id == Some(entity)));
        cimmeria_wire::names::entity_client_method(is_player, index)
    });
    MessageNames {
        msg_id: Some(head.msg_id),
        msg_name: cimmeria_wire::names::client_msg_name(head.msg_id),
        method_name,
    }
}

#[track_caller]
pub(super) fn shadow_register_reliable_send_with_details(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    seq: u32,
    raw_bytes: cimmeria_mercury::packet::Bytes,
    details: ReliableSendDetails,
) {
    use cimmeria_mercury::packet::{Bytes, Packet, PacketFlags};

    let wire_len = raw_bytes.len();
    let wire_fingerprint = cimmeria_mercury::instrumentation::wire_fingerprint(&raw_bytes);
    let caller = std::panic::Location::caller();
    let send_site = format!("{}:{}", caller.file(), caller.line());
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
        return;
    }
    channel.set_sent_packet_context(
        seq,
        send_site.clone(),
        details.kind,
        details.fragment,
        details.message_count,
        details.first_message,
    );
    let (fragment_index, fragment_count) = details.fragment.unzip();
    let packet_max_size = cimmeria_mercury::consts::PACKET_MAX_SIZE;
    if wire_len > packet_max_size {
        tracing::warn!(
            target: "mercury.reliable_send",
            event = "reliable_send",
            peer = %addr,
            seq,
            wire_len,
            packet_max_size,
            wire_fingerprint = %wire_fingerprint,
            %send_site,
            send_kind = details.kind,
            ?fragment_index,
            ?fragment_count,
            message_count = ?details.message_count,
            account_id = state.account_id,
            account_name = state.account_name.as_deref(),
            player_id = ?state.active_player_id,
            player_name = state.player_name.as_deref(),
            entity_id = ?state.player_entity_id,
            entity_name = state.player_name.as_deref(),
            "encrypted reliable datagram exceeds the Mercury UDP payload limit"
        );
    } else {
        // One per reliable datagram, witness traffic included: DEBUG,
        // still exported (instrumentation-discipline hot-path rule).
        tracing::debug!(
            target: "mercury.reliable_send",
            event = "reliable_send",
            peer = %addr,
            seq,
            wire_len,
            wire_fingerprint = %wire_fingerprint,
            %send_site,
            send_kind = details.kind,
            ?fragment_index,
            ?fragment_count,
            message_count = ?details.message_count,
            account_id = state.account_id,
            account_name = state.account_name.as_deref(),
            player_id = ?state.active_player_id,
            player_name = state.player_name.as_deref(),
            entity_id = ?state.player_entity_id,
            entity_name = state.player_name.as_deref(),
            "encrypted reliable datagram sent"
        );
    }
}
