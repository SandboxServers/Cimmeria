//! Per-name rate limit for hooks that see a stream of named events.
//!
//! A fixed 1-in-N sampler (`SamplingCounter`) is wrong for a hook whose
//! events differ in kind: one hot name (an input action every frame) would
//! drown the rare one an agent is waiting for (a dialog opening). This
//! keeps a token bucket per name instead. A name that fires rarely always
//! gets through; a hot name is held to [`RATE_PER_SEC`] with a burst of
//! [`BURST`], and the next event that does get through carries how many
//! were suppressed in between, so counts stay recoverable.
//!
//! The table is capped at [`MAX_NAMES`]; names beyond it share one
//! overflow bucket, so a flood of distinct names cannot grow memory
//! without bound inside the game process.

use std::collections::HashMap;

/// Events allowed back to back before the rate applies.
pub const BURST: u32 = 8;

/// Sustained events per second per name.
pub const RATE_PER_SEC: u32 = 4;

/// Distinct names tracked before the overflow bucket takes over.
pub const MAX_NAMES: usize = 1024;

/// Name the overflow bucket reports under.
pub const OVERFLOW_NAME: &str = "<overflow>";

/// Token amounts are kept in thousandths so refill needs no floats.
const MILLI: u64 = 1000;

#[derive(Debug, Clone, Copy)]
struct Bucket {
    /// Thousandths of a token.
    tokens_milli: u64,
    last_ms: u64,
    suppressed: u64,
}

impl Bucket {
    fn full(now_ms: u64, burst: u32) -> Self {
        Self {
            tokens_milli: u64::from(burst) * MILLI,
            last_ms: now_ms,
            suppressed: 0,
        }
    }
}

/// What to do with one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Emit it. `suppressed` events of the same name were dropped since
    /// the last one emitted.
    Emit {
        /// Dropped since the previous emitted event of this name.
        suppressed: u64,
    },
    /// Drop it (counted into the next `Emit`).
    Suppress,
}

/// The per-name token buckets.
#[derive(Debug)]
pub struct NameThrottle {
    buckets: HashMap<String, Bucket>,
    burst: u32,
    rate_per_sec: u32,
}

impl Default for NameThrottle {
    fn default() -> Self {
        Self::with_limits(BURST, RATE_PER_SEC)
    }
}

impl NameThrottle {
    /// An empty table with the default limits ([`BURST`], [`RATE_PER_SEC`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty table with its own limits: `burst` events back to back, then
    /// `rate_per_sec` sustained per name. A `burst` of 0 would never emit, so
    /// it is raised to 1.
    pub fn with_limits(burst: u32, rate_per_sec: u32) -> Self {
        Self {
            buckets: HashMap::new(),
            burst: burst.max(1),
            rate_per_sec,
        }
    }

    /// Decide for one event named `name` at monotonic time `now_ms`.
    pub fn check(&mut self, name: &str, now_ms: u64) -> Decision {
        let key = if self.buckets.contains_key(name) || self.buckets.len() < MAX_NAMES {
            name
        } else {
            OVERFLOW_NAME
        };
        let burst = self.burst;
        let bucket = match self.buckets.get_mut(key) {
            Some(b) => b,
            None => self
                .buckets
                .entry(key.to_string())
                .or_insert_with(|| Bucket::full(now_ms, burst)),
        };

        let elapsed = now_ms.saturating_sub(bucket.last_ms);
        bucket.last_ms = now_ms;
        let cap = u64::from(burst) * MILLI;
        bucket.tokens_milli =
            (bucket.tokens_milli + elapsed * u64::from(self.rate_per_sec)).min(cap);

        if bucket.tokens_milli >= MILLI {
            bucket.tokens_milli -= MILLI;
            let suppressed = std::mem::take(&mut bucket.suppressed);
            Decision::Emit { suppressed }
        } else {
            bucket.suppressed += 1;
            Decision::Suppress
        }
    }

