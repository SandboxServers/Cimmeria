//! Reliable UDP channel state machine.
//!
//! Each remote peer gets a dedicated `Channel` that tracks the sliding windows
//! for transmit and receive, handles ACK processing, and manages retransmission
//! timers. The channel lifecycle mirrors the C++ `Mercury::Channel` class.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use cimmeria_common::Result;

use crate::clock::{Clock, SystemClock};
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
    pub raw_bytes: Bytes,
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

    /// Deferred-send queue: packets that hit the [`consts::TX_WINDOW_SIZE`]
    /// cap when first registered, holding bookkeeping for retransmit until
    /// a TX-window slot frees up via [`Self::process_acks`].
    ///
    /// Entries are inserted in caller-allocated sequence order (the same
    /// order the bytes went on the wire) and promoted FIFO into the
    /// TX window when ACKs drain it. Capped at [`consts::MAX_UNSENT_PACKETS`]
    /// so a peer that has stopped acking cannot grow the queue unbounded —
    /// past the cap, `register_sent_packet` returns an error and the
    /// inactivity timeout reaps the channel.
    ///
    /// Replaces the prior "downgrade reliable send to best-effort" path
    /// at the services layer: the bytes that went on the wire stay tracked
    /// here for retransmit, so a lost packet beyond the TX window cap is
    /// no longer silently un-recoverable.
    pub unsent_packets: VecDeque<TxEntry>,

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
    ///
    /// **Cleanup contract:** there is no periodic time-based cleanup.
    /// Partial reassemblies are evicted only when a new bundle arrives
    /// whose sequence range overlaps an in-progress one (handled
    /// inside [`FragmentAssembler::add_fragment`]). When this channel
    /// is dropped, the assembler goes with it, taking any remaining
    /// partials. See `crates/mercury/src/unpacker.rs` module doc for
    /// the full lifecycle + memory-footprint analysis.
    fragment_assembler: FragmentAssembler,

    /// Adaptive retransmission timeout state. Tracks
    /// smoothed RTT + RTT variance for this peer; consulted by
    /// `check_timeouts` to decide when an unacked packet should be
    /// retransmitted, and updated by `process_acks` (clean samples,
    /// Karn's algorithm) and `check_timeouts` (exponential backoff
    /// on retransmit).
    rto: Rto,

    /// Pluggable clock — `SystemClock` in production, `TestClock`
    /// (from `crate::test_harness`) for paired-channel tests that
    /// need deterministic time advancement. Every read of "now"
    /// inside `Channel` flows through this so tests can drive the
    /// keepalive / RTO / inactivity machinery without sleeping.
    clock: Arc<dyn Clock>,
}

impl Channel {
    /// Create a new channel to `remote_addr` in the `Connecting` state,
    /// seeded with the default [`RtoConfig`] and the production
    /// [`SystemClock`].
    pub fn new(remote_addr: SocketAddr) -> Self {
        Self::with_clock_and_rto_config(remote_addr, Arc::new(SystemClock), RtoConfig::default())
    }

    /// Create a new channel with explicit [`RtoConfig`] — primarily for
    /// tests that need tight RTO bounds to keep test runtimes short.
    pub fn with_rto_config(remote_addr: SocketAddr, rto_config: RtoConfig) -> Self {
        Self::with_clock_and_rto_config(remote_addr, Arc::new(SystemClock), rto_config)
    }

    /// Create a new channel with an explicit [`Clock`] but the default
    /// [`RtoConfig`]. The loopback test harness uses this to inject a
    /// `TestClock` while keeping the production RTO bounds.
    pub fn with_clock(remote_addr: SocketAddr, clock: Arc<dyn Clock>) -> Self {
        Self::with_clock_and_rto_config(remote_addr, clock, RtoConfig::default())
    }

