//! End-to-end live-DB smoke tests that exercise full multi-handler
//! flows against the seeded database.
//!
//! Unlike the per-handler tests under
//! `world_entry/methods/.../tests.rs`, these tests run a single
//! large PL/pgSQL block authored as a hand-written script in
//! `tools/`. The script asserts via `RAISE EXCEPTION`, which sqlx
//! surfaces as an error — so the only thing the Rust harness needs
//! to do is execute the script and assert `is_ok()`.
//!
//! Rollback semantics on pass vs fail:
//!
//! - **Pass:** the script's own `BEGIN ... ROLLBACK` wrapper leaves
//!   the DB byte-identical to the start.
//! - **Fail:** a PL/pgSQL `RAISE EXCEPTION` aborts the statement and
//!   Postgres marks the session's transaction as aborted (it does
//!   *not* issue an automatic `ROLLBACK` itself — further commands
//!   on the connection would fail with "current transaction is
//!   aborted" until an explicit `ROLLBACK`). sqlx surfaces the
//!   error to the harness, the test panics, and the pool issues
//!   `ROLLBACK` on connection release. Net effect: seed data
//!   unchanged regardless of pass or fail.

use crate::test_support::require_db_or_skip;

/// `tools/vendor_store_smoke.sql` source, embedded so the test is
/// portable to environments where the repo `tools/` directory isn't
/// at a known path. The leading `\set ON_ERROR_STOP on` is a
/// psql-only metacommand and is stripped at runtime — sqlx already
/// errors on the first failed statement.
const VENDOR_STORE_SMOKE_SQL: &str = include_str!("../../../../tools/vendor_store_smoke.sql");

/// End-to-end smoke for the vendor stack: opens a temporary
/// account+player, runs a sell → buyback → grant → purchase
/// sequence against the seeded vendors, and asserts every
/// intermediate state matches expectations. Rolls back at the end
/// so the seed data is unchanged.
///
/// The script self-contains every assertion via PL/pgSQL
/// `RAISE EXCEPTION`. sqlx surfaces those as `Err`, so the harness
/// only has to execute and `expect("ok")`.
///
/// Catches whole-stack regressions that per-handler tests miss —
/// e.g. if `handle_sell_vendor_items` and `handle_buyback_vendor_items`
/// stop agreeing on the `flags` column's role as the buyback unit
/// price, the per-handler tests still pass (each side is internally
/// consistent) but the smoke fails because the round-trip prices
/// don't match.
#[tokio::test]
async fn vendor_store_smoke_passes_against_seed_data() {
    let pool = require_db_or_skip!();

    // Strip the psql-only `\set ON_ERROR_STOP on` directive.
    // Everything else in the file is real SQL.
    let sql: String = VENDOR_STORE_SMOKE_SQL
        .lines()
        .filter(|line| !line.trim_start().starts_with("\\set"))
        .collect::<Vec<_>>()
        .join("\n");

    sqlx::raw_sql(&sql)
        .execute(&pool)
        .await
        .expect("vendor_store_smoke.sql must run without raising an exception");
}
