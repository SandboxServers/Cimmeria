//! Deterministic 1-in-N counter for the per-packet log firehoses.
//!
//! The firehose framework itself (targets, the `FIREHOSES` table, the
//! emitters) is `cimmeria_wire::firehose`. The sampler lives here, at the
//! bottom of the crate graph, because `cimmeria-mercury` samples its own
//! per-datagram rows (TG-MER-09) and sits below `cimmeria-wire`.

use std::sync::atomic::{AtomicU64, Ordering};

/// Admits every N-th occurrence, starting with the first.
///
/// Count-based rather than time-based (compare `LogThrottle` in
/// `cell/space_manager/movement_telemetry/`): a firehose's rate is the thing
/// SigNoz is asked about, and a fixed ratio lets a query multiply it back.
/// Time-based throttling would make the ratio depend on load.
///
/// One sampler per call site, shared across every client and entity, so
/// production uses a `static` (or a field of a long-lived owner such as the
/// UDP transport). Tests build their own so the count does not depend on
/// which other tests ran first in the process.
#[derive(Debug)]
pub struct FirehoseSampler {
    every: u64,
    seen: AtomicU64,
}

impl FirehoseSampler {
    /// # Panics
    ///
    /// On `every == 0`; in a `static` initialiser that is a compile error.
    pub const fn new(every: u64) -> Self {
        assert!(every > 0, "a 1-in-0 sampler admits nothing");
        Self {
            every,
            seen: AtomicU64::new(0),
        }
    }

    /// The N in 1-in-N; logged as `sampled_1_in`.
    pub fn every(&self) -> u64 {
        self.every
    }

    /// Count one occurrence. `Some(suppressed)` when this one is the sample,
    /// where `suppressed` is the number of occurrences dropped since the
    /// previous sample (0 for the very first, `every - 1` after that).
    pub fn admit(&self) -> Option<u64> {
        // Relaxed: the counter orders nothing else, and a race between two
        // tasks only decides WHICH of them logs the sample.
        let n = self.seen.fetch_add(1, Ordering::Relaxed);
        if n.is_multiple_of(self.every) {
            Some(if n == 0 { 0 } else { self.every - 1 })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `n` occurrences through a fresh 1-in-`every` sampler. Returns the
    /// `suppressed` value of each admitted one.
    fn run(every: u64, n: u64) -> Vec<u64> {
        let s = FirehoseSampler::new(every);
        (0..n).filter_map(|_| s.admit()).collect()
    }

    #[test]
    fn sampler_admits_the_first_occurrence_then_every_nth() {
        assert_eq!(
            run(3, 7),
            [0, 2, 2],
            "occurrences 0, 3 and 6 are the samples"
        );
        assert_eq!(run(1, 4), [0, 0, 0, 0], "1-in-1 admits everything");
    }

    /// The rate identity SigNoz queries rely on: the number of samples is
    /// `ceil(n / every)`, and `sum(1 + suppressed)` reconstructs every
    /// occurrence up to and including the last sample.
    #[test]
    fn sampler_counts_add_up() {
        for (every, n) in [(53, 1_000), (101, 10_000), (53, 53), (53, 54), (101, 1)] {
            let samples = run(every, n);
            assert_eq!(
                samples.len() as u64,
                n.div_ceil(every),
                "every={every} n={n}"
            );
            let reconstructed: u64 = samples.iter().map(|s| 1 + s).sum();
            let last_sample_index = (samples.len() as u64 - 1) * every;
            assert_eq!(
                reconstructed,
                last_sample_index + 1,
                "every={every} n={n}: 1 + suppressed must sum to the occurrences \
                 seen by the last sample"
            );
        }
    }
}
