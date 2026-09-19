//! The live-DB test gate: open a pool against `DATABASE_URL`, skip when
//! no database is configured, and **fail** when one is configured but
//! unreachable.
//!
//! Re-exported from [`crate::test_support`], so call sites keep writing
//! `use crate::test_support::require_db_or_skip;`. See
//! `docs/architecture/integration-test-infra.md` for local setup.
//!
//! # Why unreachable fails instead of skipping (#615)
//!
//! A skip reports as a pass. When a configured-but-unreachable database
//! (wrong port, wrong credentials, server down) also skipped, a run
//! against it executed none of the live-DB guards and still came back
//! green, which looked exactly like a real pass. Setting `DATABASE_URL` is
//! a request to run the live-DB tier, so not being able to reach the
//! database is an error.

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::PgPool;

/// Why [`test_pool`] produced no pool.
#[derive(Debug)]
pub(crate) enum SkipReason {
    /// `DATABASE_URL` was unset or empty: a legitimate skip (fresh
    /// checkout, or CI's no-DB pass).
    NotConfigured,
    /// `DATABASE_URL` was set but `connect()` failed. Carries sqlx's
    /// error text plus the target host, port and database (never the
    /// password). [`require_db_or_skip!`] turns this into a test failure.
    ConnectFailed(String),
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::NotConfigured => write!(f, "DATABASE_URL not set"),
            SkipReason::ConnectFailed(e) => write!(f, "DATABASE_URL set but connect failed: {e}"),
        }
    }
}

/// Open a `PgPool` against the developer-supplied `DATABASE_URL`, or
/// return a [`SkipReason`] explaining why no pool was produced.
///
/// Bounded to 4 connections — high enough for tests that exercise
/// concurrent paths (drainer + caller in parallel), low enough that
/// a careless test loop can't exhaust a hand-tuned local Postgres.
pub(crate) async fn test_pool() -> Result<PgPool, SkipReason> {
    test_pool_from_url(std::env::var("DATABASE_URL").ok().as_deref()).await
}

/// [`test_pool`] with the URL passed in rather than read from the
/// environment, so the gate can be tested without mutating process env.
pub(crate) async fn test_pool_from_url(url: Option<&str>) -> Result<PgPool, SkipReason> {
    let url = match url {
        Some(u) if !u.is_empty() => u,
        _ => return Err(SkipReason::NotConfigured),
    };
    let options: PgConnectOptions = url
        .parse()
        .map_err(|e| SkipReason::ConnectFailed(format!("invalid DATABASE_URL: {e}")))?;
    // A refused connection is retried until the acquire timeout, so sqlx's
    // error usually just says "pool timed out". Name the target so a wrong
    // port or host is obvious from the failure alone.
    let target = format!(
        "{}:{}/{}",
        options.get_host(),
        options.get_port(),
        options.get_database().unwrap_or("")
    );
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| SkipReason::ConnectFailed(format!("{e} (target {target})")))
}

/// Decide what a live-DB test does with a [`test_pool`] result: run with
/// the pool (`Some`), skip (`None`, only when `DATABASE_URL` is unset), or
/// panic when `DATABASE_URL` is set but the database is unreachable.
///
/// `test` names the caller in the skip/panic message.
pub(crate) fn pool_or_skip(result: Result<PgPool, SkipReason>, test: &str) -> Option<PgPool> {
    match result {
        Ok(pool) => Some(pool),
        Err(SkipReason::NotConfigured) => {
            eprintln!(
                "{test}: skipping live-DB test ({})",
                SkipReason::NotConfigured
            );
            None
        }
        Err(reason @ SkipReason::ConnectFailed(_)) => panic!(
            "{test}: {reason}. DATABASE_URL is set, so the live-DB tests must run: \
             fix the URL (the bundled Postgres listens on :5433) or unset it to skip them."
        ),
    }
}

/// Get a live-DB pool, or return early from the test when no database is
/// configured. Panics when `DATABASE_URL` is set but unreachable — see the
/// module docs.
///
/// ```ignore
/// #[tokio::test]
/// async fn my_db_test() {
///     let pool = require_db_or_skip!();
///     // ... test body uses pool ...
/// }
/// ```
macro_rules! require_db_or_skip {
    () => {{
        match $crate::test_support::pool_or_skip(
            $crate::test_support::test_pool().await,
            module_path!(),
        ) {
            Some(p) => p,
            None => return,
        }
    }};
}

pub(crate) use require_db_or_skip;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unset_or_empty_url_is_not_configured() {
        assert!(matches!(
            test_pool_from_url(None).await,
            Err(SkipReason::NotConfigured)
        ));
        // CI's coverage job clears the URL with `DATABASE_URL: ""` for its
        // no-DB pass; that must keep skipping.
        assert!(matches!(
            test_pool_from_url(Some("")).await,
            Err(SkipReason::NotConfigured)
        ));
    }

    /// Port 1 on loopback has no listener. sqlx retries the refused
    /// connection until the 5 s acquire timeout, so this test takes ~5 s.
    #[tokio::test]
    async fn unreachable_url_is_connect_failed_and_names_the_target() {
        let result = test_pool_from_url(Some("postgres://nobody:s3cret-pw@127.0.0.1:1/sgw")).await;
        let Err(SkipReason::ConnectFailed(msg)) = result else {
            panic!("expected ConnectFailed, got {result:?}");
        };
        assert!(msg.contains("127.0.0.1:1/sgw"), "target missing: {msg}");
        assert!(!msg.contains("s3cret-pw"), "password leaked: {msg}");
    }

    #[test]
    fn not_configured_skips() {
        assert!(pool_or_skip(Err(SkipReason::NotConfigured), "t").is_none());
    }

    /// The #615 regression guard: reverting to "skip on any error" makes
    /// this return `None` instead of panicking.
    #[test]
    #[should_panic(expected = "DATABASE_URL is set, so the live-DB tests must run")]
    fn connect_failed_fails_the_test() {
        pool_or_skip(
            Err(SkipReason::ConnectFailed("connection refused".into())),
            "t",
        );
    }
}
