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

/// The most keys the limiter holds. A new key past this drops the window that
/// expires soonest first (never the global sign-up window), so a flood of
/// distinct keys cannot grow memory without end.
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
pub struct RateLimiter {
    windows: Mutex<HashMap<(Bucket, String), Window>>,
    signups_per_ip_per_day: u32,
    signups_global_per_hour: u32,
}

/// Prints no keys, so a log line never carries client IPs or account names.
impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimiter").finish_non_exhaustive()
    }
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
        let limit = match bucket {
            Bucket::AnyRoute => 60,
            Bucket::Authorize => 20,
            Bucket::Token => 30,
            Bucket::LinkExistingIp | Bucket::LinkExistingAccount => 5,
            Bucket::SignupIp => self.signups_per_ip_per_day,
            Bucket::SignupGlobal => self.signups_global_per_hour,
        };
        (limit, window_len(bucket))
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<(Bucket, String), Window>> {
        self.windows.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The length of the fixed window of one bucket.
fn window_len(bucket: Bucket) -> Duration {
    match bucket {
        Bucket::AnyRoute => MINUTE,
        Bucket::Authorize | Bucket::Token => TEN_MINUTES,
        Bucket::LinkExistingIp | Bucket::LinkExistingAccount => FIFTEEN_MINUTES,
        Bucket::SignupGlobal => HOUR,
        Bucket::SignupIp => DAY,
    }
}

/// Drops the window that expires soonest. The global sign-up window is never
/// dropped, so a flood of other keys cannot reopen the server-wide cap.
fn evict_oldest(windows: &mut HashMap<(Bucket, String), Window>) {
    let victim = windows
        .iter()
        .filter(|((b, _), _)| *b != Bucket::SignupGlobal)
        .min_by_key(|((b, _), w)| w.start + window_len(*b))
        .map(|(k, _)| k.clone());
    if let Some(key) = victim {
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
        assert_eq!(refused.secs, 900);
        assert_eq!(
            rl.check(
                Bucket::LinkExistingIp,
                "203.0.113.5",
                t0 + Duration::from_secs(100)
            )
            .unwrap_err()
            .secs,
            800
        );
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
    fn limiter_drops_the_oldest_key_first() {
        let rl = limiter();
        let t0 = Instant::now();
        assert!(rl.check(Bucket::AnyRoute, "old", t0).is_ok());
        for i in 0..MAX_KEYS - 1 {
            let key = format!("key-{i}");
            let later = t0 + Duration::from_secs(1);
            assert!(rl.check(Bucket::AnyRoute, &key, later).is_ok());
        }
        let newest = t0 + Duration::from_secs(2);
        assert!(rl.check(Bucket::AnyRoute, "new", newest).is_ok());
        assert_eq!(rl.lock().len(), MAX_KEYS);
        assert!(!rl
            .lock()
            .contains_key(&(Bucket::AnyRoute, "old".to_string())));
        assert!(rl
            .lock()
            .contains_key(&(Bucket::AnyRoute, "new".to_string())));
    }

    #[test]
    fn limiter_eviction_spares_the_global_signup_window() {
        let limits = Limits {
            signups_per_ip_per_day: 3,
            signups_global_per_hour: 1,
            min_provider_account_age_days: 0,
        };
        let rl = RateLimiter::with_limits(&limits);
        let t0 = Instant::now();
        assert!(rl.check(Bucket::SignupGlobal, "", t0).is_ok());
        let t1 = t0 + Duration::from_secs(1);
        for i in 0..MAX_KEYS {
            let key = format!("flood-{i}");
            assert!(rl.check(Bucket::AnyRoute, &key, t1).is_ok());
        }
        let t2 = t0 + Duration::from_secs(2);
        assert!(rl.check(Bucket::SignupGlobal, "", t2).is_err());
    }

    #[test]
    fn bucket_limits_and_windows_match_c7() {
        let t0 = Instant::now();
        let cases: [(Bucket, u32, u64); 5] = [
            (Bucket::AnyRoute, 60, 60),
            (Bucket::Authorize, 20, 600),
            (Bucket::Token, 30, 600),
            (Bucket::LinkExistingIp, 5, 900),
            (Bucket::LinkExistingAccount, 5, 900),
        ];
        for (bucket, limit, window_secs) in cases {
            let rl = limiter();
            for _ in 0..limit {
                assert!(rl.check(bucket, "203.0.113.5", t0).is_ok(), "{bucket:?}");
            }
            assert!(rl.check(bucket, "203.0.113.5", t0).is_err(), "{bucket:?}");
            let before = t0 + Duration::from_secs(window_secs - 1);
            assert!(
                rl.check(bucket, "203.0.113.5", before).is_err(),
                "{bucket:?}"
            );
            let after = t0 + Duration::from_secs(window_secs);
            assert!(rl.check(bucket, "203.0.113.5", after).is_ok(), "{bucket:?}");
        }
    }

    #[test]
    fn limiter_debug_shows_no_keys() {
        let rl = limiter();
        let now = Instant::now();
        assert!(rl.check(Bucket::AnyRoute, "203.0.113.77", now).is_ok());
        assert!(!format!("{rl:?}").contains("203.0.113.77"));
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
