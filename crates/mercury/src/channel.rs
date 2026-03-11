//! Reliable UDP channel state machine.
//!
//! Each remote peer gets a dedicated `Channel` that tracks the sliding windows
//! for transmit and receive, handles ACK processing, and manages retransmission
//! timers. The channel lifecycle mirrors the C++ `Mercury::Channel` class.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

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
    pub fn send_packet(&mut self, mut packet: Packet) -> Result<()> {
        if self.tx_window.len() >= consts::TX_WINDOW_SIZE {
            return Err(cimmeria_common::CimmeriaError::Channel(
                format!(
                    "TX window full ({} packets), cannot enqueue seq={}",
                    self.tx_window.len(),
                    self.next_tx_seq,
                ),
            ));
        }

        // Stamp the outgoing sequence number onto the packet.
        packet.sequence = self.next_tx_seq;
        self.next_tx_seq = self.next_tx_seq.wrapping_add(1);

        let now = Instant::now();
        self.tx_window.push_back(TxEntry {
            packet,
            last_sent: now,
            retransmit_count: 0,
        });
        self.last_activity = now;

        Ok(())
    }

    /// Process an inbound packet, inserting it into the RX window.
    ///
    /// Returns `Ok(Some(packets))` with any newly in-order packets that
    /// can be delivered upstream, or `Ok(None)` if we are still waiting
    /// for earlier sequences.
    pub fn receive_packet(&mut self, packet: Packet) -> Result<Option<Vec<Packet>>> {
        let seq = packet.sequence;
        self.last_activity = Instant::now();

        // How far ahead of our expected sequence is this packet?
        // Wrapping subtraction handles sequence wraparound.
        let offset = seq.wrapping_sub(self.expected_rx_seq) as usize;

        // Drop if behind expected (duplicate/old) or beyond the window.
        if offset >= consts::RX_WINDOW_SIZE {
            // Either a duplicate (seq < expected, wrapping makes offset huge)
            // or too far ahead to buffer.
            return Ok(None);
        }

        // Grow the VecDeque with None slots if needed to reach the offset.
        while self.rx_window.len() <= offset {
            self.rx_window.push_back(None);
        }

        // Insert (ignore duplicates — don't overwrite an already-received slot).
        if self.rx_window[offset].is_none() {
            self.rx_window[offset] = Some(RxEntry {
                packet,
                received_at: self.last_activity,
            });
        }

        // Slide the window: drain consecutive Some entries from the front.
        let mut delivered = Vec::new();
        while let Some(Some(_)) = self.rx_window.front() {
            // Front slot is filled — deliver it.
            let entry = self.rx_window.pop_front().unwrap().unwrap();
            self.expected_rx_seq = self.expected_rx_seq.wrapping_add(1);
            delivered.push(entry.packet);
        }

        if delivered.is_empty() {
            Ok(None)
        } else {
            Ok(Some(delivered))
        }
    }

    /// Process acknowledgement information received from the peer.
    ///
    /// Removes acknowledged packets from the TX window and resets
    /// retransmit timers for selectively NACKed packets.
    pub fn process_acks(&mut self, ack_seq: u32) -> Result<()> {
        self.last_activity = Instant::now();

        // Cumulative ACK: remove all TX entries with sequence <= ack_seq.
        // The tx_window is ordered by sequence (oldest at front), so we can
        // drain from the front until we hit a sequence beyond the ACK.
        while let Some(front) = self.tx_window.front() {
            // Wrapping comparison: treat (front.seq - ack_seq) as signed.
            // If front.seq <= ack_seq (modular), the difference wraps to
            // a large positive value when front.seq > ack_seq.
            let diff = front.packet.sequence.wrapping_sub(ack_seq);
            if diff == 0 || diff > 0x8000_0000 {
                // front.packet.sequence <= ack_seq (in modular arithmetic)
                self.tx_window.pop_front();
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Check all packets in the TX window for retransmission timeouts.
    ///
    /// Returns a list of packets that need to be retransmitted.
    pub fn check_timeouts(&mut self) -> Vec<Packet> {
        let now = Instant::now();
        let timeout = std::time::Duration::from_millis(consts::ACK_TIMEOUT_MS);
        let mut retransmits = Vec::new();

        for entry in self.tx_window.iter_mut() {
            if now.duration_since(entry.last_sent) >= timeout {
                entry.retransmit_count += 1;
                entry.last_sent = now;
                retransmits.push(entry.packet.clone());
            }
        }

        retransmits
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

    /// Helper: create a minimal valid Packet for channel tests.
    fn test_packet() -> Packet {
        use bytes::Bytes;
        use crate::packet::PacketFlags;

        Packet::new(
            PacketFlags::default(),
            0, // sequence is overwritten by send_packet
            Bytes::from_static(&[0xDE, 0xAD]),
        )
    }

    fn test_addr() -> SocketAddr {
        "127.0.0.1:9000".parse().unwrap()
    }

    #[test]
    fn mark_reliable_stores_packet() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();

        assert_eq!(ch.tx_window.len(), 1);
        // send_packet stamps seq=0 (the first sequence number).
        assert_eq!(ch.tx_window[0].packet.sequence, 0);
        assert_eq!(ch.tx_window[0].retransmit_count, 0);
    }

    #[test]
    fn on_ack_removes_packet() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        assert_eq!(ch.tx_window.len(), 1);

        // Cumulative ACK for seq=0 should drain the window.
        ch.process_acks(0).unwrap();
        assert!(ch.tx_window.is_empty());
    }

    #[test]
    fn check_retransmits_returns_expired() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();

        // Backdate last_sent by 800ms — well past the 700ms ACK_TIMEOUT_MS.
        ch.tx_window[0].last_sent =
            std::time::Instant::now() - std::time::Duration::from_millis(800);

        let retransmits = ch.check_timeouts();
        assert_eq!(retransmits.len(), 1);
        assert_eq!(retransmits[0].sequence, 0);
        // check_timeouts bumps the retransmit counter.
        assert_eq!(ch.tx_window[0].retransmit_count, 1);
    }

    #[test]
    fn check_retransmits_empty_when_fresh() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();

        // Immediately after send, the packet is well within the 700ms timeout.
        let retransmits = ch.check_timeouts();
        assert!(retransmits.is_empty());
        assert_eq!(ch.tx_window[0].retransmit_count, 0);
    }

    #[test]
    fn sliding_window_rejects_overflow() {
        let mut ch = Channel::new(test_addr());

        // Fill the TX window to capacity (TX_WINDOW_SIZE = 45).
        for _ in 0..consts::TX_WINDOW_SIZE {
            ch.send_packet(test_packet()).unwrap();
        }
        assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);

        // The 46th packet must be rejected.
        let result = ch.send_packet(test_packet());
        assert!(result.is_err());

        // Window size unchanged — the rejected packet was not inserted.
        assert_eq!(ch.tx_window.len(), consts::TX_WINDOW_SIZE);
    }
}
