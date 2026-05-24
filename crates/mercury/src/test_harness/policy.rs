//! Per-direction packet manipulation policy for the loopback harness.
//!
//! Policies are applied at the **sender side** of the loopback pump: an
//! "A→B drop" causes A to never put the bytes on the wire, which produces
//! the same observable outcome on B (and the same RTO/retransmit
//! consequences on A) as a wire-side drop. This is the only honest
//! arrangement on real loopback sockets — there's no OS hook between A's
//! `send_to` and B's `recv_from`.
//!
//! # Chaos-testing primitives
//!
//! Beyond the simple deterministic primitives (`drop_next`,
//! `drop_at_send_count`, `reorder_pairs`, `duplicate_every`,
//! `latency`), the policy supports three additional shapes that exist
//! to enable the network-chaos scenario tests:
//!
//! - **Probabilistic drop** via [`Self::drop_probability`] backed by a
//!   seeded `ChaCha20Rng`. Bitwise-deterministic across platforms; the
//!   `(numerator, denominator)` ratio (e.g. `(5, 100)` = 5%) avoids
//!   float comparisons that could differ between hosts.
//! - **One-shot duplicate** via [`Self::duplicate_next_count`] — emit
//!   `N` extra copies of the next send and reset to 0. Distinct from
//!   the cyclic `duplicate_every`.
//! - **Multi-packet reorder** via [`Self::reorder_buffer_size`] — hold
//!   the next `N` sends and flush them in **reverse arrival order**
//!   when the buffer fills. Generalises the pair-only
//!   `reorder_pairs`.

use std::time::Duration;

use bytes::Bytes;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Drop ratio expressed as `(numerator, denominator)` — e.g.
/// `DropProbability::pct(5)` for 5% loss. Stored as an integer ratio
/// instead of an `f32` to keep deterministic RNG comparisons
/// bitwise-identical across platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropProbability {
    pub numerator: u32,
    pub denominator: u32,
}

impl DropProbability {
    /// `pct(5)` = 5% drop rate. Saturates `n` at 100.
    pub const fn pct(n: u32) -> Self {
        Self {
            numerator: if n > 100 { 100 } else { n },
            denominator: 100,
        }
    }

    /// `per_thousand(50)` = 5% drop rate at thousand-granularity, for
    /// scenarios that want sub-percent precision.
    pub const fn per_thousand(n: u32) -> Self {
        Self {
            numerator: if n > 1000 { 1000 } else { n },
            denominator: 1000,
        }
    }

    /// Roll the RNG once and return `true` if the packet should be
    /// dropped. Caller advances per-send; the policy field's
    /// `drop_count` records how many drops actually fired (for
    /// diagnostics + invariant checks).
    pub fn roll(self, rng: &mut ChaCha20Rng) -> bool {
        if self.numerator == 0 || self.denominator == 0 {
            return false;
        }
        let r: u32 = rng.random_range(0..self.denominator);
        r < self.numerator
    }
}

