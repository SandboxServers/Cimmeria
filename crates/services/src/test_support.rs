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

/// Open a `PgPool` against the developer-supplied `DATABASE_URL`, or
/// return `None` if the variable is unset / connection fails.
///
/// Bounded to 4 connections — high enough for tests that exercise
/// concurrent paths (drainer + caller in parallel), low enough that
/// a careless test loop can't exhaust a hand-tuned local Postgres.
pub(crate) async fn test_pool() -> Option<PgPool> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return None,
    };
    match PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
    {
        Ok(pool) => Some(pool),
        Err(e) => {
            eprintln!("DATABASE_URL set but connect failed: {e}");
            None
        }
    }
}

/// Convenience macro: skip a test with a clear message if no DB is
/// available. Pairs with [`test_pool`] — same gate, less ceremony at
/// each call site.
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
            Some(p) => p,
            None => {
                eprintln!(
                    "{}: DATABASE_URL not set; skipping live-DB test",
                    module_path!()
                );
                return;
            }
        }
    }};
}

pub(crate) use require_db_or_skip;
