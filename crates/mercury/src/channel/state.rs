//! Channel lifecycle state and per-packet TX/RX bookkeeping records.
//!
//! These plain data types are the building blocks the [`Channel`] state
//! machine in [`super::channel_core`] operates on.
//!
//! [`Channel`]: super::Channel

use std::time::Instant;

use crate::packet::{MessageHead, Packet, ParsedPacket};

// ── Channel state ───────────────────────────────────────────────────────────

/// Connection lifecycle states for a Mercury channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    /// Handshake in progress — waiting for the peer to acknowledge channel creation.
    Connecting,
    /// Channel is established and operational.
    Connected,
    /// Graceful shutdown initiated — draining remaining reliable packets.
    Disconnecting,
    /// Channel is fully closed.
    Disconnected,
}

// ── Log identity ────────────────────────────────────────────────────────────

/// Who is on the other end of a channel, for the channel's own log rows
/// (Rule 5 identity with its Rule 6 names). The transport knows only the
/// peer address, so the service copies the session's identity in through
/// [`Channel::set_log_identity`](super::Channel::set_log_identity) and the
/// channel stamps it on the rows it writes (`mercury.retransmit`,
/// `mercury.backpressure`, `mercury.tx_hole`, `mercury.rx_order`).
///
/// Every field is `Option` and starts `None`: before Phase 3 resolves the
/// account the channel has no one to name, and an unknown field is left off
/// the row, never written as `""` or `0`. Names are interned by the service,
/// so the struct is `Copy` and a per-tick sync costs no allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LogIdentity {
    /// The login's `account.account_id`.
    pub account_id: Option<u32>,
    /// The login name, paired with `account_id`.
    pub account_name: Option<&'static str>,
    /// The `sgw_player.player_id` being played; `None` at character select.
    pub player_id: Option<i32>,
    /// The character name, paired with `player_id`.
    pub player_name: Option<&'static str>,
}

// ── Per-packet TX metadata ──────────────────────────────────────────────────

/// A sent packet's first message, identified and named by the service
/// (`cimmeria_wire::names`; the transport has no method tables and no
/// session key). Built only when the transmit-hole watchdog warns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageNames {
    /// Its Mercury message id.
    pub msg_id: Option<u8>,
    /// The Mercury message name (`createEntity`, or `entityMethod`).
    pub msg_name: Option<&'static str>,
    /// The entity method, for an entity method call.
    pub method_name: Option<&'static str>,
}

/// Bookkeeping for a packet sitting in the transmit window awaiting ACK.
#[derive(Debug, Clone)]
pub struct TxEntry {
    /// The packet that was sent (retained for metadata — flags, seq).
    pub packet: Packet,
    /// When this packet was first sent. Unlike `last_sent`, a retransmit
    /// does not move it, so it ages the entry for
    /// [`ChannelHealth::oldest_unacked_age_ms`].
    pub first_sent: Instant,
    /// When this packet was last (re)transmitted.
    pub last_sent: Instant,
    /// How many times this packet has been retransmitted.
    pub retransmit_count: u32,
    /// Already-encrypted bytes that went on the wire for this packet's
    /// initial send. Retained so retransmits can re-send the exact same
    /// datagram without re-encrypting (which would require the session
    /// key be carried through `Channel`).
    ///
    /// Empty for entries inserted via the deprecated `send_packet` path
    /// (which stamps a sequence but never sees the encrypted bytes);
    /// `check_timeouts` silently skips bytes-empty entries during the
    /// retransmit scan.
    pub raw_bytes: bytes::Bytes,
    /// Dispatch site recorded by the service when the encrypted datagram
    /// was first sent. Absent for sends owned entirely by the channel.
    pub send_site: Option<String>,
    /// Broad send path; the call site narrows it to the handler.
    pub send_kind: Option<&'static str>,
    /// One-based fragment position and total, for a witness bundle.
    pub fragment_index: Option<usize>,
    pub fragment_count: Option<usize>,
    /// Message count in a bundled witness send.
    pub message_count: Option<usize>,
    /// The message the packet's body starts in, recorded at send time only
    /// for a fragment past the first of a bundle (its body starts inside the
    /// stream); any other packet's is read from `raw_bytes` on a stall.
    pub first_message: Option<MessageHead>,
    /// Most retransmits this entry gets before the channel gives up on
    /// it and drops it from the window unacked. `None` (every ordinary
    /// reliable packet) resends until acked. Set through
    /// [`Channel::register_sent_packet_capped`](super::Channel::register_sent_packet_capped).
    pub retransmit_cap: Option<u32>,
}

/// A reliable packet the channel stopped resending because it reached
/// its [`TxEntry::retransmit_cap`] without an ACK. Collected by
/// [`Channel::take_abandoned`](super::Channel::take_abandoned).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbandonedPacket {
    /// Sequence of the dropped entry.
    pub seq: u32,
    /// Retransmits it had when dropped (equal to its cap).
    pub retransmit_count: u32,
}

/// Bookkeeping for a received reliable packet buffered in the receive
/// window behind a gap.
#[derive(Debug, Clone)]
pub struct RxEntry {
    /// The received packet, footers already parsed (fragment range
    /// included, so a released fragment can go to the assembler).
    pub packet: ParsedPacket,
    /// When the packet was received.
    pub received_at: Instant,
}

/// A point-in-time snapshot of one channel's health, from
/// [`Channel::health`](super::Channel::health). Plain numbers, so a caller
/// can copy it out under the channel lock and log it after the lock is
/// gone (`session.end`, the inactivity row).
///
/// It answers "did the client vanish, or did the server stop getting
/// acks?": a large `last_rx_age_ms` with an empty window is a client that
/// went quiet; a deep `tx_outstanding` with an old
/// `oldest_unacked_age_ms` and climbing `retransmits_total` is a peer
/// that still talks but no longer acks what the server sends.
///
/// The `*_total` and stall counters run over the channel's whole life;
/// the ages are measured at the moment of the snapshot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChannelHealth {
    /// Reliable packets sent and not yet acked: the TX window plus the
    /// deferred-send queue behind it.
    pub tx_outstanding: u32,
    /// Milliseconds since the oldest unacked packet was first sent;
    /// `None` when nothing is outstanding.
    pub oldest_unacked_age_ms: Option<u64>,
    /// Retransmits sent over the channel's life (one per expired entry
    /// resent by `check_timeouts`).
    pub retransmits_total: u64,
    /// Transmit holes opened over the channel's life.
    pub tx_holes: u64,
    /// Transmit holes that stayed open past the watchdog threshold.
    pub tx_hole_stalls: u64,
    /// Receive gaps that blocked delivery past the stall threshold.
    pub rx_stalls: u64,
    /// Smoothed round-trip time in milliseconds; `None` before the first
    /// clean RTT sample.
    pub srtt_ms: Option<u64>,
    /// Milliseconds since the channel last heard from the peer.
    pub last_rx_age_ms: u64,
}

/// A transmit hole: the oldest outstanding reliable packet, once the peer
/// has acked a packet sent after it. The peer delivers nothing reliable
/// past a gap, so while this is open it is holding every later reliable
/// message. Tracked by [`super::Channel::process_ack_footer`], reported by
/// [`super::Channel::check_tx_hole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TxHole {
    /// Sequence of the packet the peer is missing.
    pub seq: u32,
    /// When the hole was first seen.
    pub since: Instant,
    /// When the watchdog last warned about it, if it has.
    pub warned_at: Option<Instant>,
}
