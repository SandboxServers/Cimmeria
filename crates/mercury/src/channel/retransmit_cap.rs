//! Per-entry retransmit caps.
//!
//! An ordinary reliable packet is resent until the peer acks it. A capped
//! one is resent at most `cap` times; when it expires again after that,
//! [`Channel::check_timeouts`] drops it from the window instead of
//! resending it, records an [`AbandonedPacket`], and the caller collects
//! the record with [`Channel::take_abandoned`] to log it.
//!
//! The one user is the base's login handshake (#842): the reply (seq 1)
//! and time-sync (seq 2) go out before the session channel exists, and
//! some real clients never ack them. Without a cap the base would resend
//! both on every RTO for the whole session, because it never reaps a
//! channel on `MAX_RETRIES`.
//!
//! Dropping an entry is not an ACK: it feeds no RTT sample and does not
//! move `highest_acked`. It does free the window slot (promoting from the
//! deferred queue, as an ACK would) and re-judges the transmit hole, so a
//! dropped packet cannot hold a `tx_hole_stall` open forever.

use bytes::Bytes;
use cimmeria_common::Result;

use crate::consts;
use crate::packet::Packet;

use super::ack::take_entry;
use super::channel_core::Channel;
use super::state::AbandonedPacket;

impl Channel {
    /// [`Self::register_sent_packet`], with the entry resent at most
    /// `cap` times. See the module doc.
    pub fn register_sent_packet_capped(
        &mut self,
        packet: Packet,
        raw_bytes: Bytes,
        cap: u32,
    ) -> Result<()> {
        let seq = packet.sequence;
        self.register_sent_packet(packet, raw_bytes)?;
        if let Some(entry) = self
            .tx_window
            .iter_mut()
            .chain(self.unsent_packets.iter_mut())
            .find(|e| e.packet.sequence == seq)
        {
            entry.retransmit_cap = Some(cap);
        }
        Ok(())
    }

    /// Capped entries dropped unacked since the last call, oldest first.
    pub fn take_abandoned(&mut self) -> Vec<AbandonedPacket> {
        std::mem::take(&mut self.abandoned)
    }

    /// Drop the capped entries `seqs` from the TX window. Called by
    /// [`Self::check_timeouts`] for entries that expired at their cap.
    pub(super) fn abandon_capped(&mut self, seqs: &[u32]) {
        for &seq in seqs {
            if let Some(entry) = take_entry(&mut self.tx_window, seq) {
                // INFO, like the retransmit rows on this target (the
                // OTLP filter exports `mercury.retransmit` at INFO). It
                // fires at most once per capped packet; the caller adds
                // the WARN with the session's identity.
                tracing::info!(
                    target: "mercury.retransmit",
                    event = "retransmit_cap_reached",
                    peer = %self.remote_addr,
                    seq,
                    retransmit_count = entry.retransmit_count,
                    "Mercury: capped reliable packet dropped unacked"
                );
                self.abandoned.push(AbandonedPacket {
                    seq,
                    retransmit_count: entry.retransmit_count,
                });
            }
        }
        while self.tx_window.len() < consts::TX_WINDOW_SIZE {
            match self.unsent_packets.pop_front() {
                Some(entry) => self.tx_window.push_back(entry),
                None => break,
            }
        }
        self.refresh_tx_hole();
    }
}
