//! The in-memory rate limits of the identity routes (contract C7).
//!
//! Each [`Bucket`] keeps a fixed window per key: the first request opens the
//! window, and a request counts only while the window is open and under its
//! limit. The clock is passed in, so tests need no sleeps. The state lives in
//! this process and resets on restart, which is the contract's choice.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::config::Limits;

/// The most keys the limiter holds. A new key past this drops the oldest
/// window first, so a flood of distinct keys cannot grow memory without end.
pub const MAX_KEYS: usize = 100_000;

const MINUTE: Duration = Duration::from_secs(60);
const TEN_MINUTES: Duration = Duration::from_secs(600);
const FIFTEEN_MINUTES: Duration = Duration::from_secs(900);
const HOUR: Duration = Duration::from_secs(3600);
const DAY: Duration = Duration::from_secs(86_400);

/// Milliseconds in one day, for the account-age check.
const MS_PER_DAY: i64 = 86_400_000;

/// Discord's snowflake epoch (2015-01-01), in milliseconds.
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

/// The limit a bucket counts against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bucket {
    /// Any `/auth/v1/*` request, per IP.
    AnyRoute,
    /// `authorize`, per IP.
    Authorize,
    /// `token`, per IP.
    Token,
    /// `link/existing`, per IP.
    LinkExistingIp,
    /// `link/existing`, per target account name.
    LinkExistingAccount,
    /// Account creation, per IP.
    SignupIp,
    /// Account creation, across the whole server (key `""`).
    SignupGlobal,
}

/// How long to wait before a refused request may succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAfter {
    /// Whole seconds, at least 1.
    pub secs: u64,
}

#[derive(Debug, Clone, Copy)]
struct Window {
    start: Instant,
    count: u32,
}

/// The keyed fixed-window limiter. Shared across the request handlers.
#[derive(Debug)]
pub struct RateLimiter {
    windows: Mutex<HashMap<(Bucket, String), Window>>,
    signups_per_ip_per_day: u32,
    signups_global_per_hour: u32,
}

impl RateLimiter {
    /// Builds the limiter. The two sign-up limits come from the config; the
    /// rest are the C7 numbers.
    pub fn with_limits(limits: &Limits) -> RateLimiter {
        RateLimiter {
            windows: Mutex::new(HashMap::new()),
            signups_per_ip_per_day: limits.signups_per_ip_per_day,
            signups_global_per_hour: limits.signups_global_per_hour,
        }
    }

    /// Counts one request under `bucket` and `key` at time `now`. Allowed
    /// requests count; a refused one returns how long to wait.
    pub fn check(&self, bucket: Bucket, key: &str, now: Instant) -> Result<(), RetryAfter> {
        let (limit, window_len) = self.policy(bucket);
        let mut windows = self.lock();
        let map_key = (bucket, key.to_string());
        if !windows.contains_key(&map_key) && windows.len() >= MAX_KEYS {
            evict_oldest(&mut windows);
        }
        let entry = windows.entry(map_key).or_insert(Window {
            start: now,
            count: 0,
        });
        let elapsed = now.saturating_duration_since(entry.start);
        if elapsed >= window_len {
            *entry = Window {
                start: now,
                count: 0,
            };
        }
        if entry.count < limit {
            entry.count += 1;
            return Ok(());
        }
        let left = window_len.saturating_sub(now.saturating_duration_since(entry.start));
        let secs = left.as_secs() + u64::from(left.subsec_nanos() > 0);
        Err(RetryAfter { secs: secs.max(1) })
    }

