//! Inbound ACK processing — how the transmit side learns a reliable packet
//! arrived — and the transmit-hole watchdog.
//!
//! # One ACK names one packet
//!
//! The SGW client queues an ACK for every reliable packet that carries a
//! valid sequence number, *before* it decides whether to deliver, buffer or
//! drop the packet: `UnAckedHandler::queueAckForPacket`
//! (`ghidra://SGW.exe@0x0158cba0`) inserts the sequence into its ack set
//! (`FUN_0157ac40(this + 0x9c, seq)`) ahead of the `inSeqAt` comparison. So
//! a packet buffered behind a gap is acked while the gap itself is not.
//!
//! The Lomiada capture (`debug/lomiada-broke-in-hallway02/`) shows it on
//! the wire. Server packet #1148 never arrived. About 100 ms later the
//! client acked #1149, then #1150..#1155 in one footer, and so on up to
//! #1358, while its own log read `Buffering packet #1149 above #1148`
//! through `#1358 above #1148`.
//!
//! An ACK is therefore **not** cumulative on a client channel. Reading one
//! as "everything up to N arrived" retires the packet the client is waiting
//! for: the ACK for #1149 would have retired #1148, and #1148 would never
//! have been resent. The client delivers nothing reliable past a gap (see
//! [`super::rx_order`]), so that one retired packet wedges every reliable
//! message after it — entity creates, leaves and method calls — for the
//! rest of the session, while unreliable movement keeps flowing. A player
//! whose channel wedges keeps seeing the entities it already has move
//! around, and never sees anything new arrive.
//!
//! [`Channel::process_ack`] retires exactly the packet an ACK names. A lost
//! packet stays outstanding until [`Channel::check_timeouts`] resends it.
//!
//! # The transmit-hole watchdog
//!
//! Once the peer has acked a packet sent after one that is still
//! outstanding, the peer is holding everything behind that one packet.
//! [`Channel::process_ack_footer`] tracks that hole, and
//! [`Channel::check_tx_hole`] warns (`mercury.tx_hole`, `event=tx_hole_stall`)
//! when it stays open for [`consts::TX_HOLE_WARN_MS`]. It is the server's
//! view of the client's receive stall — the one signal the server has that
//! a client has stopped receiving new entities.

use std::time::Duration;

use crate::consts;
use crate::packet::SEQUENCE_MASK;

use super::channel_core::{seq_mod_leq, Channel};
use super::state::{TxEntry, TxHole};

/// True if `a` is strictly before `b` in the 28-bit modular sequence space.
fn seq_mod_lt(a: u32, b: u32) -> bool {
    (a & SEQUENCE_MASK) != (b & SEQUENCE_MASK) && seq_mod_leq(a, b)
}

/// Remove the entry for `seq` from `deque`, if there is one.
pub(super) fn take_entry(
    deque: &mut std::collections::VecDeque<TxEntry>,
    seq: u32,
) -> Option<TxEntry> {
    let idx = deque.iter().position(|e| e.packet.sequence == seq)?;
    deque.remove(idx)
}

/// A transmit hole the watchdog reported. See [`Channel::check_tx_hole`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxHoleStall {
    /// Sequence of the reliable packet the peer is missing.
    pub seq: u32,
    /// How long the hole has been open.
    pub stalled_for: Duration,
    /// Highest sequence the peer has acked.
    pub highest_acked: u32,
    /// How many times the missing packet has been retransmitted.
    pub retransmit_count: u32,
    /// Reliable packets still outstanding (TX window plus deferred queue).
    pub outstanding: usize,
    /// Size of the encrypted UDP payload initially sent for the missing seq.
    pub wire_len: usize,
    /// FNV-1a key of that exact payload, shared with client socket logs.
    pub wire_fingerprint: String,
    /// Service dispatch site that sent the packet, when known.
    pub send_site: Option<String>,
    /// Send path and bundle position, if the service supplied them.
    pub send_kind: Option<&'static str>,
    pub fragment_index: Option<usize>,
    pub fragment_count: Option<usize>,
    pub message_count: Option<usize>,
    /// True on the first warning for this hole. False on the throttled
    /// repeats.
    pub first_warning: bool,
}

