//! Session counters, and the rule for when a count is worth a log line.
//!
//! The DLL has no telemetry channel of its own, so these counters plus the
//! log are how a tester sees what happened: how many calls were claimed,
//! delivered, or dropped and why.

use std::sync::atomic::{AtomicU64, Ordering};

/// Everything the patch counts. Lock-free, so the network thread can bump
/// them without blocking.
#[derive(Debug, Default)]
pub struct Counters {
    /// Black Market calls claimed and decoded on the network thread.
    pub received: AtomicU64,
    /// Black Market calls for an entity that is not the local player. Left
    /// to the client's own (silent) drop.
    pub not_local_player: AtomicU64,
    /// Claimed calls whose arguments did not decode.
    pub decode_failed: AtomicU64,
    /// Decoded calls dropped because the queue was full.
    pub dropped_queue_full: AtomicU64,
    /// Calls handed to the Lua overlay without a Lua error.
    pub delivered: AtomicU64,
    /// Calls dropped because the global `CimmeriaBM` table is missing: the
    /// UI overlay is not installed.
    pub dropped_no_overlay: AtomicU64,
    /// Calls dropped because `CimmeriaBM` has no function for them.
    pub dropped_no_handler: AtomicU64,
    /// Calls whose Lua handler raised an error.
    pub handler_failed: AtomicU64,
}

impl Counters {
    /// All zero.
    pub const fn new() -> Self {
        Self {
            received: AtomicU64::new(0),
            not_local_player: AtomicU64::new(0),
            decode_failed: AtomicU64::new(0),
            dropped_queue_full: AtomicU64::new(0),
            delivered: AtomicU64::new(0),
            dropped_no_overlay: AtomicU64::new(0),
            dropped_no_handler: AtomicU64::new(0),
            handler_failed: AtomicU64::new(0),
        }
    }
}

/// Add one to `counter` and return the new value.
pub fn bump(counter: &AtomicU64) -> u64 {
    counter.fetch_add(1, Ordering::Relaxed) + 1
}

/// Whether the `n`th occurrence of something deserves a log line: the
/// first, the tenth, the hundredth and so on. Keeps a server that spams a
/// bad payload from filling the log.
pub fn is_log_worthy(n: u64) -> bool {
    let mut p = 1u64;
    loop {
        if n == p {
            return true;
        }
        if n < p {
            return false;
        }
        match p.checked_mul(10) {
            Some(next) => p = next,
            None => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_returns_the_new_count() {
        let c = AtomicU64::new(0);
        assert_eq!(bump(&c), 1);
        assert_eq!(bump(&c), 2);
    }

    #[test]
    fn powers_of_ten_are_log_worthy() {
        let hits: Vec<u64> = (0..=1000).filter(|n| is_log_worthy(*n)).collect();
        assert_eq!(hits, [1, 10, 100, 1000]);
        assert!(is_log_worthy(10_000_000_000_000_000_000));
        assert!(!is_log_worthy(u64::MAX));
    }
}
