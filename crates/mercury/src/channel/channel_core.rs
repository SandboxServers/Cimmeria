//! The [`Channel`] reliable-UDP state machine: sliding TX/RX windows, ACK
//! processing, adaptive RTO retransmission, keepalive, and inactivity
//! detection.
//!
//! The plain bookkeeping records ([`ChannelState`], [`TxEntry`], [`RxEntry`])
//! live in [`super::state`]; the RTO retransmit scan and dead-peer check in
//! [`super::retransmit`]; the multi-channel tick punch-list
//! ([`super::TickActions`]) lives in [`super::tick`].

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use cimmeria_common::Result;

use crate::clock::{Clock, SystemClock};
use crate::consts;
use crate::packet::Packet;
use crate::unpacker::FragmentAssembler;

use super::rto::{Rto, RtoConfig};
use super::state::{AbandonedPacket, ChannelState, RxEntry, TxEntry, TxHole};

/// True if `a` is sequence-before-or-equal `b` in Mercury's 28-bit modular
/// sequence space (`SEQUENCE_MASK = 0x0FFF_FFFF`).
///
/// Shared by [`Channel::register_sent_packet`]'s sorted insert and the
/// transmit-hole tracking in [`super::ack`] — both need the same "is this
/// seq at or before that one, allowing for wraparound" comparator. A plain
/// `a <= b` on the raw `u32`s breaks the moment either operand wraps past
/// `NULL_SEQUENCE`; masking the difference to the 28-bit half range
/// (`0x0800_0000`) interprets wraparound correctly:
///
/// ```text
/// a = 0x0FFF_FFFE, b = 0x0000_0001 (b is after the wrap)
///   32-bit: diff = 0x0FFF_FFFD, > 0x8000_0000 is false -> "a after b" (wrong)
///   28-bit: diff = 0x0FFF_FFFD, > 0x0800_0000 is true  -> "a before b" (right)
/// ```
pub(super) fn seq_mod_leq(a: u32, b: u32) -> bool {
    let a = a & crate::packet::SEQUENCE_MASK;
    let b = b & crate::packet::SEQUENCE_MASK;
    let diff = a.wrapping_sub(b) & crate::packet::SEQUENCE_MASK;
    diff == 0 || diff > 0x0800_0000
}

