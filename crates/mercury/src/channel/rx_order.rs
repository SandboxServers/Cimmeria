//! In-order delivery of inbound **reliable** packets — the receive half of
//! the Mercury channel.
//!
//! # What the SGW client does (the model this file copies)
//!
//! Every inbound packet on a client channel goes through
//! `Nub::processFilteredPacket` (`ghidra://SGW.exe@0x01580ad4`). Once the
//! footers are stripped it splits on `FLAG_RELIABLE` (`0x10`):
//!
//! - **Reliable** packets go to `UnAckedHandler::queueAckForPacket`
//!   (`ghidra://SGW.exe@0x0158cba0`). The ACK is queued first, for every
//!   in-range sequence. Then:
//!   - `seq == inSeqAt` (`ChannelInternal+0x50`): `inSeqAt` advances, and
//!     every packet already buffered at the following sequences is chained
//!     on behind it. The caller processes that chain in sequence order.
//!   - `seq` ahead of `inSeqAt` but within the window (`+0x30`): the packet
//!     is buffered in the slot table at `+0x40` (mask `+0x44`), log string
//!     `"Buffering packet #%d above #%d"` at `0x01b1a040`. Nothing is
//!     delivered.
//!   - A slot already holding that sequence: dropped,
//!     `"Discarding already-buffered packet #%d"` at `0x01b19fe8`.
//!   - Behind `inSeqAt`: dropped, `"Discarding already-seen packet #%d below
//!     inSeqAt #%d"` at `0x01b19f30`.
//!   - Further ahead than the window: dropped with a warning,
//!     `"Sequence number #%d is way out of window #%d!"` at `0x01b19f90`.
//! - **Unreliable** packets skip the window. `FUN_0158bb50` checks them
//!   against a separate dedup structure at `+0x128` and the survivors are
//!   processed immediately, in arrival order.
//!
//! `inSeqAt` starts at `0x10000000` (`SEQ_NULL`) in the `ChannelInternal`
//! constructor (`ghidra://SGW.exe@0x0158c7b0`), and `queueAckForPacket`
//! adopts the first reliable sequence it sees. The window is 512: the
//! `Channel` constructor (`ghidra://SGW.exe@0x01576bf0`) writes `0x200` to
//! `Channel+0x2c`, which `ChannelInternal` copies to `+0x30`.
//!
//! # What this file does
//!
//! [`Channel::receive_parsed`] is the same gate on a [`ParsedPacket`]:
//! reliable packets are released in sequence order, a gap holds everything
//! behind it until the retransmit fills it, duplicates are dropped, and
//! unreliable packets go straight through. Released packets then pass
//! through the per-channel fragment assembler.
//!
//! Two deliberate differences from the client:
//!
//! - A packet further ahead than the window is **not** acked, so the sender
//!   keeps retransmitting it until the window reaches it. The client acks it
//!   and drops it, which loses it for good; nothing on the wire depends on
//!   that.
//! - Unreliable packets are not deduplicated. The client's `+0x128`
//!   structure only suppresses network-duplicated unreliable datagrams,
//!   which are all position frames or keepalives that a repeat cannot harm.

use bytes::Bytes;
use cimmeria_common::Result;

use crate::consts;
use crate::packet::{Packet, PacketFlags, ParsedPacket, SEQUENCE_MASK};

use super::channel_core::Channel;
use super::state::RxEntry;

/// Half of the 28-bit sequence space. A modular distance at or beyond this
/// from `expected_rx_seq` means "behind", mirroring the client's
/// `uVar6 < 0x8000001` ahead test in `queueAckForPacket`.
const HALF_SEQ_SPACE: u32 = 0x0800_0000;

/// What the receive gate did with one inbound packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RxOutcome {
    /// Unreliable, or carrying no sequence number: delivered on arrival.
    Unordered,
    /// The next expected reliable packet. It was delivered, together with
    /// any buffered packets it unblocked.
    InOrder,
    /// Reliable and ahead of a gap: held until the gap fills.
    Buffered,
    /// Reliable and already seen, either delivered or buffered: dropped.
    Duplicate,
    /// Reliable and further ahead than the receive window: dropped and not
    /// acked, so the sender retransmits it.
    OutOfWindow,
}

/// Result of [`Channel::receive_parsed`].
#[derive(Debug, Clone)]
pub struct RxDelivery {
    /// Complete bundle bodies ready for dispatch, in delivery order. Can be
    /// empty (buffered, duplicate, or a fragment of an incomplete bundle),
    /// and can hold several bundles when one packet fills a gap.
    pub bundles: Vec<Bytes>,
    /// The sequence number the caller owes the peer an ACK for, if any.
    /// `Some` for every reliable packet except [`RxOutcome::OutOfWindow`].
    pub ack: Option<u32>,
    /// What the gate did with the packet.
    pub outcome: RxOutcome,
}

impl Channel {
    /// Pin the next reliable sequence number this channel expects from the
    /// peer, instead of adopting the first one that arrives.
    ///
    /// Use it when the peer's starting sequence is known. The SGW client's
    /// first reliable packet on a fresh channel is seq 0
    /// (`authenticate` + `enableEntities`, flags `0x58`, in the
    /// `castle_cellblock_head` capture). Pinning it means a lost first
    /// packet is recovered by retransmit rather than skipped over when a
    /// later one is adopted as the start.
    pub fn anchor_rx_seq(&mut self, next_expected: u32) {
        self.expected_rx_seq = next_expected & SEQUENCE_MASK;
        self.rx_anchored = true;
        self.rx_window.clear();
    }

