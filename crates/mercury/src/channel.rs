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
}

impl Channel {
    /// Create a new channel to `remote_addr` in the `Connecting` state.
    pub fn new(remote_addr: SocketAddr) -> Self {
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
        }
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
    /// Removes acknowledged packets from the TX window and resets
    /// retransmit timers for selectively NACKed packets.
    pub fn process_acks(&mut self, ack_seq: u32) -> Result<()> {
        // ACK frames are peer-originated → counts as receive-side activity,
        // not send-side. (We aren't putting bytes on the wire; we're
        // observing the peer's response to a prior emit.)
        self.last_received = Instant::now();

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
    /// Returns a list of packets that need to be retransmitted. The caller
    /// is expected to put the returned packets on the wire — so when this
    /// returns a non-empty list we bump `last_sent` too, otherwise a
    /// channel actively retransmitting would still look idle from the
    /// keepalive helper's perspective and emit redundant pings on top of
    /// the retransmits.
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

        if !retransmits.is_empty() {
            self.last_sent = now;
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
    pub fn is_timed_out(&self) -> bool {
        let peer_idle_ms = self.last_received.elapsed().as_millis() as u64;

        // Check if any TX entry has exceeded max retries.
        let max_retries_exceeded = self.tx_window.iter().any(|entry| {
            entry.retransmit_count >= consts::MAX_RETRIES
        });

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

    // ── Split last_activity into last_sent / last_received ─────────────

    #[test]
    fn send_packet_updates_only_last_sent() {
        let mut ch = Channel::new(test_addr());
        // Backdate both clocks to a known past time so we can detect which
        // one moves forward.
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.send_packet(test_packet()).unwrap();

        assert!(ch.last_sent > baseline, "send_packet must reset last_sent");
        assert_eq!(ch.last_received, baseline, "send_packet must NOT touch last_received");
    }

    #[test]
    fn receive_packet_updates_only_last_received() {
        use bytes::Bytes;
        use crate::packet::PacketFlags;

        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        // Inbound packet at expected_rx_seq=0.
        let pkt = Packet::new(PacketFlags::default(), 0, Bytes::from_static(&[0xAB]));
        ch.receive_packet(pkt).unwrap();

        assert!(ch.last_received > baseline, "receive_packet must reset last_received");
        assert_eq!(ch.last_sent, baseline, "receive_packet must NOT touch last_sent");
    }

    #[test]
    fn process_acks_updates_only_last_received() {
        let mut ch = Channel::new(test_addr());
        // Need a packet in flight for the ACK to drain.
        ch.send_packet(test_packet()).unwrap();

        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.process_acks(0).unwrap();

        // ACK is peer-originated data — counts as receive, not send.
        assert!(ch.last_received > baseline, "process_acks must reset last_received");
        assert_eq!(ch.last_sent, baseline, "process_acks must NOT touch last_sent");
    }

    #[test]
    fn keepalive_due_when_we_havent_sent_in_a_while() {
        let mut ch = Channel::new(test_addr());
        // Force last_sent into the past beyond KEEPALIVE_INTERVAL_MS, but
        // keep last_received fresh (peer is talking to us).
        ch.last_sent = std::time::Instant::now()
            - std::time::Duration::from_millis(consts::KEEPALIVE_INTERVAL_MS + 100);
        ch.last_received = std::time::Instant::now();

        // Inbound peer traffic must NOT suppress our send-side keepalive
        // — NAT entries time out per direction.
        assert!(ch.keepalive_due(),
            "keepalive must be due based on OUR send-side silence, regardless of peer activity");
    }

    #[test]
    fn keepalive_not_due_right_after_sending() {
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        // Just sent; no keepalive needed for at least KEEPALIVE_INTERVAL_MS.
        assert!(!ch.keepalive_due());
    }

    #[test]
    fn is_timed_out_fires_on_silent_peer_even_if_we_keep_sending() {
        let mut ch = Channel::new(test_addr());
        // Simulate "we keep blasting world updates at a dead client":
        // last_sent is fresh, but last_received is stale past the
        // configured INACTIVITY_TIMEOUT_MS.
        ch.last_sent = std::time::Instant::now();
        ch.last_received = std::time::Instant::now()
            - std::time::Duration::from_millis(consts::INACTIVITY_TIMEOUT_MS + 100);

        // Conflated `last_activity` would have been refreshed by our own
        // sends — so dead clients would never be reaped. Splitting the
        // clocks closes that hole.
        assert!(ch.is_timed_out(),
            "is_timed_out must trigger on peer silence regardless of our outgoing traffic");
    }

    #[test]
    fn is_timed_out_does_not_fire_when_peer_is_chatty() {
        let mut ch = Channel::new(test_addr());
        // Both sides active recently — nothing to disconnect.
        ch.last_sent = std::time::Instant::now();
        ch.last_received = std::time::Instant::now();
        assert!(!ch.is_timed_out());
    }

    #[test]
    fn touch_sent_only_moves_send_clock() {
        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.touch_sent();

        assert!(ch.last_sent > baseline, "touch_sent must move last_sent");
        assert_eq!(ch.last_received, baseline, "touch_sent must NOT move last_received");
    }

    #[test]
    fn touch_received_only_moves_receive_clock() {
        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;
        ch.last_received = baseline;

        ch.touch_received();

        assert!(ch.last_received > baseline, "touch_received must move last_received");
        assert_eq!(ch.last_sent, baseline, "touch_received must NOT move last_sent");
    }

    #[test]
    fn check_timeouts_bumps_last_sent_when_retransmitting() {
        // A channel that's actively retransmitting a lost reliable packet
        // is putting bytes on the wire — the keepalive helper must observe
        // that activity and not emit redundant pings on top of the retries.
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        // Backdate both: the entry's last_sent so check_timeouts sees it as
        // expired, AND Channel.last_sent so we can detect whether it moves.
        let backdate = std::time::Instant::now() - std::time::Duration::from_millis(800);
        ch.tx_window[0].last_sent = backdate;
        ch.last_sent = backdate;

        let retransmits = ch.check_timeouts();
        assert_eq!(retransmits.len(), 1, "expired packet should be retransmitted");
        assert!(ch.last_sent > backdate,
            "check_timeouts emitting retransmits must bump Channel.last_sent");
    }

    #[test]
    fn check_timeouts_does_not_bump_last_sent_when_no_retransmits() {
        // A no-op pass (nothing expired) shouldn't perturb the keepalive
        // timer — otherwise the periodic tick would silently keep the
        // channel "active" forever.
        let mut ch = Channel::new(test_addr());
        ch.send_packet(test_packet()).unwrap();
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_sent = baseline;

        let retransmits = ch.check_timeouts();
        assert!(retransmits.is_empty());
        assert_eq!(ch.last_sent, baseline,
            "no-op check_timeouts must leave last_sent untouched");
    }

    // ── Per-channel fragment reassembly ──────────────────────────────

    /// Build then parse a fragmented Mercury packet — same helper shape
    /// `unpacker::tests` uses, duplicated here because the inner module
    /// is private.
    fn build_then_parse_fragment(
        seq: u32,
        frag_begin: u32,
        frag_end: u32,
        body: &[u8],
    ) -> crate::packet::ParsedPacket {
        use crate::packet::{build_outgoing_fragmented, parse_incoming};
        let raw = build_outgoing_fragmented(0, body, seq, frag_begin, frag_end, &[]);
        parse_incoming(&raw).unwrap()
    }

    #[test]
    fn reassemble_parsed_passes_through_non_fragmented() {
        use crate::packet::{build_outgoing, parse_incoming, FLAG_HAS_SEQUENCE};
        let mut ch = Channel::new(test_addr());
        let raw = build_outgoing(FLAG_HAS_SEQUENCE, b"hello", Some(7), &[], None);
        let parsed = parse_incoming(&raw).unwrap();

        let body = ch.reassemble_parsed(&parsed).unwrap()
            .expect("non-fragmented should pass through");
        assert_eq!(body.as_ref(), b"hello");
    }

    #[test]
    fn reassemble_parsed_completes_3_fragment_bundle() {
        let mut ch = Channel::new(test_addr());
        let f0 = build_then_parse_fragment(10, 10, 12, b"AAA");
        let f1 = build_then_parse_fragment(11, 10, 12, b"BBB");
        let f2 = build_then_parse_fragment(12, 10, 12, b"CCC");

        assert!(ch.reassemble_parsed(&f0).unwrap().is_none());
        assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
        let body = ch.reassemble_parsed(&f2).unwrap().expect("third fragment completes");
        assert_eq!(body.as_ref(), b"AAABBBCCC");
    }

    #[test]
    fn reassemble_parsed_bumps_last_received() {
        // Receive-side observation must move last_received so the
        // peer-silence detector sees fragment activity as keepalive-
        // equivalent. Without this, a peer streaming a large bundle of
        // fragments would still look idle until the bundle assembled.
        let mut ch = Channel::new(test_addr());
        let baseline = std::time::Instant::now() - std::time::Duration::from_secs(10);
        ch.last_received = baseline;
        ch.last_sent = baseline;

        let f0 = build_then_parse_fragment(20, 20, 21, b"part-one");
        ch.reassemble_parsed(&f0).unwrap();

        assert!(ch.last_received > baseline, "fragment receive must move last_received");
        assert_eq!(ch.last_sent, baseline, "fragment receive must NOT move last_sent");
    }

    #[test]
    fn reassemble_parsed_isolates_per_channel_state() {
        // Two channels with overlapping fragment seq ranges must NOT
        // share reassembly buffers — that's the whole point of putting
        // the assembler on the channel rather than the Nub.
        let mut a = Channel::new("127.0.0.1:8001".parse().unwrap());
        let mut b = Channel::new("127.0.0.1:8002".parse().unwrap());

        let a0 = build_then_parse_fragment(50, 50, 51, b"a-part-1");
        let a1 = build_then_parse_fragment(51, 50, 51, b"a-part-2");
        // b uses the SAME seq range (50..=51) — an unscoped assembler
        // would conflate b's fragments with a's, flush a partial bundle
        // early, or error on conflicting total_frags.
        let b0 = build_then_parse_fragment(50, 50, 51, b"BBB-1");

        assert!(a.reassemble_parsed(&a0).unwrap().is_none());
        // b's fragment must not affect a's pending state.
        assert!(b.reassemble_parsed(&b0).unwrap().is_none());
        let a_body = a.reassemble_parsed(&a1).unwrap().expect("a's bundle completes");
        assert_eq!(a_body.as_ref(), b"a-part-1a-part-2",
            "channel a must reassemble its own fragments without b's interference");
    }

    #[test]
    fn cleanup_stale_fragments_drops_partial_bundles() {
        let mut ch = Channel::new(test_addr());
        let f0 = build_then_parse_fragment(40, 40, 42, b"only-one");
        ch.reassemble_parsed(&f0).unwrap();

        ch.cleanup_stale_fragments(std::time::Duration::ZERO);

        // Subsequent re-receipt of f0 must start fresh — if the partial
        // bundle wasn't reaped, the assembler would silently treat the
        // re-arrival as a duplicate-fragment dedup and never complete.
        let f1 = build_then_parse_fragment(41, 40, 42, b"two");
        let f2 = build_then_parse_fragment(42, 40, 42, b"three");
        assert!(ch.reassemble_parsed(&f0).unwrap().is_none());
        assert!(ch.reassemble_parsed(&f1).unwrap().is_none());
        let body = ch.reassemble_parsed(&f2).unwrap().expect("post-cleanup bundle completes");
        assert_eq!(body.as_ref(), b"only-onetwothree");
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
