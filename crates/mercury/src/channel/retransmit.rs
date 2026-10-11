//! Retransmission and reap timing for a [`Channel`]: the per-tick RTO scan
//! that selects expired TX-window entries for resend
//! ([`Channel::check_timeouts`]), and the dead-peer check the tick loop reaps
//! on ([`Channel::is_timed_out`]).
//!
//! Per-entry retransmit caps live in [`super::retransmit_cap`]; the adaptive
//! RTO itself in [`super::rto`].

use bytes::Bytes;

use crate::consts;

use super::channel_core::Channel;

impl Channel {
    /// Check all packets in the TX window for retransmission timeouts.
    ///
    /// Uses the per-channel adaptive RTO — `self.rto().current()` — rather
    /// than a fixed `consts::ACK_TIMEOUT_MS`. Each expired entry
    /// (whose `last_sent` is older than the current RTO) is selected for
    /// retransmission: we increment `retransmit_count`, refresh
    /// `last_sent`, and emit the entry's cached encrypted bytes for the
    /// caller to put on the wire. After the scan, if anything was
    /// retransmitted, `on_retransmit()` doubles the RTO once
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
        let now = self.clock().now();
        let timeout = self.rto().current();
        let mut retransmits = Vec::new();
        let mut budget = consts::RETRANSMIT_BUDGET_PER_TICK;
        // Track whether *any* expired entry was processed (regardless of
        // whether it had bytes available to resend), so the channel-level
        // side effects (RTO backoff + last_sent bump) fire on the
        // semantic event "the channel timed out", not the narrower event
        // "we had bytes to put on the wire". A channel full of bytes-empty
        // entries (`send_packet` legacy path) would otherwise never back off.
        let mut any_expired = false;
        // Capped entries that expired again at their cap: dropped below
        // instead of resent (`super::retransmit_cap`).
        let mut at_cap: Vec<u32> = Vec::new();
        // Channel-wide cause fields for the retransmit row, read once per
        // scan: the RTT the RTO was derived from, and how long the peer has
        // been silent. A long `peer_last_rx_ms` says the client went away; a
        // short one with retransmits says it is talking but not acking.
        let srtt_ms = self
            .rto()
            .srtt()
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        let peer_last_rx_ms = u64::try_from(
            now.saturating_duration_since(self.last_received)
                .as_millis(),
        )
        .unwrap_or(u64::MAX);
        // Who the peer is (TG-MER-03), copied out before the loop borrows
        // the window mutably. Empty until the service syncs it.
        let identity = self.log_identity;

        for entry in self.tx_window.iter_mut() {
            if budget == 0 {
                break;
            }
            if now.duration_since(entry.last_sent) < timeout {
                continue;
            }
            if entry
                .retransmit_cap
                .is_some_and(|cap| entry.retransmit_count >= cap)
            {
                at_cap.push(entry.packet.sequence);
                continue;
            }
            // Expired — count against budget and refresh.
            any_expired = true;
            budget -= 1;
            entry.retransmit_count += 1;
            entry.last_sent = now;
            self.retransmits_total += 1;
            // Stable retransmit event for SigNoz: lets operators ask
            // "is the network actually lossy?" by counting events with
            // `target = "mercury.retransmit"` per peer / per minute.
            // info level — these are rare in a healthy channel; a
            // sustained rate indicates real loss or a stalled peer. The
            // per-scan budget bounds it to `RETRANSMIT_BUDGET_PER_TICK` rows
            // per channel per tick. This is the only per-retransmit row:
            // the tick loop that sends the bytes logs only a send failure.
            //
            // Cause fields (TG-MER-02): `age_ms` is how long the packet has
            // been unacked (`first_sent`, which a retransmit does not move);
            // `send_kind` / `fragment_index` say which send path produced
            // it; `wire_fingerprint` joins it to the client's socket log.
            // The account and player pairs (TG-MER-03) say whose session it
            // is, so a retransmit burst filters by player, not by address.
            // `msg_id` is present only where the sender recorded the head
            // (a bundle fragment past the first): the retained bytes are
            // encrypted and the transport holds no session key.
            let age_ms = u64::try_from(now.saturating_duration_since(entry.first_sent).as_millis())
                .unwrap_or(u64::MAX);
            tracing::info!(
                target: "mercury.retransmit",
                event = "retransmit",
                peer = %self.remote_addr,
                account_id = identity.account_id,
                account_name = identity.account_name,
                player_id = identity.player_id,
                player_name = identity.player_name,
                seq = entry.packet.sequence,
                retransmit_count = entry.retransmit_count,
                rto_ms = timeout.as_millis() as u64,
                age_ms,
                srtt_ms,
                peer_last_rx_ms,
                send_kind = entry.send_kind,
                msg_id = entry.first_message.map(|m| m.msg_id), // nt:id-only transport has no method tables, so no msg_name here
                fragment_index = entry.fragment_index,
                wire_fingerprint = %crate::instrumentation::wire_fingerprint(&entry.raw_bytes),
                "Mercury: reliable packet retransmit"
            );
            if !entry.raw_bytes.is_empty() {
                retransmits.push(entry.raw_bytes.clone());
            }
        }

        if any_expired {
            self.last_sent = now;
            self.rto_mut().on_retransmit();
        }
        if !at_cap.is_empty() {
            self.abandon_capped(&at_cap);
        }
        retransmits
    }

    /// Returns `true` if the channel has exceeded the maximum retry count
    /// or the peer has been silent past `MERCURY_PEER_DEAD_MS`.
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
            .clock()
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
        let peer_idle_timeout = peer_idle_ms > consts::MERCURY_PEER_DEAD_MS;

        max_retries_exceeded || peer_idle_timeout
    }
}
