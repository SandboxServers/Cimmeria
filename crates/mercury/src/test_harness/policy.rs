//! Per-direction packet manipulation policy for the loopback harness.
//!
//! Policies are applied at the **sender side** of the loopback pump: an
//! "A→B drop" causes A to never put the bytes on the wire, which produces
//! the same observable outcome on B (and the same RTO/retransmit
//! consequences on A) as a wire-side drop. This is the only honest
//! arrangement on real loopback sockets — there's no OS hook between A's
//! `send_to` and B's `recv_from`.

use std::time::Duration;

/// Per-direction packet manipulation policy.
///
/// Default is lossless, in-order, zero-latency loopback.
#[derive(Debug, Default, Clone)]
pub struct NetworkPolicy {
    /// Drop the next N packets in `direction` before resuming normal
    /// delivery. Decremented as drops fire.
    pub drop_next: NetworkDirection<u32>,
    /// Insert `latency` before delivering each packet in `direction`.
    /// Implemented via `tokio::time::sleep` on the sender side.
    pub latency: NetworkDirection<Duration>,
    /// Deliver packets in `direction` swapped in arrival pairs:
    /// `(1, 2, 3, 4)` → `(2, 1, 4, 3)`. Off by default.
    pub reorder_pairs: NetworkDirection<bool>,
    /// When set, duplicate every Nth packet in `direction` (so the
    /// receiver sees it twice). Off by default.
    pub duplicate_every: NetworkDirection<Option<u32>>,
}

/// Per-direction value pair. `a_to_b` is the direction from peer `a` to
/// peer `b`; `b_to_a` is the reverse.
#[derive(Debug, Default, Clone, Copy)]
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

impl<T: Default> NetworkDirection<T> {
    /// Mutable handle to the side of this direction-pair that matches
    /// `dir` — handy for tests that need to decrement a counter (e.g.,
    /// `drop_next`).
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
        assert_eq!(p.latency.a_to_b, Duration::ZERO);
        assert!(!p.reorder_pairs.a_to_b);
        assert!(p.duplicate_every.a_to_b.is_none());
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
}
