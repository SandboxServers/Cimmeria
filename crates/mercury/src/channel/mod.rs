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
use crate::packet::{Packet, ParsedPacket};
use crate::unpacker::FragmentAssembler;

pub mod rto;

use rto::{Rto, RtoConfig};

#[cfg(test)]
mod tests;

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

    /// Wall-clock of the last outbound packet (send-side activity).
    /// Updated whenever we put bytes on the wire — `send_packet`,
    /// `check_timeouts` (when it returns retransmits the caller will
    /// emit), and `touch_sent` for out-of-band emits like keepalives.
    /// Used by [`Self::keepalive_due`] to decide when to send a
    /// keepalive: "we haven't sent anything in a while", independent
    /// of what the peer has done.
    pub last_sent: Instant,

    /// Wall-clock of the last inbound packet (receive-side activity).
    /// Updated by every code path that observes peer-originated bytes:
    /// `receive_packet`, `process_acks` (an ACK frame is peer data),
    /// and `touch_received` for callers that count peer traffic at
    /// the socket layer. Used by [`Self::is_timed_out`] to detect a
    /// silent peer — disconnect when the peer hasn't said anything
    /// in a while, independent of what we've sent them.
    ///
    /// Splitting `last_sent` and `last_received` is what makes both
    /// keepalive AND idle-disconnect actually fire on this side.
    /// Conflating them via a single `last_activity` reset on both
    /// paths meant our own sends would suppress the peer-silence
    /// check, so dead peers were never disconnected and (depending
    /// on traffic shape) keepalives never fired either. C++ Mercury
    /// maintains two timestamps for the same reason — see
    /// `src/mercury/channel.cpp`'s `lastReceived_` / `lastSent_`.
    pub last_received: Instant,

    /// Per-channel fragment reassembly buffer for `FLAG_FRAGMENTED`
    /// packets. Lives on `Channel` (not `Nub`) because the reassembly
    /// key (first-fragment sequence number) is per-peer — different
    /// channels can legitimately reuse the same low sequence numbers
    /// without colliding in a shared map.
    fragment_assembler: FragmentAssembler,

    /// Adaptive retransmission timeout state (issue #308). Tracks
    /// smoothed RTT + RTT variance for this peer; consulted by
    /// `check_timeouts` to decide when an unacked packet should be
    /// retransmitted, and updated by `process_acks` (clean samples,
    /// Karn's algorithm) and `check_timeouts` (exponential backoff
    /// on retransmit).
    rto: Rto,
}

impl Channel {
    /// Create a new channel to `remote_addr` in the `Connecting` state,
    /// seeded with the default [`RtoConfig`].
    pub fn new(remote_addr: SocketAddr) -> Self {
        Self::with_rto_config(remote_addr, RtoConfig::default())
    }

    /// Create a new channel with explicit [`RtoConfig`] — primarily for
    /// tests that need tight RTO bounds to keep test runtimes short.
    pub fn with_rto_config(remote_addr: SocketAddr, rto_config: RtoConfig) -> Self {
        let now = Instant::now();
        Self {
            state: ChannelState::Connecting,
            tx_window: VecDeque::with_capacity(consts::TX_WINDOW_SIZE),
            rx_window: VecDeque::with_capacity(consts::RX_WINDOW_SIZE),
            next_tx_seq: 0,
            expected_rx_seq: 0,
            remote_addr,
            last_sent: now,
            last_received: now,
            fragment_assembler: FragmentAssembler::new(),
            rto: Rto::new(rto_config),
        }
    }

    /// Read-only access to the per-channel RTO state. Diagnostics /
    /// logging only; the algorithm itself is internal to `Channel`.
    pub fn rto(&self) -> &Rto {
        &self.rto
    }