/// Per-direction packet manipulation policy.
///
/// Default is lossless, in-order, zero-latency loopback.
///
/// The fields split into two groups:
///
/// - **Configuration** (set by tests): [`Self::drop_next`],
///   [`Self::drop_at_send_count`], [`Self::drop_probability`],
///   [`Self::latency`], [`Self::reorder_pairs`],
///   [`Self::reorder_buffer_size`], [`Self::duplicate_every`],
///   [`Self::duplicate_next_count`], [`Self::rng_seed`].
/// - **Internal pump state** (managed by the send path; tests can read
///   for diagnostics but should not mutate directly):
///   [`Self::send_count`], [`Self::drop_count`],
///   [`Self::duplicate_count`], [`Self::reorder_held`],
///   [`Self::reorder_buffer`], [`Self::rng`].
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

    /// Probabilistic per-packet drop with a seeded `ChaCha20Rng`. For
    /// the `sustained_5pct_loss_60s` Monte Carlo and similar
    /// chaos scenarios. Set [`Self::rng_seed`] for reproducibility.
    pub drop_probability: NetworkDirection<Option<DropProbability>>,

    /// Insert `latency` before delivering each packet in `direction`.
    /// Implemented via `tokio::time::sleep` on the sender side.
    pub latency: NetworkDirection<Duration>,

    /// Deliver packets in `direction` swapped in arrival pairs:
    /// `(1, 2, 3, 4)` → `(2, 1, 4, 3)`. The implementation holds the
    /// first packet of each pair in [`Self::reorder_held`] until the
    /// second arrives, then ships them in `(second, first)` order.
    pub reorder_pairs: NetworkDirection<bool>,

    /// Generalised reorder: hold up to `N` sends in
    /// [`Self::reorder_buffer`], then flush in **reverse arrival
    /// order** when the buffer is full. `N=0` disables. `N=2`
    /// behaves like [`Self::reorder_pairs`] but via the larger
    /// buffer path (use `reorder_pairs` for the dedicated fast path).
    /// Required by the `reorder_within_rx_window` scenario which
    /// needs to scramble more than two packets.
    pub reorder_buffer_size: NetworkDirection<u32>,

    /// When set to `Some(n)` with `n >= 1`, duplicate every Nth send
    /// in `direction` (so the receiver sees it twice). For example,
    /// `Some(2)` duplicates the 2nd, 4th, 6th... send. `None` or
    /// `Some(0)` = no duplication.
    pub duplicate_every: NetworkDirection<Option<u32>>,

    /// One-shot: duplicate the next send `N` extra times, then
    /// reset to 0. So `duplicate_next_count = 4` produces 5 copies
    /// of the next packet (1 original + 4 duplicates). Distinct
    /// from [`Self::duplicate_every`] which is cyclic. Required by
    /// the `duplicate_flood` scenario.
    pub duplicate_next_count: NetworkDirection<u32>,

    /// Optional seed for the deterministic RNG used by
    /// [`Self::drop_probability`]. When `None`, the RNG seeds from
    /// `0` so tests are still reproducible (any test using
    /// probabilistic drop should explicitly set a seed for
    /// debuggability).
    pub rng_seed: Option<u64>,

    /// Internal: per-direction send counter (1-indexed). Incremented
    /// **before** each send-policy decision so the first send sees
    /// `send_count == 1`.
    pub send_count: NetworkDirection<u32>,

    /// Internal: per-direction drop counter. Bumped each time any
    /// drop policy (drop_next, drop_at_send_count, drop_probability)
    /// fires. Useful for invariant assertions ("drop rate roughly
    /// matched the configured probability").
    pub drop_count: NetworkDirection<u32>,

    /// Internal: per-direction duplicate-cycle counter. Increments on
    /// every send; when it hits the configured `duplicate_every`
    /// value, a copy is emitted and the counter resets to 0.
    pub duplicate_count: NetworkDirection<u32>,

    /// Internal: per-direction reorder buffer (pair mode). Holds the
    /// first packet of each pair while waiting for the second.
    pub reorder_held: NetworkDirection<Option<Bytes>>,

    /// Internal: per-direction reorder buffer (multi-packet mode).
    /// When `reorder_buffer_size` is `N`, this fills to `N` then
    /// flushes reversed.
    pub reorder_buffer: NetworkDirection<Vec<Bytes>>,

    /// Internal: per-direction RNG state. Lazily initialised on
    /// first probabilistic-drop check using [`Self::rng_seed`] (or
    /// `0` if unset).
    pub rng: NetworkDirection<Option<ChaCha20Rng>>,
}

impl NetworkPolicy {
    /// Reset all per-direction internal counters. Useful between test
    /// phases where the test wants a clean slate (e.g., "ignore
    /// everything before this point — now drop the 2nd send").
    ///
    /// Does NOT reseed the RNG — tests that need a fresh RNG should
    /// set `rng_seed` and then drop the existing `rng` instances
    /// directly.
    pub fn reset_counters(&mut self) {
        self.send_count = NetworkDirection::default();
        self.drop_count = NetworkDirection::default();
        self.duplicate_count = NetworkDirection::default();
        self.reorder_held = NetworkDirection::default();
        self.reorder_buffer = NetworkDirection::default();
    }

