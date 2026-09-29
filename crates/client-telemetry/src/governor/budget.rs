//! Per-target rate budget for [`super::Class::Budgeted`] events.
//!
//! A token bucket per target: `burst` events back to back, then
//! `per_sec` sustained. Over budget is not a drop: the governor puts the
//! event in the target's rollup. At most [`MAX_TARGETS`] targets get a
//! bucket of their own; the rest share one overflow bucket, so a flood of
//! distinct target names cannot grow memory.

use std::collections::HashMap;

/// Targets with their own bucket.
pub const MAX_TARGETS: usize = 256;

const OVERFLOW: &str = "<overflow>";

/// Token amounts are kept in thousandths so refill needs no floats.
const MILLI: u64 = 1000;

#[derive(Debug, Clone, Copy)]
struct Bucket {
    tokens_milli: u64,
    last_ms: i64,
}

/// The per-target buckets.
#[derive(Debug)]
pub struct Budget {
    burst: u32,
    per_sec: u32,
    buckets: HashMap<String, Bucket>,
}

impl Budget {
    /// Buckets holding `burst` tokens, refilled at `per_sec`.
    pub fn new(burst: u32, per_sec: u32) -> Self {
        Self {
            burst,
            per_sec,
            buckets: HashMap::new(),
        }
    }

    /// Take one token for `target` at wall time `now_ms`. `true` = under
    /// budget.
    pub fn take(&mut self, target: &str, now_ms: i64) -> bool {
        let key = if self.buckets.contains_key(target) || self.buckets.len() < MAX_TARGETS {
            target
        } else {
            OVERFLOW
        };
        let cap = u64::from(self.burst) * MILLI;
        let bucket = self.buckets.entry(key.to_string()).or_insert(Bucket {
            tokens_milli: cap,
            last_ms: now_ms,
        });
        // A clock that steps backwards refills nothing rather than panicking.
        let elapsed = u64::try_from(now_ms - bucket.last_ms).unwrap_or(0);
        bucket.last_ms = bucket.last_ms.max(now_ms);
        bucket.tokens_milli = (bucket.tokens_milli + elapsed * u64::from(self.per_sec)).min(cap);
        if bucket.tokens_milli >= MILLI {
            bucket.tokens_milli -= MILLI;
            true
        } else {
            false
        }
    }
}