/// Insert `entry` into `deque` at the position that keeps it sorted by
/// `packet.sequence` (oldest/lowest at the front), per [`seq_mod_leq`].
///
/// Scans backward from the tail rather than the head: the overwhelming
/// common case is that sends reach `register_sent_packet` in allocation
/// order, so the new entry belongs at the back and this is O(1). Only a
/// genuine cross-task race (see the call sites) pays the O(n) scan, and
/// `tx_window`/`unsent_packets` are small (32 / 1024 cap) even then.
fn insert_tx_entry_sorted(deque: &mut VecDeque<TxEntry>, entry: TxEntry) {
    let seq = entry.packet.sequence;
    let mut idx = deque.len();
    while idx > 0 && !seq_mod_leq(deque[idx - 1].packet.sequence, seq) {
        idx -= 1;
    }
    deque.insert(idx, entry);
}

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
    /// a TX-window slot frees up via [`Self::process_ack`].
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

    /// Inbound reliable packets buffered for ordered delivery. Slot `i`
    /// holds sequence `expected_rx_seq + i`. See [`super::rx_order`].
    pub rx_window: VecDeque<Option<RxEntry>>,

    /// Next sequence number to assign to an outbound packet.
    pub next_tx_seq: u32,

    /// Next reliable sequence number we expect to receive from the peer.
    /// Meaningful once [`Self::rx_anchored`] is true.
    pub expected_rx_seq: u32,

    /// Whether `expected_rx_seq` has been set, by
    /// [`Self::anchor_rx_seq`] or by adopting the first reliable packet
    /// (the client's `inSeqAt == SEQ_NULL` start state).
    pub(super) rx_anchored: bool,

    /// When the current reliable gap started blocking delivery: set when
    /// a packet is buffered behind a gap, cleared when the window drains.
    /// Read by [`Self::check_rx_stall`].
    pub(super) rx_gap_since: Option<Instant>,

    /// When the stall watchdog last warned about the current gap.
    pub(super) rx_stall_warned_at: Option<Instant>,

    /// Distinct reliable gaps that blocked delivery past
    /// [`consts::RX_STALL_WARN_MS`] over this channel's life.
    pub rx_stalls: u64,

    /// Highest sequence the peer has acked, in 28-bit modular order.
    /// Only ACKs that retire an outstanding entry move it. Read by the
    /// transmit-hole tracking in [`super::ack`].
    pub(super) highest_acked: Option<u32>,

    /// The transmit hole currently open: the oldest outstanding reliable
    /// packet, when the peer has already acked a later one. See
    /// [`super::ack`].
    pub(super) tx_hole: Option<TxHole>,

    /// Distinct transmit holes opened over this channel's life. Each one
    /// is a reliable packet the peer received a successor of first: lost
    /// in flight, or reordered.
    pub tx_holes: u64,

    /// Distinct transmit holes that stayed open past
    /// [`consts::TX_HOLE_WARN_MS`] over this channel's life.
    pub tx_hole_stalls: u64,

    /// Retransmits sent over this channel's life, bumped once per expired
    /// entry in [`Self::check_timeouts`]. Read through [`Self::health`].
    pub retransmits_total: u64,

    /// Capped entries dropped unacked since the last
    /// [`Self::take_abandoned`]. See [`super::retransmit_cap`].
    pub(super) abandoned: Vec<AbandonedPacket>,

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
    /// `receive_packet`, `process_ack` (an ACK frame is peer data),
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
    /// Partial reassemblies are evicted when a new bundle arrives whose
    /// sequence range overlaps an in-progress one, or when the
    /// per-channel caps ([`consts::MAX_PENDING_FRAGMENTED_BUNDLES`],
    /// [`consts::MAX_PENDING_FRAGMENT_BYTES`]) are reached (both handled
    /// inside [`FragmentAssembler::add_fragment`]). When this channel
    /// is dropped, the assembler goes with it, taking any remaining
    /// partials. See the `crate::unpacker` module doc for the full
    /// lifecycle and the cap policy.
    pub(super) fragment_assembler: FragmentAssembler,

    /// Reassembly cap drops not yet reported in a WARN, and when the
    /// last WARN went out. See [`super::fragment_caps`].
    pub(super) fragment_cap_unreported: crate::unpacker::FragmentCapHits,
    pub(super) fragment_cap_warned_at: Option<Instant>,

    /// Incomplete fragmented bundles dropped by the reassembly caps over
    /// this channel's life.
    pub fragment_cap_drops: u64,

    /// Adaptive retransmission timeout state. Tracks
    /// smoothed RTT + RTT variance for this peer; consulted by
    /// `check_timeouts` to decide when an unacked packet should be
    /// retransmitted, and updated by `process_ack` (clean samples,
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
            // Allocate empty — the window only fills behind a gap.
            rx_window: VecDeque::new(),
            next_tx_seq: 0,
            expected_rx_seq: 0,
            rx_anchored: false,
            rx_gap_since: None,
            rx_stall_warned_at: None,
            rx_stalls: 0,
            highest_acked: None,
            tx_hole: None,
            tx_holes: 0,
            tx_hole_stalls: 0,
            retransmits_total: 0,
            abandoned: Vec::new(),
            remote_addr,
            last_sent: now,
            last_received: now,
            fragment_assembler: FragmentAssembler::new(),
            fragment_cap_unreported: Default::default(),
            fragment_cap_warned_at: None,
            fragment_cap_drops: 0,
            rto: Rto::new(rto_config),
            clock,
        }
    }

    /// Read-only access to the per-channel RTO state. Diagnostics /
    /// logging only; the algorithm itself is internal to `Channel`.
    pub fn rto(&self) -> &Rto {
        &self.rto
    }

    /// Mutable RTO state, for the ACK path in [`super::ack`] (clean RTT
    /// samples).
    pub(super) fn rto_mut(&mut self) -> &mut Rto {
        &mut self.rto
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
    /// that went out on the wire: [`process_ack`] retires the entry whose
    /// sequence the peer's ACK names, so a mismatched one is never retired
    /// and is retransmitted until the channel times out.
    ///
    /// `raw_bytes` is the exact encrypted datagram that was sent. The
    /// retransmit driver re-sends these bytes verbatim — no re-encryption.
    /// Pass `Bytes::new()` to register-without-retransmit-capability (rare;
    /// only useful for shadow-mode observation that never needs to resend).
    ///
    /// **Overflow handling:** when the TX window is at [`consts::TX_WINDOW_SIZE`]
    /// the entry is appended to [`Self::unsent_packets`] instead, where it
    /// stays bookkept until a TX-window slot frees. [`process_ack`] retires
    /// a queued entry the peer acks directly and promotes the remaining
    /// oldest-first into freed slots; only after
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
    /// [`process_ack`]: Self::process_ack
    pub fn register_sent_packet(&mut self, packet: Packet, raw_bytes: Bytes) -> Result<()> {
        // Caller-provided sequence must respect the 28-bit Mercury sequence
        // space (`mercury-wire-format` spec §2.4 R4). An out-of-range seq
        // here would corrupt the sorted windows and the transmit-hole
        // tracking (both assume sequences stay within the same modular
        // space the wire uses).
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
            first_sent: now,
            last_sent: now,
            retransmit_count: 0,
            raw_bytes,
            send_site: None,
            send_kind: None,
            fragment_index: None,
            fragment_count: None,
            message_count: None,
            first_message: None,
            retransmit_cap: None,
        };

        if self.tx_window.len() < consts::TX_WINDOW_SIZE {
            // Sorted insert, not `push_back` (NA39). The services layer
            // reserves a sequence number (an atomic fetch-add under a
            // briefly-held, *different* lock) and only calls back in here
            // after the async socket send completes — a gap wide enough
            // for a second, genuinely concurrent task (base's recv-loop,
            // a `tokio::spawn`ed fan-out, cell's witness dispatch) to
            // reserve a LATER sequence for the same witness and still
            // win the race to `register_sent_packet`. `push_back` would
            // then leave `tx_window` non-monotonic. ACK processing no
            // longer depends on the order (it retires the exact sequence
            // an ACK names), but the retransmit scan resends oldest-first
            // within its per-tick budget, promotion is FIFO, and the
            // transmit-hole tracking reads the front as "oldest
            // outstanding", so the window is kept sorted regardless of
            // registration order.
            insert_tx_entry_sorted(&mut self.tx_window, entry);
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
        // Same sorted-insert rationale as the tx_window branch above —
        // promotion takes this queue front-first.
        insert_tx_entry_sorted(&mut self.unsent_packets, entry);
        // Update `last_sent` even on the queued path — the bytes went out
        // on the wire, so for keepalive-timing purposes the channel was
        // active in the send direction.
        self.last_sent = now;
        Ok(())
    }

    /// Attach the service dispatch site to a packet registered after its
    /// socket send. Called while the session lock is still held, before an
    /// ACK can retire the entry.
    pub fn set_sent_packet_context(
        &mut self,
        seq: u32,
        site: String,
        kind: &'static str,
        fragment: Option<(usize, usize)>,
        message_count: Option<usize>,
        first_message: Option<crate::packet::MessageHead>,
    ) {
        if let Some(entry) = self
            .tx_window
            .iter_mut()
            .chain(self.unsent_packets.iter_mut())
            .find(|e| e.packet.sequence == seq)
        {
            entry.send_site = Some(site);
            entry.send_kind = Some(kind);
            if let Some((index, count)) = fragment {
                entry.fragment_index = Some(index);
                entry.fragment_count = Some(count);
            }
            entry.message_count = message_count;
            entry.first_message = first_message;
        }
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

        // Backpressure observability — warn at 50% / 75% / 90% window
        // saturation so SigNoz surfaces stalled clients before the TX
        // window is fully exhausted. The freeze investigation (2026-05-26)
        // showed an existing-character login flood that filled the
        // window faster than the client could ACK; this gives operators
        // a pre-fail signal. Stable `target: "mercury.backpressure"` so
        // `groupBy peer` immediately shows which clients are struggling.
        let unacked = self.tx_window.len();
        let warn_threshold = consts::TX_WINDOW_SIZE / 2;
        if unacked >= warn_threshold {
            let pct = (unacked * 100) / consts::TX_WINDOW_SIZE;
            tracing::warn!(
                target: "mercury.backpressure",
                peer = %self.remote_addr,
                unacked,
                window_size = consts::TX_WINDOW_SIZE,
                fill_pct = pct,
                next_seq = self.next_tx_seq,
                "send window deep — client may be stalled or RTT spiking"
            );
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
            first_sent: now,
            last_sent: now,
            retransmit_count: 0,
            // send_packet entries have no raw bytes — caller hasn't
            // encrypted yet (the seq was just stamped). Such entries
            // are silently skipped during the retransmit scan.
            raw_bytes: Bytes::new(),
            send_site: None,
            send_kind: None,
            fragment_index: None,
            fragment_count: None,
            message_count: None,
            first_message: None,
            retransmit_cap: None,
        });
        self.last_sent = now;

        Ok(())
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
    /// `process_ack` (e.g., raw datagram counted at the socket layer).
    pub fn touch_received(&mut self) {
        self.last_received = self.clock.now();
    }

    /// Read-only access to the channel's clock. The harness uses this
    /// to advance a `TestClock` from outside without a back-channel.
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}