    /// Create a new channel with both an explicit [`Clock`] and
    /// [`RtoConfig`]. The full power-user constructor that all the
    /// shorter ones funnel through.
    pub fn with_clock_and_rto_config(
        remote_addr: SocketAddr,
        clock: Arc<dyn Clock>,
        rto_config: RtoConfig,
    ) -> Self {
        let now = clock.now();
        Self {
            state: ChannelState::Connecting,
            tx_window: VecDeque::with_capacity(consts::TX_WINDOW_SIZE),
            // Allocate empty — the queue is only populated when a sustained
            // send burst overflows the TX window, which is rare in steady
            // state. Pre-allocating MAX_UNSENT_PACKETS would waste memory
            // on every channel for a path that fires only under congestion.
            unsent_packets: VecDeque::new(),
            rx_window: VecDeque::with_capacity(consts::RX_WINDOW_SIZE),
            next_tx_seq: 0,
            expected_rx_seq: 0,
            remote_addr,
            last_sent: now,
            last_received: now,
            fragment_assembler: FragmentAssembler::new(),
            rto: Rto::new(rto_config),
            clock,
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
        self.last_received = self.clock.now();
        self.fragment_assembler.process_parsed(pkt)
    }

    /// Register a packet that the caller has already assigned a sequence
    /// number to, encrypted, and put on the wire.
    ///
    /// Used during the gradual services-layer migration to `Channel`-managed
    /// sends. The legacy send path stamps sequences via a session-local
    /// `Arc<AtomicU32>` and encrypts + transmits directly;
    /// this method lets us mirror those sends into the channel's TX window
    /// so ACK tracking, RTO sampling, AND retransmit are all live BEFORE
    /// every send site has migrated to [`send_packet`].
    ///
    /// Caller MUST ensure `packet.sequence` matches the sequence number
    /// that went out on the wire (otherwise the cumulative-ACK drain in
    /// [`process_acks`] won't find the right entry).
    ///
    /// `raw_bytes` is the exact encrypted datagram that was sent. The
    /// retransmit driver re-sends these bytes verbatim — no re-encryption.
    /// Pass `Bytes::new()` to register-without-retransmit-capability (rare;
    /// only useful for shadow-mode observation that never needs to resend).
    ///
    /// **Overflow handling:** when the TX window is at [`consts::TX_WINDOW_SIZE`]
    /// the entry is appended to [`Self::unsent_packets`] instead, where it
    /// stays bookkept until a TX-window slot frees. [`process_acks`] drains
    /// any queued entry whose seq is covered by the cumulative ACK and
    /// promotes the remaining oldest-first into freed slots; only after
    /// promotion is an entry eligible for retransmit by [`check_timeouts`]
    /// (the retransmit scan only walks `tx_window`, not the queue). This
    /// replaces the prior "return Err on overflow" behavior that the
    /// services layer silently downgraded to best-effort.
    ///
    /// Only two error paths remain: an out-of-range sequence number (a
    /// programming bug — the 28-bit Mercury space is the contract), and
    /// the unsent-queue cap being hit ([`consts::MAX_UNSENT_PACKETS`]),
    /// which indicates the peer has stopped acking entirely and the
    /// channel is on its way to the inactivity-timeout reap.
    ///
    /// [`send_packet`]: Self::send_packet
    /// [`process_acks`]: Self::process_acks
    pub fn register_sent_packet(&mut self, packet: Packet, raw_bytes: Bytes) -> Result<()> {
        // Caller-provided sequence must respect the 28-bit Mercury sequence
        // space (`mercury-wire-format` spec §2.4 R4). An out-of-range seq
        // here would corrupt the cumulative-ACK drain (which assumes
        // sequences stay within the same modular space the wire uses).
        if packet.sequence >= crate::packet::NULL_SEQUENCE {
            return Err(cimmeria_common::CimmeriaError::Channel(format!(
                "register_sent_packet: caller-assigned seq=0x{:08X} is outside the 28-bit valid range (max 0x{:08X})",
                packet.sequence,
                crate::packet::NULL_SEQUENCE - 1,
            )));
        }

        let now = self.clock.now();
        let entry = TxEntry {
            packet,
            last_sent: now,
            retransmit_count: 0,
            raw_bytes,
        };

        if self.tx_window.len() < consts::TX_WINDOW_SIZE {
            self.tx_window.push_back(entry);
            self.last_sent = now;
            return Ok(());
        }

        // TX window full — defer to the unsent-packets queue. The packet
        // bytes already went on the wire; this preserves retransmit
        // bookkeeping for the case where the original send is lost. See
        // the field doc on `unsent_packets` for the drain/promotion path.
        if self.unsent_packets.len() >= consts::MAX_UNSENT_PACKETS {
            // The packet bytes already went on the wire; only the
            // retransmit/ACK bookkeeping is being rejected here. The
            // peer-side outcome is "we sent it once; if it was lost we
            // cannot recover it" — which is fine because the inactivity
            // timer is about to reap this channel.
            return Err(cimmeria_common::CimmeriaError::Channel(format!(
                "unsent-packets queue full ({} packets); channel likely dead, awaiting inactivity reap. bookkeeping rejected for seq={} (datagram already on wire, no retransmit possible)",
                self.unsent_packets.len(),
                entry.packet.sequence,
            )));
        }
        self.unsent_packets.push_back(entry);
        // Update `last_sent` even on the queued path — the bytes went out
        // on the wire, so for keepalive-timing purposes the channel was
        // active in the send direction.
        self.last_sent = now;
        Ok(())
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

        // Stamp the outgoing sequence number onto the packet. Mask with
        // `SEQUENCE_MASK` (28 bits) so the counter can't overflow into
        // the `NULL_SEQUENCE` sentinel range. The wrap-and-mask combo
        // keeps the counter within the spec's 28-bit space across
        // `u32::MAX / SEQUENCE_MASK` cycles.
        packet.sequence = self.next_tx_seq & crate::packet::SEQUENCE_MASK;
        self.next_tx_seq = self.next_tx_seq.wrapping_add(1) & crate::packet::SEQUENCE_MASK;

        // Instrument: outbound UDP packet bookkeeping. Emitted post-seq
        // stamping so the recorded `seq` matches what'll go on the wire.
        // (Note: the bytes haven't actually been sent yet — caller still
        // needs to encrypt + transmit — but they will, and the wire-side
        // retransmit is keyed off this same `Packet`, so this is the
        // single source-of-truth seam for "we attempted to send seq=X".)
        crate::instrumentation::record_udp_packet(
            crate::instrumentation::Direction::Out,
            packet.sequence,
            packet.flags.0,
            packet.body.len(),
            self.remote_addr,
        );

        let now = self.clock.now();
        self.tx_window.push_back(TxEntry {
            packet,
            last_sent: now,
            retransmit_count: 0,
            // send_packet entries have no raw bytes — caller hasn't
            // encrypted yet (the seq was just stamped). Such entries
            // are silently skipped during the retransmit scan.
            raw_bytes: Bytes::new(),
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
        self.last_received = self.clock.now();

        // Instrument: inbound UDP packet observed. We record EVERY
        // received packet here — including duplicates and packets
        // beyond the window — because those are exactly the kind of
        // anomalies the analytical store needs to surface. Filtering
        // duplicates out at the record site would hide them in the
        // SigNoz "incoming packet rate" plot.
        crate::instrumentation::record_udp_packet(
            crate::instrumentation::Direction::In,
            seq,
            packet.flags.0,
            packet.body.len(),
            self.remote_addr,
        );

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
    ///
    /// Also drains any [`Self::unsent_packets`] entries covered by the
    /// cumulative ACK (their bytes went on the wire when they were
    /// queued, so the peer can ack them while they're still queued), and
    /// promotes oldest-first from the queue into freed TX-window slots so
    /// the retransmit scan can pick them up if needed. Promoted entries
    /// keep their original `last_sent` so an entry that sat in the queue
    /// past its RTO will be retransmitted on the next [`Self::check_timeouts`]
    /// pass.
    pub fn process_acks(&mut self, ack_seq: u32) -> Result<()> {
        // ACK frames are peer-originated → counts as receive-side activity,
        // not send-side. (We aren't putting bytes on the wire; we're
        // observing the peer's response to a prior emit.)
        let now = self.clock.now();
        self.last_received = now;

        let ack_seq_masked = ack_seq & crate::packet::SEQUENCE_MASK;

        // 28-bit modular `seq <= ack_seq` comparator. Used to drain both
        // tx_window and unsent_packets — both are ordered by allocation
        // sequence so a front-of-deque check suffices.
        //
        // Mercury sequence space is 28 bits (`SEQUENCE_MASK = 0x0FFF_FFFF`),
        // not 32. The modular `<=` comparison must use the 28-bit
        // half-range (`1 << 27` = `0x0800_0000`), not the 32-bit one.
        // Mask both operands and the difference to the 28-bit space so
        // wraparound past `0x0FFF_FFFF` is interpreted correctly:
        //   front.seq=0x0FFF_FFFE, ack=0x0000_0001 (post-wrap):
        //     32-bit cmp:  diff = 0x0FFF_FFFD; diff > 0x8000_0000 = false
        //                  → STOP (wrong — front IS <= ack post-wrap)
        //     28-bit cmp:  diff = 0x0FFF_FFFD & MASK = 0x0FFF_FFFD;
        //                  0x0FFF_FFFD > 0x0800_0000 = true → DRAIN (correct)
        let covered = |seq: u32| -> bool {
            let seq_masked = seq & crate::packet::SEQUENCE_MASK;
            let diff = seq_masked.wrapping_sub(ack_seq_masked) & crate::packet::SEQUENCE_MASK;
            diff == 0 || diff > 0x0800_0000
        };

        // Cumulative ACK: remove all TX entries with sequence <= ack_seq.
        // The tx_window is ordered by sequence (oldest at front), so we can
        // drain from the front until we hit a sequence beyond the ACK.
        while let Some(front) = self.tx_window.front() {
            if covered(front.packet.sequence) {
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

        // Same drain logic against unsent_packets: queued entries can be
        // acked too. No RTT sample here — queued entries are by
        // definition past the trivial "just sent" case and the timing
        // wouldn't be informative.
        while let Some(front) = self.unsent_packets.front() {
            if covered(front.packet.sequence) {
                self.unsent_packets.pop_front();
            } else {
                break;
            }
        }

        // Promote oldest queued entries into the freed TX-window slots so
        // the retransmit scan can fire on them. Preserve `last_sent` —
        // if the entry sat in the queue past RTO, the next
        // `check_timeouts` pass will retransmit it (correct: bytes went
        // on the wire when queued, no ack means lost in flight).
        while self.tx_window.len() < consts::TX_WINDOW_SIZE {
            match self.unsent_packets.pop_front() {
                Some(entry) => self.tx_window.push_back(entry),
                None => break,
            }
        }

        Ok(())
    }

    /// Check all packets in the TX window for retransmission timeouts.
    ///
    /// Uses the per-channel adaptive RTO — `self.rto.current()` — rather
    /// than a fixed `consts::ACK_TIMEOUT_MS`. Each expired entry
    /// (whose `last_sent` is older than the current RTO) is selected for
    /// retransmission: we increment `retransmit_count`, refresh
    /// `last_sent`, and emit the entry's cached encrypted bytes for the
    /// caller to put on the wire. After the scan, if anything was
    /// retransmitted, `self.rto.on_retransmit()` doubles the RTO once
    /// (Karn's backoff).
    ///
    /// **Per-tick budget (#292 #6):** the scan stops after
    /// `consts::RETRANSMIT_BUDGET_PER_TICK` expired entries have been
    /// processed. The remaining expired entries (if any) wait for the
    /// next tick. Without this cap, a saturated link with 32 in-flight
    /// reliable packets and a brief stall would have us blast all 32
    /// retransmits in a single tick — likely worsening the congestion
    /// rather than recovering from it.
    ///
    /// **on_retransmit fires ONCE per scan**, not per-entry. The RTO
    /// doubles in response to *the channel timing out*, not in response
    /// to each individual packet past deadline. Doubling per-entry
    /// would compound the backoff catastrophically.
    ///
    /// **Bytes-empty entries are silently skipped** (their
    /// `retransmit_count` IS still bumped). Those are entries inserted
    /// via the deprecated `send_packet` path, which doesn't carry the
    /// encrypted bytes needed to resend. They drain normally on ACK,
    /// just can't be resent on loss.
    ///
    /// Returns a `Vec<Bytes>` of encrypted-and-ready-to-wire datagrams.
    /// The caller `socket.send_to`s each one to `self.remote_addr`.
    pub fn check_timeouts(&mut self) -> Vec<Bytes> {
        let now = self.clock.now();
        let timeout = self.rto.current();
        let mut retransmits = Vec::new();
        let mut budget = consts::RETRANSMIT_BUDGET_PER_TICK;
        // Track whether *any* expired entry was processed (regardless of
        // whether it had bytes available to resend), so the channel-level
        // side effects (RTO backoff + last_sent bump) fire on the
        // semantic event "the channel timed out", not the narrower event
        // "we had bytes to put on the wire". A channel full of bytes-empty
        // entries (`send_packet` legacy path) would otherwise never back off.
        let mut any_expired = false;

        for entry in self.tx_window.iter_mut() {
            if budget == 0 {
                break;
            }
            if now.duration_since(entry.last_sent) < timeout {
                continue;
            }
            // Expired — count against budget and refresh.
            any_expired = true;
            budget -= 1;
            entry.retransmit_count += 1;
            entry.last_sent = now;
            // Stable retransmit event for SigNoz: lets operators ask
            // "is the network actually lossy?" by counting events with
            // `target = "mercury.retransmit"` per peer / per minute.
            // info level — these are rare in a healthy channel; a
            // sustained rate indicates real loss or a stalled peer.
            tracing::info!(
                target: "mercury.retransmit",
                peer = %self.remote_addr,
                seq = entry.packet.sequence,
                retransmit_count = entry.retransmit_count,
                rto_ms = timeout.as_millis() as u64,
                "Mercury: reliable packet retransmit"
            );
            if !entry.raw_bytes.is_empty() {
                retransmits.push(entry.raw_bytes.clone());
            }
        }

        if any_expired {
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
        let peer_idle_ms = self
            .clock
            .now()
            .duration_since(self.last_received)
            .as_millis() as u64;

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
        self.clock.now().duration_since(self.last_sent).as_millis() as u64
            >= consts::KEEPALIVE_INTERVAL_MS
    }

    /// Mark the send clock as just-now. Use after sending a keepalive (or
    /// any out-of-band packet that doesn't go through `send_packet`).
    pub fn touch_sent(&mut self) {
        self.last_sent = self.clock.now();
    }

    /// Mark the receive clock as just-now. Use after observing peer-
    /// originated traffic that doesn't flow through `receive_packet` /
    /// `process_acks` (e.g., raw datagram counted at the socket layer).
    pub fn touch_received(&mut self) {
        self.last_received = self.clock.now();
    }

    /// Read-only access to the channel's clock. The harness uses this
    /// to advance a `TestClock` from outside without a back-channel.
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

// ── Tick driver contract ────────────────────────────────────────────────────

/// Punch list returned by a tick pass over a collection of channels.
///
/// Cimmeria's actual tick driver lives in
/// `crates/services/src/base/tick_sync.rs` — a per-session task that drives
/// one channel per loop iteration via [`Channel::check_timeouts`],
/// [`Channel::keepalive_due`], and [`Channel::is_timed_out`] directly. This
/// type exists to document the bridge contract for any future driver that
/// needs to fan a single tick across many channels (e.g., a registry-style
/// loop). The fields are the I/O punch list; ownership of the actual send /
/// teardown lives with the caller.
///
/// **Ordering invariant the caller must preserve** (mirrors C++ BigWorld's
/// `BaseNub::processPendingEvents` order, and the per-session loop in
/// `tick_sync.rs` matches it implicitly because it only sees one channel):
///
///   1. **Prune dead channels first.** [`Channel::is_timed_out`] uses
///      strict `>` against `consts::MAX_RETRIES`, so a packet whose
///      retransmit_count was bumped TO `MAX_RETRIES` on the previous tick
///      gets a full `ACK_TIMEOUT_MS` window to land before this tick reaps
///      the channel. Pruning before collecting retransmits prevents queuing
///      work for a channel we're about to throw away.
///   2. **Collect retransmits** via [`Channel::check_timeouts`]. That call
///      bumps each touched entry's `retransmit_count` and the channel's
///      `last_sent` — so a channel actively retransmitting will NOT also be
///      flagged for a keepalive in step 3.
///   3. **Schedule keepalives** via [`Channel::keepalive_due`]. The
///      contract is intentionally lazy: a tick that emits a keepalive does
///      NOT call [`Channel::touch_sent`] eagerly — the caller is expected
///      to call it after the bytes actually go on the wire. If the I/O
///      layer drops the action, the next tick re-flags the same channel
///      rather than silently suppressing the keepalive for a full interval.
///
/// **Not done at this layer:** there is intentionally no fragment-reassembly
/// sweep. Per `mercury-wire-format` spec §2.4.1 R13 + §2.10 S6, abandoned
/// reassemblies are evicted only when a new overlapping bundle arrives
/// (handled inside [`crate::unpacker::FragmentAssembler::add_fragment`]) or
/// when the channel itself is torn down. An earlier implementation ran a
/// 30s periodic sweep that silently dropped in-progress reassemblies the
/// client would have kept; that is gone.
#[derive(Default)]
pub struct TickActions {
    /// Reliable packet datagrams (already encrypted) that hit their
    /// channel's adaptive RTO without being acked and need to go back
    /// on the wire. Pre-bound to the destination address so the caller
    /// `socket.send_to`s each pair directly — no re-encryption needed.
    pub retransmits: Vec<(SocketAddr, Bytes)>,
    /// Channels that haven't sent anything in `KEEPALIVE_INTERVAL_MS`.
    /// The caller emits a keepalive to each of these addresses and is
    /// expected to call [`Channel::touch_sent`] after the bytes land on
    /// the wire — a dropped action will be re-flagged on the next tick
    /// rather than silently suppressed for a full interval.
    pub keepalives: Vec<SocketAddr>,
    /// Channels removed from the registry this pass (silent peers past
    /// `INACTIVITY_TIMEOUT_MS`, or any channel that exceeded
    /// `MAX_RETRIES` on a reliable packet). Returned so the caller can
    /// run cleanup against per-session state outside the channel itself.
    pub dead_channels: Vec<(SocketAddr, Channel)>,
}