    /// Feed a parsed Mercury packet through this channel's fragment
    /// assembler and bump `last_received`.
    ///
    /// Non-fragmented packets pass through immediately; fragmented packets
    /// buffer until the bundle is complete. This is the receive-path
    /// equivalent of [`Self::send_packet`] for FLAG_FRAGMENTED bundles —
    /// non-fragmented `Packet`s still go through [`Self::receive_packet`].
    ///
    /// Per-channel ownership of the assembler matters: keying reassembly
    /// by sequence number alone would let one peer's fragments collide
    /// with another peer's identical sequence numbers in a shared map.
    /// Tying the assembler to the channel makes the per-peer scope
    /// implicit.
    pub fn reassemble_parsed(&mut self, pkt: &ParsedPacket) -> Result<Option<Bytes>> {
        self.last_received = Instant::now();
        self.fragment_assembler.process_parsed(pkt)
    }

    /// Drop reassembly buffers older than `max_age`. The drainer
    /// `Nub::tick` should call this periodically per channel, or callers
    /// can drive it themselves. Without it, fragments from a never-
    /// completing bundle would pin memory until the channel itself dies.
    pub fn cleanup_stale_fragments(&mut self, max_age: std::time::Duration) {
        self.fragment_assembler.cleanup_stale(max_age);
    }