impl Channel {
    /// Retire the outstanding reliable packet that one ACK names.
    ///
    /// Looks the sequence up in the TX window, then in the deferred queue
    /// (the bytes of a queued entry went on the wire when it was queued, so
    /// the peer can ack it there). A clean round — the packet was never
    /// retransmitted — feeds an RTT sample to the RTO smoother (Karn's
    /// algorithm: the ACK of a retransmitted packet is ambiguous about which
    /// copy arrived). Freed TX-window slots are refilled oldest-first from
    /// the deferred queue so the retransmit scan can see those entries.
    /// Promoted entries keep their `last_sent`, so one that sat in the
    /// queue past its RTO is resent on the next [`Self::check_timeouts`].
    ///
    /// Only the named packet is retired — see the module doc for why an ACK
    /// is not cumulative here. An ACK that names nothing outstanding (a
    /// duplicate, or the ACK of a retransmitted copy that already landed)
    /// changes nothing but `last_received`.
    ///
    /// This does not update the transmit-hole tracking; a caller holding a
    /// whole ACK footer uses [`Self::process_ack_footer`], which does.
    ///
    /// Returns whether a packet was retired.
    pub fn process_ack(&mut self, ack_seq: u32) -> bool {
        // An ACK is peer-originated data: receive-side activity.
        let now = self.clock().now();
        self.last_received = now;

        let seq = ack_seq & SEQUENCE_MASK;
        let retired = if let Some(entry) = take_entry(&mut self.tx_window, seq) {
            if entry.retransmit_count == 0 {
                self.rto_mut()
                    .on_sample(now.duration_since(entry.last_sent));
            }
            true
        } else {
            take_entry(&mut self.unsent_packets, seq).is_some()
        };
        if !retired {
            return false;
        }

        if self
            .highest_acked
            .is_none_or(|highest| seq_mod_lt(highest, seq))
        {
            self.highest_acked = Some(seq);
        }

        while self.tx_window.len() < consts::TX_WINDOW_SIZE {
            match self.unsent_packets.pop_front() {
                Some(entry) => self.tx_window.push_back(entry),
                None => break,
            }
        }
        true
    }

    /// Apply every ACK in one packet's footer, then update the
    /// transmit-hole tracking.
    ///
    /// The hole is judged once per footer, not per ACK, so a footer that
    /// lists its ACKs newest-first (`[1155, 1154, …, 1150]`) does not open
    /// and close a hole on its way through.
    ///
    /// Returns how many packets were retired.
    pub fn process_ack_footer(&mut self, acks: &[u32]) -> usize {
        let retired = acks.iter().filter(|&&seq| self.process_ack(seq)).count();
        self.refresh_tx_hole();
        retired
    }

    /// The oldest outstanding reliable sequence, across the TX window and
    /// the deferred queue. Each is kept sorted, so their fronts suffice.
    fn oldest_outstanding(&self) -> Option<u32> {
        let window = self.tx_window.front().map(|e| e.packet.sequence);
        let queued = self.unsent_packets.front().map(|e| e.packet.sequence);
        match (window, queued) {
            (Some(w), Some(q)) => Some(if seq_mod_leq(w, q) { w } else { q }),
            (w, q) => w.or(q),
        }
    }

    /// Open, move or close the transmit hole after an ACK footer.
    pub(super) fn refresh_tx_hole(&mut self) {
        let now = self.clock().now();
        let hole_seq = match (self.oldest_outstanding(), self.highest_acked) {
            (Some(oldest), Some(highest)) if seq_mod_lt(oldest, highest) => Some(oldest),
            _ => None,
        };
        if self.tx_hole.map(|h| h.seq) == hole_seq {
            return;
        }
        // The previous hole, if any, is closed: its packet was acked, or it
        // was the first of several missing packets and the next one now
        // leads. Say so only for a hole that was reported as a stall.
        if let Some(closed) = self.tx_hole.take() {
            if closed.warned_at.is_some() {
                tracing::info!(
                    target: "mercury.tx_hole",
                    event = "tx_hole_closed",
                    peer = %self.remote_addr,
                    seq = closed.seq,
                    open_ms = now.saturating_duration_since(closed.since).as_millis() as u64,
                    "reliable packet the peer was missing has been acked -- its held messages are released"
                );
            }
        }
        if let (Some(seq), Some(highest)) = (hole_seq, self.highest_acked) {
            self.tx_holes += 1;
            tracing::debug!(
                target: "mercury.tx_hole",
                event = "tx_hole_open",
                peer = %self.remote_addr,
                seq,
                highest_acked = highest,
                "peer acked a later reliable packet first -- this one was lost or reordered"
            );
            self.tx_hole = Some(TxHole {
                seq,
                since: now,
                warned_at: None,
            });
        }
    }

