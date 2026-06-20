//! Sampling counter — emit one event every N invocations.
//!
//! `FEngineLoop::Tick` fires at 30-120 Hz; emitting one event per
//! tick would burn ~36-144k events/min on the wire. The 1/100
//! sampling drops that to a few hundred/min while still surfacing
//! frame-time anomalies (a 100x sample is plenty to spot a stall
//! in a typical 1-2 second freeze investigation window).
//!
//! Counter is `AtomicU64` so producers across threads share state.
//! Tests pin the modulo behaviour so a regression that flips the
//! `fetch_add` ordering or the comparison doesn't silently change
//! the sample rate.

use std::sync::atomic::{AtomicU64, Ordering};

/// Per-hook sampling counter. Constructed `const`-fn so it can
/// live in a `static` next to its hook function.
pub struct SamplingCounter {
    count: AtomicU64,
    /// Emit every Nth invocation. `1` = emit every call.
    /// `100` = emit 1 of every 100.
    every: u64,
}

impl SamplingCounter {
    /// Construct a counter that emits every `every` invocations.
    /// Use `1` for "always emit." Compile-time panics if
    /// `every == 0` would be ideal but `const fn` panic is awkward
    /// across MSRV — caller-side discipline instead.
    pub const fn new(every: u64) -> Self {
        Self {
            count: AtomicU64::new(0),
            every,
        }
    }

    /// Increment + return `true` when this invocation should emit.
    /// Threadsafe via `Relaxed` ordering — for sampling we only
    /// need eventual consistency on the counter, not a happens-
    /// before relationship with anything else.
    pub fn should_emit(&self) -> bool {
        let n = self.count.fetch_add(1, Ordering::Relaxed);
        self.every > 0 && n.is_multiple_of(self.every)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Emits on N=0, N, 2N, 3N, ... — skips the others.
    #[test]
    fn samples_every_n() {
        let s = SamplingCounter::new(3);
        let emits: Vec<bool> = (0..10).map(|_| s.should_emit()).collect();
        // 0 → emit, 1 skip, 2 skip, 3 emit, 4 skip, 5 skip, 6 emit, ...
        assert_eq!(
            emits,
            vec![true, false, false, true, false, false, true, false, false, true]
        );
    }

    /// `every = 1` is "emit every invocation."
    #[test]
    fn every_one_always_emits() {
        let s = SamplingCounter::new(1);
        for _ in 0..50 {
            assert!(s.should_emit());
        }
    }

    /// Threadsafe — no double-counting under concurrent emit.
    /// 4 threads × 100 invocations × `every = 5` should produce
    /// exactly (4*100)/5 = 80 emits.
    #[test]
    fn threadsafe_emit_count() {
        use std::sync::Arc;
        let s = Arc::new(SamplingCounter::new(5));
        let mut handles = vec![];
        let counter = Arc::new(AtomicU64::new(0));
        for _ in 0..4 {
            let s = s.clone();
            let counter = counter.clone();
            handles.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    if s.should_emit() {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(counter.load(Ordering::Relaxed), 80);
    }
}