    /// The limit and window length of one bucket.
    fn policy(&self, bucket: Bucket) -> (u32, Duration) {
        match bucket {
            Bucket::AnyRoute => (60, MINUTE),
            Bucket::Authorize => (20, TEN_MINUTES),
            Bucket::Token => (30, TEN_MINUTES),
            Bucket::LinkExistingIp | Bucket::LinkExistingAccount => (5, FIFTEEN_MINUTES),
            Bucket::SignupIp => (self.signups_per_ip_per_day, DAY),
            Bucket::SignupGlobal => (self.signups_global_per_hour, HOUR),
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<(Bucket, String), Window>> {
        self.windows.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Drops the window that opened first.
fn evict_oldest(windows: &mut HashMap<(Bucket, String), Window>) {
    let oldest = windows
        .iter()
        .min_by_key(|(_, w)| w.start)
        .map(|(k, _)| k.clone());
    if let Some(key) = oldest {
        windows.remove(&key);
    }
}

/// The creation time of a Discord account, in Unix milliseconds, read from
/// its snowflake id: the id's high bits count milliseconds since 2015-01-01.
pub fn discord_created_ms(snowflake: u64) -> i64 {
    ((snowflake >> 22) + DISCORD_EPOCH_MS) as i64
}

/// Whether a provider account is old enough to sign up. Always true when
/// `min_days` is 0 or the provider gave no creation time.
pub fn old_enough(created_ms: Option<i64>, now_ms: i64, min_days: u32) -> bool {
    match created_ms {
        Some(created) if min_days > 0 => {
            now_ms.saturating_sub(created) >= i64::from(min_days) * MS_PER_DAY
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter() -> RateLimiter {
        RateLimiter::with_limits(&Limits::default())
    }

    #[test]
    fn limiter_counts_per_bucket_and_key() {
        let rl = limiter();
        let t0 = Instant::now();
        for _ in 0..5 {
            assert!(rl.check(Bucket::LinkExistingIp, "203.0.113.5", t0).is_ok());
        }
        let refused = rl
            .check(Bucket::LinkExistingIp, "203.0.113.5", t0)
            .expect_err("the sixth request in the window is refused");
        assert!(refused.secs > 0);
        assert!(rl.check(Bucket::LinkExistingIp, "203.0.113.6", t0).is_ok());
        // The same key in another bucket has its own count.
        assert!(rl
            .check(Bucket::LinkExistingAccount, "203.0.113.5", t0)
            .is_ok());
        let later = t0 + Duration::from_secs(15 * 60);
        assert!(rl
            .check(Bucket::LinkExistingIp, "203.0.113.5", later)
            .is_ok());
    }

    #[test]
    fn limiter_caps_its_keys() {
        let rl = limiter();
        let now = Instant::now();
        for i in 0..=MAX_KEYS {
            let key = format!("key-{i}");
            assert!(rl.check(Bucket::AnyRoute, &key, now).is_ok());
        }
        assert!(rl.lock().len() <= MAX_KEYS);
    }

    #[test]
    fn signup_limits_come_from_config() {
        let limits = Limits {
            signups_per_ip_per_day: 1,
            signups_global_per_hour: 2,
            min_provider_account_age_days: 0,
        };
        let rl = RateLimiter::with_limits(&limits);
        let now = Instant::now();
        assert!(rl.check(Bucket::SignupIp, "203.0.113.5", now).is_ok());
        assert!(rl.check(Bucket::SignupIp, "203.0.113.5", now).is_err());
        assert!(rl.check(Bucket::SignupGlobal, "", now).is_ok());
        assert!(rl.check(Bucket::SignupGlobal, "", now).is_ok());
        assert!(rl.check(Bucket::SignupGlobal, "", now).is_err());
    }

    #[test]
    fn discord_snowflake_age() {
        assert_eq!(
            discord_created_ms(175_928_847_299_117_063),
            1_462_015_105_796
        );
    }

    #[test]
    fn old_enough_rules() {
        let now = 1_000 * MS_PER_DAY;
        assert!(old_enough(None, now, 30));
        assert!(old_enough(Some(now), now, 0));
        assert!(!old_enough(Some(now - MS_PER_DAY), now, 2));
        assert!(old_enough(Some(now - 2 * MS_PER_DAY), now, 2));
    }
}
