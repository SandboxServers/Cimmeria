//! Per-channel rate limiter: a simple token bucket.

use std::time::Instant;

/// Simple token bucket. `capacity` tokens refill at `refill_per_sec`.
/// `try_acquire` returns `true` if a token is available, `false` if not.
pub(super) struct TokenBucket {
    /// Rate this bucket was constructed for (per minute). Stored so the
    /// sender task can detect a live-reload rate change and rebuild
    /// without losing accumulated tokens needlessly.
    pub(super) rate_per_min: u32,
    capacity: f64,
    tokens: f64,
    refill_per_sec: f64,
    last_refill: Instant,
}

impl TokenBucket {
    pub(super) fn new(rate_per_min: u32) -> Self {
        let per_sec = rate_per_min as f64 / 60.0;
        // Burst capacity = 1 minute's worth (or 5 — whichever is smaller —
        // matches Discord's 5-msg burst budget).
        let capacity = rate_per_min.min(5) as f64;
        Self {
            rate_per_min,
            capacity,
            tokens: capacity,
            refill_per_sec: per_sec,
            last_refill: Instant::now(),
        }
    }

    pub(super) fn try_acquire(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        self.last_refill = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn token_bucket_caps_to_burst() {
        let mut bucket = TokenBucket::new(60); // 1/sec, burst 5
                                               // Burst of 5 should all succeed.
        for _ in 0..5 {
            assert!(bucket.try_acquire(), "burst-budget acquires must succeed");
        }
        // 6th immediately fails.
        assert!(!bucket.try_acquire(), "6th acquire must fail before refill");
    }

    #[tokio::test]
    async fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(600); // 10/sec
        for _ in 0..5 {
            bucket.try_acquire();
        }
        assert!(!bucket.try_acquire());
        tokio::time::sleep(Duration::from_millis(200)).await;
        // 200ms × 10/sec = 2 tokens.
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
    }
}
