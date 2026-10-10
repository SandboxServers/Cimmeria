//! The channel's entry into fragment reassembly, and the rate-limited
//! warning for bundles the reassembly caps drop.
//!
//! The caps themselves live in [`crate::unpacker::FragmentAssembler`]
//! ([`consts::MAX_PENDING_FRAGMENTED_BUNDLES`],
//! [`consts::MAX_PENDING_FRAGMENT_BYTES`]). The assembler only counts what
//! it drops; this file turns those counts into one structured WARN per
//! [`consts::FRAGMENT_CAP_WARN_INTERVAL_MS`] per channel
//! (`target: "mercury.fragment_caps"`, `event = "fragment_cap"`), so a
//! peer flooding partial bundles cannot flood the log as well.

use std::time::Duration;

use bytes::Bytes;
use cimmeria_common::Result;

use crate::consts;
use crate::packet::ParsedPacket;
use crate::unpacker::FragmentCapHits;

use super::channel_core::Channel;

impl Channel {
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
    /// implicit, and makes the reassembly caps per channel.
    pub fn reassemble_parsed(&mut self, pkt: &ParsedPacket) -> Result<Option<Bytes>> {
        self.last_received = self.clock().now();
        let result = self.fragment_assembler.process_parsed(pkt);
        let hits = self.fragment_assembler.take_cap_hits();
        if !hits.is_empty() {
            self.note_fragment_cap_hits(hits);
        }
        result
    }

    /// Incomplete fragmented bundles currently held by this channel.
    pub fn pending_fragment_bundles(&self) -> usize {
        self.fragment_assembler.pending_count()
    }

    /// Fragment payload bytes currently held by this channel's incomplete
    /// bundles.
    pub fn pending_fragment_bytes(&self) -> usize {
        self.fragment_assembler.pending_bytes()
    }

    /// Count cap drops and warn at most once per
    /// [`consts::FRAGMENT_CAP_WARN_INTERVAL_MS`]. Drops inside the window
    /// are carried into the next warning.
    fn note_fragment_cap_hits(&mut self, hits: FragmentCapHits) {
        self.fragment_cap_drops += hits.total();
        self.fragment_cap_unreported.merge(hits);

        let now = self.clock().now();
        let interval = Duration::from_millis(consts::FRAGMENT_CAP_WARN_INTERVAL_MS);
        if self
            .fragment_cap_warned_at
            .is_some_and(|last| now.saturating_duration_since(last) < interval)
        {
            return;
        }
        self.fragment_cap_warned_at = Some(now);
        let report = std::mem::take(&mut self.fragment_cap_unreported);
        let (last_reason, last_first_seq, last_received, last_total) = match report.last {
            Some((reason, seq, received, total)) => {
                (reason.as_str(), Some(seq), Some(received), Some(total))
            }
            None => ("none", None, None, None),
        };
        tracing::warn!(
            target: "mercury.fragment_caps",
            event = "fragment_cap",
            reason = last_reason,
            peer = %self.remote_addr,
            dropped = report.total(),
            count_evictions = report.count_evictions,
            byte_evictions = report.byte_evictions,
            oversize_drops = report.oversize_drops,
            last_first_seq,
            last_received,
            last_total,
            pending_bundles = self.fragment_assembler.pending_count(),
            pending_bytes = self.fragment_assembler.pending_bytes(),
            max_bundles = consts::MAX_PENDING_FRAGMENTED_BUNDLES,
            max_bytes = consts::MAX_PENDING_FRAGMENT_BYTES,
            lifetime_drops = self.fragment_cap_drops,
            "incomplete fragmented bundles dropped to stay within the per-channel reassembly caps"
        );
    }
}