    /// Force-seed (or reseed) the RNG for `direction`. Tests that
    /// want a fresh deterministic RNG mid-run call this after
    /// updating [`Self::rng_seed`].
    pub fn seed_rng(&mut self, direction: Direction, seed: u64) {
        *self.rng.get_mut(direction) = Some(ChaCha20Rng::seed_from_u64(seed));
    }

    /// Returns the per-direction RNG, lazily seeding from
    /// [`Self::rng_seed`] (default `0`) on first access.
    pub fn rng_mut(&mut self, direction: Direction) -> &mut ChaCha20Rng {
        let seed = self.rng_seed.unwrap_or(0);
        self.rng
            .get_mut(direction)
            .get_or_insert_with(|| ChaCha20Rng::seed_from_u64(seed))
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
        assert!(p.drop_probability.a_to_b.is_none());
        assert_eq!(p.latency.a_to_b, Duration::ZERO);
        assert!(!p.reorder_pairs.a_to_b);
        assert_eq!(p.reorder_buffer_size.a_to_b, 0);
        assert!(p.duplicate_every.a_to_b.is_none());
        assert_eq!(p.duplicate_next_count.a_to_b, 0);
        assert_eq!(p.send_count.a_to_b, 0);
        assert_eq!(p.drop_count.a_to_b, 0);
        assert_eq!(p.duplicate_count.a_to_b, 0);
        assert!(p.reorder_held.a_to_b.is_none());
        assert!(p.reorder_buffer.a_to_b.is_empty());
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
        p.drop_count.a_to_b = 99;
        p.reorder_buffer.a_to_b = vec![Bytes::from_static(b"held")];

        p.reset_counters();

        // Config preserved:
        assert_eq!(p.drop_next.a_to_b, 5);
        assert_eq!(p.duplicate_every.a_to_b, Some(3));
        // Internal state reset:
        assert_eq!(p.send_count.a_to_b, 0);
        assert_eq!(p.duplicate_count.b_to_a, 0);
        assert_eq!(p.drop_count.a_to_b, 0);
        assert!(p.reorder_buffer.a_to_b.is_empty());
    }

    #[test]
    fn drop_probability_pct_clamps_to_100() {
        assert_eq!(DropProbability::pct(200).numerator, 100);
        assert_eq!(DropProbability::pct(50).numerator, 50);
        assert_eq!(DropProbability::pct(0).numerator, 0);
    }

    #[test]
    fn drop_probability_roll_is_seeded_deterministic() {
        // Two RNGs with the same seed produce the same outcomes.
        let prob = DropProbability::pct(50);
        let mut rng_a = ChaCha20Rng::seed_from_u64(42);
        let mut rng_b = ChaCha20Rng::seed_from_u64(42);
        for _ in 0..100 {
            assert_eq!(prob.roll(&mut rng_a), prob.roll(&mut rng_b));
        }
    }

    #[test]
    fn drop_probability_zero_never_drops() {
        let prob = DropProbability::pct(0);
        let mut rng = ChaCha20Rng::seed_from_u64(7);
        for _ in 0..1000 {
            assert!(!prob.roll(&mut rng));
        }
    }

    #[test]
    fn drop_probability_hundred_always_drops() {
        let prob = DropProbability::pct(100);
        let mut rng = ChaCha20Rng::seed_from_u64(7);
        for _ in 0..1000 {
            assert!(prob.roll(&mut rng));
        }
    }

    #[test]
    fn rng_mut_seeds_lazily_from_rng_seed() {
        let mut p = NetworkPolicy {
            rng_seed: Some(123),
            ..Default::default()
        };
        // First access initialises:
        let val_a = p.rng_mut(Direction::AToB).random::<u32>();
        // Second access uses the SAME RNG instance (advances state):
        let val_b = p.rng_mut(Direction::AToB).random::<u32>();
        assert_ne!(val_a, val_b, "consecutive RNG reads must advance state");

        // Re-seeding via `seed_rng` replaces the RNG; next read
        // matches a fresh seed-123 RNG's first value:
        p.seed_rng(Direction::AToB, 123);
        let val_c = p.rng_mut(Direction::AToB).random::<u32>();
        assert_eq!(
            val_a, val_c,
            "re-seeding to the same seed reproduces the first value"
        );
    }
}