    /// Whether the receive window has a starting sequence yet, either from
    /// [`Self::anchor_rx_seq`] or from the first reliable packet received.
    pub fn rx_anchored(&self) -> bool {
        self.rx_anchored
    }

    /// Feed one parsed inbound packet through the in-order delivery gate
    /// and the fragment assembler. The live receive path for both the
    /// server's client sessions and the loopback/wireclient harness.
    ///
    /// ACK footers on the packet are not processed here; the caller hands
    /// them to [`Self::process_acks`] for every packet, whatever the gate
    /// decides, as the client does.
    pub fn receive_parsed(&mut self, pkt: ParsedPacket) -> Result<RxDelivery> {
        self.last_received = self.clock().now();

        let seq = match (pkt.is_reliable(), pkt.seq_id) {
            (true, Some(seq)) => seq & SEQUENCE_MASK,
            _ => {
                let bundles = self.reassemble_parsed(&pkt)?.into_iter().collect();
                return Ok(RxDelivery {
                    bundles,
                    ack: None,
                    outcome: RxOutcome::Unordered,
                });
            }
        };

        let (outcome, released) = self.admit(seq, pkt);
        let mut bundles = Vec::new();
        for pkt in released {
            if let Some(body) = self.reassemble_parsed(&pkt)? {
                bundles.push(body);
            }
        }
        match outcome {
            RxOutcome::Buffered => tracing::debug!(
                target: "mercury.rx_order",
                event = "buffered",
                peer = %self.remote_addr,
                seq,
                expected = self.expected_rx_seq,
                "reliable packet ahead of a gap -- holding it until the gap fills"
            ),
            RxOutcome::Duplicate => tracing::debug!(
                target: "mercury.rx_order",
                event = "duplicate",
                peer = %self.remote_addr,
                seq,
                expected = self.expected_rx_seq,
                "reliable packet already seen -- dropped, still acked"
            ),
            RxOutcome::OutOfWindow => tracing::warn!(
                target: "mercury.rx_order",
                event = "out_of_window",
                reason = "beyond_rx_window",
                peer = %self.remote_addr,
                seq,
                expected = self.expected_rx_seq,
                window = consts::RX_WINDOW_SIZE,
                "reliable packet beyond the receive window -- dropped unacked, sender will retransmit"
            ),
            RxOutcome::InOrder | RxOutcome::Unordered => {}
        }
        Ok(RxDelivery {
            bundles,
            ack: (outcome != RxOutcome::OutOfWindow).then_some(seq),
            outcome,
        })
    }

    /// Process an inbound [`Packet`], treating it as reliable and
    /// inserting it into the RX window.
    ///
    /// Returns `Ok(Some(packets))` with any newly in-order packets that
    /// can be delivered upstream, or `Ok(None)` if we are still waiting
    /// for earlier sequences. Packet-level twin of [`Self::receive_parsed`]
    /// sharing the same window; kept for callers that hold a legacy
    /// [`Packet`]. It skips fragment reassembly.
    pub fn receive_packet(&mut self, packet: Packet) -> Result<Option<Vec<Packet>>> {
        let seq = packet.sequence;
        self.last_received = self.clock().now();

        // Instrument: inbound UDP packet observed. Every received packet is
        // recorded, duplicates included, because those are the anomalies
        // the analytical store needs to surface.
        crate::instrumentation::record_udp_packet(
            crate::instrumentation::Direction::In,
            seq,
            packet.flags.0,
            packet.body.len(),
            self.remote_addr,
        );

        let parsed = ParsedPacket {
            flags: packet.flags.0,
            body: packet.body,
            seq_id: Some(seq),
            first_req_offset: None,
            frag_begin: None,
            frag_end: None,
            acks: Vec::new(),
        };
        let (_, released) = self.admit(seq & SEQUENCE_MASK, parsed);
        if released.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            released
                .into_iter()
                .map(|p| Packet {
                    flags: PacketFlags::from_byte(p.flags),
                    sequence: p.seq_id.unwrap_or_default(),
                    body: p.body,
                })
                .collect(),
        ))
    }

    /// The window itself: place `pkt` (reliable, sequence `seq`) and return
    /// what happened plus every packet now deliverable in order.
    fn admit(&mut self, seq: u32, pkt: ParsedPacket) -> (RxOutcome, Vec<ParsedPacket>) {
        if !self.rx_anchored {
            // queueAckForPacket: `if (inSeqAt == SEQ_NULL) inSeqAt = seq;`
            self.expected_rx_seq = seq;
            self.rx_anchored = true;
        }

        let offset = seq.wrapping_sub(self.expected_rx_seq) & SEQUENCE_MASK;
        if offset >= HALF_SEQ_SPACE {
            return (RxOutcome::Duplicate, Vec::new());
        }
        let offset = offset as usize;
        if offset >= consts::RX_WINDOW_SIZE {
            return (RxOutcome::OutOfWindow, Vec::new());
        }

        while self.rx_window.len() <= offset {
            self.rx_window.push_back(None);
        }
        if self.rx_window[offset].is_some() {
            return (RxOutcome::Duplicate, Vec::new());
        }
        self.rx_window[offset] = Some(RxEntry {
            packet: pkt,
            received_at: self.last_received,
        });

        let mut released = Vec::new();
        while let Some(Some(_)) = self.rx_window.front() {
            if let Some(Some(entry)) = self.rx_window.pop_front() {
                released.push(entry.packet);
            }
            self.expected_rx_seq = self.expected_rx_seq.wrapping_add(1) & SEQUENCE_MASK;
        }
        let outcome = if released.is_empty() {
            RxOutcome::Buffered
        } else {
            RxOutcome::InOrder
        };
        (outcome, released)
    }
}
