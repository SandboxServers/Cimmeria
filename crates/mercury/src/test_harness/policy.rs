//! Per-direction packet manipulation policy for the loopback harness.
//!
//! Policies are applied at the **sender side** of the loopback pump: an
//! "A→B drop" causes A to never put the bytes on the wire, which produces
//! the same observable outcome on B (and the same RTO/retransmit
//! consequences on A) as a wire-side drop. This is the only honest
//! arrangement on real loopback sockets — there's no OS hook between A's
//! `send_to` and B's `recv_from`.

use std::time::Duration;

use bytes::Bytes;

/// Per-direction packet manipulation policy.
///
/// Default is lossless, in-order, zero-latency loopback.
///
/// The fields split into two groups:
///
/// - **Configuration** (set by tests): [`Self::drop_next`],
///   [`Self::drop_at_send_count`], [`Self::latency`],
///   [`Self::reorder_pairs`], [`Self::duplicate_every`].
/// - **Internal pump state** (managed by the send path; tests can read
///   for diagnostics but should not mutate directly):
///   [`Self::send_count`], [`Self::duplicate_count`],
///   [`Self::reorder_held`].
///
/// Tests that need to reset counters between phases call
/// [`Self::reset_counters`].
#[derive(Debug, Default)]
pub struct NetworkPolicy {
    /// Drop the next N packets in `direction` before resuming normal
    /// delivery. Decremented as drops fire.
    pub drop_next: NetworkDirection<u32>,

    /// Drop the send whose [`Self::send_count`] reaches this value.
    /// 1-indexed: `Some(2)` drops the second send. Cleared after the
    /// matching send to avoid repeated drops on retransmits.
    pub drop_at_send_count: NetworkDirection<Option<u32>>,

    /// Insert `latency` before delivering each packet in `direction`.
    /// Implemented via `tokio::time::sleep` on the sender side.
    pub latency: NetworkDirection<Duration>,

    /// Deliver packets in `direction` swapped in arrival pairs:
    /// `(1, 2, 3, 4)` → `(2, 1, 4, 3)`. The implementation holds the
    /// first packet of each pair in [`Self::reorder_held`] until the
    /// second arrives, then ships them in `(second, first)` order.
    pub reorder_pairs: NetworkDirection<bool>,

    /// When set to `Some(n)` with `n >= 1`, duplicate every Nth send
    /// in `direction` (so the receiver sees it twice). For example,
    /// `Some(2)` duplicates the 2nd, 4th, 6th... send. `None` or
    /// `Some(0)` = no duplication.
    pub duplicate_every: NetworkDirection<Option<u32>>,

    /// Internal: per-direction send counter (1-indexed). Incremented
    /// **before** each send-policy decision so the first send sees
    /// `send_count == 1`.
    pub send_count: NetworkDirection<u32>,

    /// Internal: per-direction duplicate-cycle counter. Increments on
    /// every send; when it hits the configured `duplicate_every`
    /// value, a copy is emitted and the counter resets to 0.
    pub duplicate_count: NetworkDirection<u32>,

    /// Internal: per-direction reorder buffer. Holds the first packet
    /// of each pair while waiting for the second. The send path uses
    /// `Option::take` to drain it.
    pub reorder_held: NetworkDirection<Option<Bytes>>,
}

impl NetworkPolicy {
    /// Reset all per-direction internal counters. Useful between test
    /// phases where the test wants a clean slate (e.g., "ignore
    /// everything before this point — now drop the 2nd send").
    pub fn reset_counters(&mut self) {
        self.send_count = NetworkDirection::default();
        self.duplicate_count = NetworkDirection::default();
        self.reorder_held = NetworkDirection::default();
    }
}

/// Per-direction value pair. `a_to_b` is the direction from peer `a` to
/// peer `b`; `b_to_a` is the reverse.
#[derive(Debug, Default, Clone)]
pub struct NetworkDirection<T> {
    pub a_to_b: T,
    pub b_to_a: T,
}

/// Tag identifying which direction a packet travels — used by the recv
/// pump to look up the right side of each [`NetworkDirection`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    AToB,
    BToA,
}

impl<T: Copy> NetworkDirection<T> {
    /// Read the side of this direction-pair that matches `dir`.
    pub fn get(&self, dir: Direction) -> T {
        match dir {
            Direction::AToB => self.a_to_b,
            Direction::BToA => self.b_to_a,
        }
    }
}

impl<T> NetworkDirection<T> {
    /// Mutable handle to the side of this direction-pair that matches
    /// `dir` — handy for tests that need to decrement a counter (e.g.,
    /// `drop_next`) and for the send pump to update internal state
    /// without cloning.
    pub fn get_mut(&mut self, dir: Direction) -> &mut T {
        match dir {
            Direction::AToB => &mut self.a_to_b,
            Direction::BToA => &mut self.b_to_a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_is_lossless() {
        let p = NetworkPolicy::default();
        assert_eq!(p.drop_next.a_to_b, 0);
        assert_eq!(p.drop_next.b_to_a, 0);
        assert!(p.drop_at_send_count.a_to_b.is_none());
        assert_eq!(p.latency.a_to_b, Duration::ZERO);
        assert!(!p.reorder_pairs.a_to_b);
        assert!(p.duplicate_every.a_to_b.is_none());
        assert_eq!(p.send_count.a_to_b, 0);
        assert_eq!(p.duplicate_count.a_to_b, 0);
        assert!(p.reorder_held.a_to_b.is_none());
    }

    #[test]
    fn direction_get_reads_correct_side() {
        let dir = NetworkDirection {
            a_to_b: 7u32,
            b_to_a: 11u32,
        };
        assert_eq!(dir.get(Direction::AToB), 7);
        assert_eq!(dir.get(Direction::BToA), 11);
    }

    #[test]
    fn reset_counters_clears_internal_state_only() {
        let mut p = NetworkPolicy::default();
        p.drop_next.a_to_b = 5;
        p.duplicate_every.a_to_b = Some(3);
        p.send_count.a_to_b = 42;
        p.duplicate_count.b_to_a = 7;

        p.reset_counters();

        // Config preserved:
        assert_eq!(p.drop_next.a_to_b, 5);
        assert_eq!(p.duplicate_every.a_to_b, Some(3));
        // Internal state reset:
        assert_eq!(p.send_count.a_to_b, 0);
        assert_eq!(p.duplicate_count.b_to_a, 0);
    }
}
