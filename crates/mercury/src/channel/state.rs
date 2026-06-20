//! Channel lifecycle state and per-packet TX/RX bookkeeping records.
//!
//! These plain data types are the building blocks the [`Channel`] state
//! machine in [`super::channel_core`] operates on.
//!
//! [`Channel`]: super::Channel

use std::time::Instant;

use crate::packet::Packet;

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

// ── Per-packet TX metadata ──────────────────────────────────────────────────

/// Bookkeeping for a packet sitting in the transmit window awaiting ACK.
#[derive(Debug, Clone)]
pub struct TxEntry {
    /// The packet that was sent (retained for metadata — flags, seq).
    pub packet: Packet,
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
}

/// Bookkeeping for a received packet in the receive window.
#[derive(Debug, Clone)]
pub struct RxEntry {
    /// The received packet.
    pub packet: Packet,
    /// When the packet was received.
    pub received_at: Instant,
}
