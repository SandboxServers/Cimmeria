//! One token bucket, on a caller-supplied clock.
//!
//! Integer tokens and whole refill periods, so a test that steps the clock by
//! exact durations gets exact answers (no `f64` drift at the boundary).

use std::time::Instant;

use super::limits::{BucketSpec, NOTIFY_INTERVAL};
use super::RateDecision;

/// A per-player, per-category token bucket. Starts full.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    spec: BucketSpec,
    tokens: u32,
    /// The instant the last whole token was credited. Partial progress
    /// towards the next token is kept by advancing this by whole periods
    /// only, never to `now`, unless the bucket filled.
    last_refill: Instant,
    /// When the last `notify: true` was handed out.
    last_notify: Option<Instant>,
}

impl TokenBucket {
    pub fn new(spec: BucketSpec, now: Instant) -> Self {
        Self {
            spec,
            tokens: spec.burst,
            last_refill: now,
            last_notify: None,
        }
    }

    /// Take one token if there is one. An `Instant` earlier than one this
    /// bucket has already seen counts as no time passing.
    pub fn check(&mut self, now: Instant) -> RateDecision {
        self.refill(now);
        if self.tokens > 0 {
            self.tokens -= 1;
            return RateDecision::Allowed;
        }
        let notify = self
            .last_notify
            .is_none_or(|t| now.saturating_duration_since(t) >= NOTIFY_INTERVAL);
        if notify {
            self.last_notify = Some(now);
        }
        RateDecision::Limited { notify }
    }

    fn refill(&mut self, now: Instant) {
        if self.tokens >= self.spec.burst {
            // Full: time spent full earns nothing, so restart the period here.
            self.last_refill = now.max(self.last_refill);
            return;
        }
        let elapsed = now.saturating_duration_since(self.last_refill).as_nanos();
        let period = self.spec.refill_every.as_nanos().max(1);
        let gained = elapsed / period;
        if gained == 0 {
            return;
        }
        let missing = u128::from(self.spec.burst - self.tokens);
        if gained >= missing {
            self.tokens = self.spec.burst;
            self.last_refill = now;
        } else {
            // `gained < missing <= burst`, so it fits a u32.
            let gained = gained as u32;
            self.tokens += gained;
            self.last_refill += self.spec.refill_every * gained;
        }
    }
}
