//! Which client datagrams prove the client is still alive.
//!
//! `last_recv` is the tick-sync loop's inactivity clock: 60 s without a
//! refresh and the session is reaped. A refresh must come from the client,
//! not from someone who can put bytes on the wire with its source address:
//!
//! - **Forged or garbage datagrams** fail the HMAC and never get here.
//! - **Replayed datagrams** pass the HMAC (a sniffer captured them whole).
//!   So passing decryption is not enough: the datagram must also do
//!   something the session has not seen before.
//!
//! A decrypted datagram refreshes the clock when any of these holds:
//!
//! 1. It is reliable and the channel's receive gate accepted it as new
//!    (`InOrder` or `Buffered`). A replay is `Duplicate`, and a far-ahead
//!    forgery cannot be built without the key.
//! 2. Its ACK footer retired at least one of our outstanding reliable
//!    packets. A replayed ACK names a packet already retired.
//! 3. It is unreliable and its sequence number is newer than any unreliable
//!    sequence the session has seen ([`UnreliableHighWater`]). The client
//!    numbers its unreliable packets on their own counter, apart from the
//!    reliable one (`docs/drafts/spec/mercury-wire-format.md`, the
//!    receiver's unreliable dedup at `ChannelInternal+0x128`), so its
//!    position updates and per-tick bundles advance it and a replay does
//!    not. An idle in-world client sends about six of these a second
//!    (`reference_client_idle_send_cadence`), so it never goes stale.
//!
//! A datagram with none of these (a replay, a duplicate retransmit, an
//! unsequenced unreliable packet) is still processed as before. It just
//! does not keep the session alive.

use std::time::Instant;

use cimmeria_mercury::channel::RxOutcome;
use cimmeria_mercury::packet::SEQUENCE_MASK;

use super::super::super::ConnectedClientState;

/// The newest unreliable sequence number the session has seen, kept in its
/// extensions. Absent until the first sequenced unreliable packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct UnreliableHighWater(pub(super) u32);

/// `true` when `seq` is after `prev` in the 28-bit sequence space (less
/// than half the space ahead, so the counter can wrap).
fn seq_after(seq: u32, prev: u32) -> bool {
    let ahead = seq.wrapping_sub(prev) & SEQUENCE_MASK;
    ahead != 0 && ahead < SEQUENCE_MASK.div_ceil(2)
}

/// Whether one decrypted datagram proves the client is alive, updating the
/// unreliable high-water mark as a side effect. `outcome` is the receive
/// gate's verdict (`None` when the gate rejected the packet), and
/// `unreliable_seq` the sequence of an unreliable packet.
fn is_fresh(
    state: &mut ConnectedClientState,
    outcome: Option<RxOutcome>,
    unreliable_seq: Option<u32>,
    acks_retired: usize,
) -> bool {
    let mut fresh = acks_retired > 0;
    match outcome {
        Some(RxOutcome::InOrder | RxOutcome::Buffered) => fresh = true,
        Some(RxOutcome::Unordered) => {
            if let Some(seq) = unreliable_seq.map(|s| s & SEQUENCE_MASK) {
                let advanced = state
                    .extensions
                    .get::<UnreliableHighWater>()
                    .is_none_or(|high| seq_after(seq, high.0));
                if advanced {
                    state.extensions.insert(UnreliableHighWater(seq));
                    fresh = true;
                }
            }
        }
        Some(RxOutcome::Duplicate | RxOutcome::OutOfWindow) | None => {}
    }
    fresh
}

/// Refresh the session's `last_recv` when the datagram is fresh (see the
/// module doc). Called under the `connected` lock, once per decrypted
/// datagram, after the receive gate.
pub(super) fn record(
    state: &mut ConnectedClientState,
    outcome: Option<RxOutcome>,
    unreliable_seq: Option<u32>,
    acks_retired: usize,
) {
    if is_fresh(state, outcome, unreliable_seq, acks_retired) {
        if let Ok(mut at) = state.last_recv.lock() {
            *at = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_comparison_wraps() {
        assert!(seq_after(1, 0));
        assert!(!seq_after(0, 0));
        assert!(!seq_after(0, 1));
        assert!(
            seq_after(0, SEQUENCE_MASK),
            "0 follows the top of the space"
        );
        assert!(!seq_after(SEQUENCE_MASK, 0));
    }
}