    /// Number of distinct buckets (overflow included).
    #[cfg(test)]
    fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emitted(d: Decision) -> bool {
        matches!(d, Decision::Emit { .. })
    }

    /// The case this exists for: a rare event is never lost behind a hot
    /// one.
    #[test]
    fn a_rare_name_gets_through_while_a_hot_one_is_capped() {
        let mut t = NameThrottle::new();
        let mut hot_emitted = 0;
        for i in 0..1000 {
            if emitted(t.check("Event_Action_MouseLook", i)) {
                hot_emitted += 1;
            }
        }
        // One second of frames: burst plus about RATE_PER_SEC refills.
        assert!(
            hot_emitted <= (BURST + RATE_PER_SEC) as usize,
            "{hot_emitted}"
        );
        assert_eq!(
            t.check("Event_NetIn_onDialogDisplay", 1000),
            Decision::Emit { suppressed: 0 }
        );
    }

    #[test]
    fn the_burst_passes_then_the_rest_is_suppressed() {
        let mut t = NameThrottle::new();
        for _ in 0..BURST {
            assert!(emitted(t.check("x", 0)));
        }
        assert_eq!(t.check("x", 0), Decision::Suppress);
        assert_eq!(t.check("x", 0), Decision::Suppress);
    }

    /// Suppressed events are reported on the next one that gets through,
    /// then the count resets.
    #[test]
    fn the_next_emit_carries_the_suppressed_count() {
        let mut t = NameThrottle::new();
        for _ in 0..BURST {
            t.check("x", 0);
        }
        for _ in 0..5 {
            assert_eq!(t.check("x", 0), Decision::Suppress);
        }
        // 1000 / RATE_PER_SEC ms refills exactly one token.
        let refill_ms = 1000 / u64::from(RATE_PER_SEC);
        assert_eq!(t.check("x", refill_ms), Decision::Emit { suppressed: 5 });
        assert_eq!(t.check("x", refill_ms), Decision::Suppress);
        assert_eq!(
            t.check("x", 2 * refill_ms),
            Decision::Emit { suppressed: 1 }
        );
    }

    #[test]
    fn tokens_never_exceed_the_burst_after_a_long_idle() {
        let mut t = NameThrottle::new();
        t.check("x", 0);
        let mut n = 0;
        while emitted(t.check("x", 3_600_000)) {
            n += 1;
        }
        assert_eq!(n, BURST);
    }

    /// The `firehose` switch swaps in bigger limits: a longer burst, and a
    /// faster refill (`rate` tokens a second, so `1000 / rate` ms a token).
    #[test]
    fn custom_limits_change_the_burst_and_the_refill() {
        let mut t = NameThrottle::with_limits(64, 64);
        for _ in 0..64 {
            assert!(emitted(t.check("x", 0)));
        }
        assert_eq!(t.check("x", 0), Decision::Suppress);
        // 1000 / 64 = 15.6 ms: 16 ms refills one token.
        assert_eq!(t.check("x", 16), Decision::Emit { suppressed: 1 });
        // A zero burst would never emit; it is raised to one.
        let mut t = NameThrottle::with_limits(0, 0);
        assert!(emitted(t.check("x", 0)));
        assert_eq!(t.check("x", 0), Decision::Suppress);
    }

    #[test]
    fn names_beyond_the_cap_share_the_overflow_bucket() {
        let mut t = NameThrottle::new();
        for i in 0..MAX_NAMES {
            t.check(&format!("name{i}"), 0);
        }
        assert_eq!(t.bucket_count(), MAX_NAMES);
        for i in 0..100 {
            t.check(&format!("late{i}"), 0);
        }
        assert_eq!(
            t.bucket_count(),
            MAX_NAMES + 1,
            "only the overflow bucket is added"
        );
        // A name already in the table keeps its own bucket.
        assert!(emitted(t.check("name0", 10_000)));
    }
}
