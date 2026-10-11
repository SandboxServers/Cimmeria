//! [`Channel::health`]: one read-only snapshot of a channel's reliability
//! counters and ages, for the rows that end or reap a session.

use super::channel_core::Channel;
use super::state::ChannelHealth;

impl Channel {
    /// Snapshot this channel's health ([`ChannelHealth`]). Read-only and
    /// cheap: one pass over the front of each send queue, no allocation.
    ///
    /// The oldest unacked entry is the earliest `first_sent` across the TX
    /// window and the deferred-send queue. Both are kept sorted by
    /// sequence, so it is the front of one of the two; a retransmit does
    /// not move `first_sent`, so a packet resent every second still ages.
    pub fn health(&self) -> ChannelHealth {
        let now = self.clock().now();
        let oldest = [self.tx_window.front(), self.unsent_packets.front()]
            .into_iter()
            .flatten()
            .map(|e| e.first_sent)
            .min();
        let outstanding = self.tx_window.len() + self.unsent_packets.len();
        ChannelHealth {
            tx_outstanding: u32::try_from(outstanding).unwrap_or(u32::MAX),
            oldest_unacked_age_ms: oldest.map(|at| millis(now.saturating_duration_since(at))),
            retransmits_total: self.retransmits_total,
            tx_holes: self.tx_holes,
            tx_hole_stalls: self.tx_hole_stalls,
            rx_stalls: self.rx_stalls,
            srtt_ms: self.rto().srtt().map(millis),
            last_rx_age_ms: millis(now.saturating_duration_since(self.last_received)),
        }
    }
}

fn millis(d: std::time::Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