    /// Queue a packet for reliable transmission.
    ///
    /// The packet is appended to the TX window and will be retransmitted
    /// until acknowledged or the retry limit is reached.
    pub fn send_packet(&mut self, mut packet: Packet) -> Result<()> {
        if self.tx_window.len() >= consts::TX_WINDOW_SIZE {
            return Err(cimmeria_common::CimmeriaError::Channel(format!(
                "TX window full ({} packets), cannot enqueue seq={}",
                self.tx_window.len(),
                self.next_tx_seq,
            )));
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
        self.last_sent = now;

        Ok(())
    }

    /// Process an inbound packet, inserting it into the RX window.
    ///
    /// Returns `Ok(Some(packets))` with any newly in-order packets that
    /// can be delivered upstream, or `Ok(None)` if we are still waiting
    /// for earlier sequences.
    pub fn receive_packet(&mut self, packet: Packet) -> Result<Option<Vec<Packet>>> {
        let seq = packet.sequence;
        self.last_received = Instant::now();

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
                received_at: self.last_received,
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
    /// Removes acknowledged packets from the TX window and feeds RTT
    /// samples to the per-channel RTO state machine for any **clean**
    /// rounds (Karn's algorithm — retransmitted packets are excluded
    /// because the ack is ambiguous about which copy reached the peer).
    pub fn process_acks(&mut self, ack_seq: u32) -> Result<()> {
        // ACK frames are peer-originated → counts as receive-side activity,
        // not send-side. (We aren't putting bytes on the wire; we're
        // observing the peer's response to a prior emit.)
        let now = Instant::now();
        self.last_received = now;

        // Cumulative ACK: remove all TX entries with sequence <= ack_seq.
        // The tx_window is ordered by sequence (oldest at front), so we can
        // drain from the front until we hit a sequence beyond the ACK.
        while let Some(front) = self.tx_window.front() {
            // Wrapping comparison: treat (front.seq - ack_seq) as signed.
            // If front.seq <= ack_seq (modular), the difference wraps to
            // a large positive value when front.seq > ack_seq.
            let diff = front.packet.sequence.wrapping_sub(ack_seq);
            if diff == 0 || diff > 0x8000_0000 {
                // front.packet.sequence <= ack_seq (in modular arithmetic).
                // Drain the entry and — if it was never retransmitted —
                // feed the round-trip time into the RTO smoother. This
                // is Karn's algorithm: only un-retransmitted samples are
                // unambiguous (we know exactly which send the ack matches).
                let entry = self
                    .tx_window
                    .pop_front()
                    .expect("front was Some inside the while loop");
                if entry.retransmit_count == 0 {
                    let rtt = now.duration_since(entry.last_sent);
                    self.rto.on_sample(rtt);
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Check all packets in the TX window for retransmission timeouts.
    ///
    /// Uses the per-channel adaptive RTO (issue #308) — `self.rto.current()`
    /// — rather than a fixed `consts::ACK_TIMEOUT_MS`. Each entry whose
    /// `last_sent` is older than the current RTO is selected for
    /// retransmission; we increment `retransmit_count`, refresh
    /// `last_sent`, and call `self.rto.on_retransmit()` to apply Karn's
    /// exponential backoff to the RTO before the next scan.
    ///
    /// `on_retransmit` is called **once per scan**, not per-entry —
    /// the RTO doubles in response to *the channel timing out*, not in
    /// response to each individual packet that happens to be past
    /// deadline at this tick. Doubling per-entry would compound the
    /// backoff catastrophically if the TX window has multiple packets
    /// in flight when the link briefly stalls.
    ///
    /// Returns a list of packets that need to be retransmitted. The caller
    /// is expected to put the returned packets on the wire — so when this
    /// returns a non-empty list we bump `last_sent` too, otherwise a
    /// channel actively retransmitting would still look idle from the
    /// keepalive helper's perspective and emit redundant pings on top of
    /// the retransmits.
    pub fn check_timeouts(&mut self) -> Vec<Packet> {
        let now = Instant::now();
        let timeout = self.rto.current();
        let mut retransmits = Vec::new();

        for entry in self.tx_window.iter_mut() {
            if now.duration_since(entry.last_sent) >= timeout {
                entry.retransmit_count += 1;
                entry.last_sent = now;
                retransmits.push(entry.packet.clone());
            }
        }

        if !retransmits.is_empty() {
            self.last_sent = now;
            self.rto.on_retransmit();
        }
        retransmits
    }

    /// Returns `true` if the channel has exceeded the maximum retry count
    /// or the peer has been silent past `INACTIVITY_TIMEOUT_MS`.
    ///
    /// Reads `last_received`, not `last_sent` — disconnect detection gates
    /// on what the PEER does (or doesn't), not on what we do. If we're
    /// broadcasting world updates to a dead client, our own outbound
    /// traffic shouldn't suppress the silence check.
    ///
    /// The retry check uses `> MAX_RETRIES` (strict), not `>=`, so the
    /// final retransmission gets a full `ACK_TIMEOUT_MS` window to be
    /// ACKed before the channel is reaped. C++ Mercury behaves the same:
    /// the peer dies on the resend timer AFTER the MAX_RETRIES'th retry,
    /// not on the retry itself. Without the strict `>`, a fast tick loop
    /// would prune the channel on the same tick that just fired the
    /// MAX_RETRIES'th retransmit, giving the last attempt zero time to
    /// land.
    pub fn is_timed_out(&self) -> bool {
        let peer_idle_ms = self.last_received.elapsed().as_millis() as u64;

        // Check if any TX entry has BEEN RETRIED past the budget — i.e.,
        // the MAX_RETRIES'th retransmit also timed out without ACK. See
        // the doc above for why this is `>` not `>=`.
        let max_retries_exceeded = self
            .tx_window
            .iter()
            .any(|entry| entry.retransmit_count > consts::MAX_RETRIES);

        // Peer has been silent past the configured tolerance — assume dead.
        let peer_idle_timeout = peer_idle_ms > consts::INACTIVITY_TIMEOUT_MS;

        max_retries_exceeded || peer_idle_timeout
    }

    /// Returns `true` when we haven't sent the peer anything in
    /// `KEEPALIVE_INTERVAL_MS` and a keepalive packet should now go out
    /// to keep NAT mappings warm.
    ///
    /// Reads `last_sent`, not `last_received` — a peer talking to us
    /// doesn't relieve us of the obligation to send something ourselves;
    /// NAT entries time out per direction.
    pub fn keepalive_due(&self) -> bool {
        self.last_sent.elapsed().as_millis() as u64 >= consts::KEEPALIVE_INTERVAL_MS
    }

    /// Mark the send clock as just-now. Use after sending a keepalive (or
    /// any out-of-band packet that doesn't go through `send_packet`).
    pub fn touch_sent(&mut self) {
        self.last_sent = Instant::now();
    }

    /// Mark the receive clock as just-now. Use after observing peer-
    /// originated traffic that doesn't flow through `receive_packet` /
    /// `process_acks` (e.g., raw datagram counted at the socket layer).
    pub fn touch_received(&mut self) {
        self.last_received = Instant::now();
    }
}
