//! Deterministic 1-in-N counter for the firehose samples.

use std::sync::atomic::{AtomicU64, Ordering};

/// Admits every N-th occurrence, starting with the first.
///
/// Count-based rather than time-based (compare `LogThrottle` in
/// `cell/space_manager/movement_telemetry/`): a firehose's rate is the thing
/// SigNoz is asked about, and a fixed ratio lets a query multiply it back.
/// Time-based throttling would make the ratio depend on load.
///
/// One sampler per call site, shared across every client and entity, so
/// production uses a `static`. Tests build their own so the count does not
/// depend on which other tests ran first in the process.
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
