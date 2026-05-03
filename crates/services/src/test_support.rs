//! Shared helpers for live-DB tests.
//!
//! Tests that need a real PostgreSQL connection call [`test_pool`] and
//! self-skip when `DATABASE_URL` is unset. The unit-test suite stays
//! green on a fresh checkout; only `DATABASE_URL=postgres://… cargo
//! test` exercises the integration path.
//!
//! See `docs/architecture/integration-test-infra.md` for the rationale,
//! local-setup steps, and per-test data-isolation patterns.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Why a live-DB test couldn't run.
///
/// Distinguishes "no DATABASE_URL configured" (expected on a fresh
/// checkout — silent skip) from "DATABASE_URL set but unreachable"
/// (likely misconfiguration — surface the connection error so the
/// developer can fix it).
pub(crate) enum SkipReason {
    /// `DATABASE_URL` env var was unset or empty.
    NotConfigured,
    /// `DATABASE_URL` was set but `connect()` failed. The string
    /// captures sqlx's underlying error for operator triage.
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
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err(SkipReason::NotConfigured),
    };
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .map_err(|e| SkipReason::ConnectFailed(e.to_string()))
}

/// Convenience macro: skip a test with a reason-specific message if
/// no DB pool is available. Pairs with [`test_pool`] — same gate,
/// less ceremony at each call site.
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
        match $crate::test_support::test_pool().await {
            Ok(p) => p,
            Err(reason) => {
                eprintln!(
                    "{}: skipping live-DB test ({reason})",
                    module_path!(),
                );
                return;
            }
        }
    }};
}

pub(crate) use require_db_or_skip;