    /// The transmit hole currently open, if any, and how long it has been
    /// open. For diagnostics.
    pub fn open_tx_hole(&self) -> Option<(u32, Duration)> {
        let hole = self.tx_hole?;
        Some((
            hole.seq,
            self.clock().now().saturating_duration_since(hole.since),
        ))
    }

    /// Transmit-hole watchdog: report a hole that has stayed open too long.
    ///
    /// Call it from the channel's periodic tick. It logs a WARN
    /// (`mercury.tx_hole`, `event=tx_hole_stall`) when a hole first crosses
    /// [`consts::TX_HOLE_WARN_MS`], then at most once per
    /// [`consts::TX_HOLE_REWARN_MS`] while the same hole stays open. `Some`
    /// is returned only on the ticks that warn. [`TxHoleStall::first_warning`]
    /// marks the first warning for a hole, and [`Self::tx_hole_stalls`]
    /// counts those.
    ///
    /// A normal loss closes well inside the threshold: the retransmit scan
    /// resends the missing packet after one RTO. A hole that outlives it
    /// means the resends are not getting through, and the peer is still
    /// holding every reliable message behind the missing one.
    pub fn check_tx_hole(&mut self) -> Option<TxHoleStall> {
        let hole = self.tx_hole?;
        let now = self.clock().now();
        let stalled_for = now.saturating_duration_since(hole.since);
        if stalled_for < Duration::from_millis(consts::TX_HOLE_WARN_MS) {
            return None;
        }
        let first_warning = hole.warned_at.is_none();
        if let Some(last) = hole.warned_at {
            if now.saturating_duration_since(last)
                < Duration::from_millis(consts::TX_HOLE_REWARN_MS)
            {
                return None;
            }
        }
        self.tx_hole = Some(TxHole {
            warned_at: Some(now),
            ..hole
        });
        if first_warning {
            self.tx_hole_stalls += 1;
        }

        let retransmit_count = self
            .tx_window
            .iter()
            .chain(self.unsent_packets.iter())
            .find(|e| e.packet.sequence == hole.seq)
            .map_or(0, |e| e.retransmit_count);
        let entry = self
            .tx_window
            .iter()
            .chain(self.unsent_packets.iter())
            .find(|e| e.packet.sequence == hole.seq);
        let stall = TxHoleStall {
            seq: hole.seq,
            stalled_for,
            highest_acked: self.highest_acked.unwrap_or(hole.seq),
            retransmit_count,
            outstanding: self.tx_window.len() + self.unsent_packets.len(),
            wire_len: entry.map_or(0, |e| e.raw_bytes.len()),
            wire_fingerprint: entry.map_or_else(String::new, |e| {
                crate::instrumentation::wire_fingerprint(&e.raw_bytes)
            }),
            send_site: entry.and_then(|e| e.send_site.clone()),
            send_kind: entry.and_then(|e| e.send_kind),
            fragment_index: entry.and_then(|e| e.fragment_index),
            fragment_count: entry.and_then(|e| e.fragment_count),
            message_count: entry.and_then(|e| e.message_count),
            first_warning,
        };
        tracing::warn!(
            target: "mercury.tx_hole",
            event = "tx_hole_stall",
            reason = "peer_missing_reliable_packet",
            peer = %self.remote_addr,
            seq = stall.seq,
            highest_acked = stall.highest_acked,
            retransmit_count = stall.retransmit_count,
            outstanding = stall.outstanding,
            wire_len = stall.wire_len,
            wire_fingerprint = %stall.wire_fingerprint,
            send_site = stall.send_site.as_deref().unwrap_or(""),
            send_kind = stall.send_kind.unwrap_or(""),
            fragment_index = ?stall.fragment_index,
            fragment_count = ?stall.fragment_count,
            message_count = ?stall.message_count,
            stalled_ms = stall.stalled_for.as_millis() as u64,
            first_warning,
            "peer has acked later reliable packets but not this one -- it is holding every \
             reliable message behind it (entity creates, leaves, method calls) until a resend lands"
        );
        Some(stall)
    }
}
