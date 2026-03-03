//! Reliable UDP channel state machine.
//!
//! Each remote peer gets a dedicated `Channel` that tracks the sliding windows
//! for transmit and receive, handles ACK processing, and manages retransmission
//! timers. The channel lifecycle mirrors the C++ `Mercury::Channel` class.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

use bytes::Bytes;
use cimmeria_common::Result;

use crate::consts;
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
    /// The packet that was sent (retained for retransmission).
    pub packet: Packet,
    /// When this packet was last (re)transmitted.
    pub last_sent: Instant,
    /// How many times this packet has been retransmitted.
    pub retransmit_count: u32,
}

/// Bookkeeping for a received packet in the receive window.
#[derive(Debug, Clone)]
pub struct RxEntry {
    /// The received packet.
    pub packet: Packet,
    /// When the packet was received.
    pub received_at: Instant,
}

// ── Channel ─────────────────────────────────────────────────────────────────

/// A reliable UDP channel to a single remote peer.
///
/// Manages sliding TX/RX windows, ACK tracking, and retransmission.
pub struct Channel {
    /// Current connection state.
    pub state: ChannelState,

    /// Outbound packets awaiting acknowledgement.
    pub tx_window: VecDeque<TxEntry>,

    /// Inbound packets buffered for ordered delivery.
    pub rx_window: VecDeque<Option<RxEntry>>,

    /// Next sequence number to assign to an outbound packet.
    pub next_tx_seq: u32,

    /// Next sequence number we expect to receive from the peer.
    pub expected_rx_seq: u32,

    /// Socket address of the remote peer.
    pub remote_addr: SocketAddr,

    /// Timestamp of the last activity (send or receive) on this channel.
    pub last_activity: Instant,
}

impl Channel {
    /// Create a new channel to `remote_addr` in the `Connecting` state.
    pub fn new(remote_addr: SocketAddr) -> Self {
        Self {
            state: ChannelState::Connecting,
            tx_window: VecDeque::with_capacity(consts::TX_WINDOW_SIZE),
            rx_window: VecDeque::with_capacity(consts::RX_WINDOW_SIZE),
            next_tx_seq: 0,
            expected_rx_seq: 0,
            remote_addr,
            last_activity: Instant::now(),
        }
    }

    /// Queue a packet for reliable transmission.
    ///
    /// The packet is appended to the TX window and will be retransmitted
    /// until acknowledged or the retry limit is reached.
    pub fn send_packet(&mut self, _packet: Packet) -> Result<()> {
        todo!("Channel::send_packet — enqueue in tx_window, assign sequence, mark timestamp")
    }

    /// Process an inbound packet, inserting it into the RX window.
    ///
    /// Returns `Ok(Some(packets))` with any newly in-order packets that
    /// can be delivered upstream, or `Ok(None)` if we are still waiting
    /// for earlier sequences.
    pub fn receive_packet(&mut self, _packet: Packet) -> Result<Option<Vec<Packet>>> {
        todo!("Channel::receive_packet — insert into rx_window, slide window, return in-order")
    }

    /// Process acknowledgement information received from the peer.
    ///
    /// Removes acknowledged packets from the TX window and resets
    /// retransmit timers for selectively NACKed packets.
    pub fn process_acks(&mut self, _ack_seq: u32) -> Result<()> {
        todo!("Channel::process_acks — remove acked packets from tx_window")
    }

    /// Check all packets in the TX window for retransmission timeouts.
    ///
    /// Returns a list of packets that need to be retransmitted.
    pub fn check_timeouts(&mut self) -> Vec<Packet> {
        todo!("Channel::check_timeouts — scan tx_window for expired packets")
    }

    /// Returns `true` if the channel has exceeded the maximum retry count
    /// or the keepalive interval has elapsed without any activity.
    pub fn is_timed_out(&self) -> bool {
        let idle_ms = self.last_activity.elapsed().as_millis() as u64;

        // Check if any TX entry has exceeded max retries.
        let max_retries_exceeded = self.tx_window.iter().any(|entry| {
            entry.retransmit_count >= consts::MAX_RETRIES
        });

        // Consider the channel timed out if idle too long with no activity
        // and no outstanding reliable traffic.
        let idle_timeout = idle_ms > consts::KEEPALIVE_INTERVAL_MS * (consts::MAX_RETRIES as u64);

        max_retries_exceeded || idle_timeout
    }

    /// Touch the last-activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[test]
    fn new_channel_starts_connecting() {
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let ch = Channel::new(addr);
        assert_eq!(ch.state, ChannelState::Connecting);
        assert_eq!(ch.next_tx_seq, 0);
        assert_eq!(ch.expected_rx_seq, 0);
        assert_eq!(ch.remote_addr, addr);
        assert!(ch.tx_window.is_empty());
        assert!(ch.rx_window.is_empty());
    }

    #[test]
    fn fresh_channel_not_timed_out() {
        let addr: SocketAddr = "127.0.0.1:9000".parse().unwrap();
        let ch = Channel::new(addr);
        assert!(!ch.is_timed_out());
    }
}
