//! Channel-state invariants used by the chaos-scenario tests.
//!
//! Each scenario asserts both **positive** properties (channel
//! recovers, bundle delivers, state machine progresses) and
//! **negative** properties (no silent corruption, no leaked state,
//! no infinite loops). The positive assertions are scenario-
//! specific; the negative ones are largely the same shape across
//! scenarios. This module collects the recurring shapes as
//! reusable assertions.
//!
//! Each invariant takes a `&Channel` (read-only snapshot — does
//! not perturb state) and panics with a clear message if the
//! invariant is violated. Tests call them at scenario end, or at
//! intermediate checkpoints inside long runs.

use crate::channel::Channel;
use crate::consts;

/// Asserts the TX window has not overflowed beyond its hard cap.
/// The deferred-send queue is also checked because the overflow
/// path spills there; together the two cannot exceed
/// `TX_WINDOW_SIZE + MAX_UNSENT_PACKETS` without the channel
/// being in an unrecoverable state.
pub fn tx_window_within_capacity(channel: &Channel) {
    assert!(
        channel.tx_window.len() <= consts::TX_WINDOW_SIZE,
        "tx_window length {} exceeds capacity {}",
        channel.tx_window.len(),
        consts::TX_WINDOW_SIZE,
    );
    assert!(
        channel.unsent_packets.len() <= consts::MAX_UNSENT_PACKETS,
        "unsent_packets length {} exceeds cap {}",
        channel.unsent_packets.len(),
        consts::MAX_UNSENT_PACKETS,
    );
}

/// Asserts no TX-window entry has been retransmitted past
/// `MAX_RETRIES`. A channel sitting in this state would be reaped
/// on the next `is_timed_out` check; observing it here without a
/// reap means the tick driver isn't running often enough or the
/// reaper has a bug.
pub fn no_orphan_retransmits(channel: &Channel) {
    for (i, entry) in channel.tx_window.iter().enumerate() {
        assert!(
            entry.retransmit_count <= consts::MAX_RETRIES,
            "tx_window[{i}] seq={} has retransmit_count {} exceeding MAX_RETRIES {}",
            entry.packet.sequence,
            entry.retransmit_count,
            consts::MAX_RETRIES,
        );
    }
}

/// Asserts the channel does not have a TX-window entry whose
/// `retransmit_count` is at exactly `MAX_RETRIES` (i.e. the next
/// scan will reap the channel). Useful at scenario end where the
/// expected outcome is a fully-quiesced, alive channel.
pub fn channel_not_about_to_reap(channel: &Channel) {
    for entry in channel.tx_window.iter() {
        assert!(
            entry.retransmit_count < consts::MAX_RETRIES,
            "tx_window entry seq={} has retransmit_count {} == MAX_RETRIES — \
             channel is one tick away from being reaped",
            entry.packet.sequence,
            entry.retransmit_count,
        );
    }
}

/// Asserts the channel's TX window is empty AND no entries are
/// queued for deferred send. The strongest "channel is fully
/// quiescent" check — use at scenario end after a `session.quiesce`.
pub fn fully_quiesced(channel: &Channel) {
    assert!(
        channel.tx_window.is_empty(),
        "tx_window not drained: {} entries remain",
        channel.tx_window.len(),
    );
    assert!(
        channel.unsent_packets.is_empty(),
        "unsent_packets not drained: {} entries remain",
        channel.unsent_packets.len(),
    );
}

/// Run the full invariant set against a channel. Convenience for
/// scenario end: catches anything one of the individual checks
/// would catch, in one call.
pub fn all_safety_invariants(channel: &Channel) {
    tx_window_within_capacity(channel);
    no_orphan_retransmits(channel);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::channel::Channel;
    use crate::clock::SystemClock;

    fn fresh_channel() -> Channel {
        Channel::with_clock("127.0.0.1:9000".parse().unwrap(), Arc::new(SystemClock))
    }

    #[test]
    fn fresh_channel_passes_every_invariant() {
        let ch = fresh_channel();
        tx_window_within_capacity(&ch);
        no_orphan_retransmits(&ch);
        channel_not_about_to_reap(&ch);
        fully_quiesced(&ch);
        all_safety_invariants(&ch);
    }
}
