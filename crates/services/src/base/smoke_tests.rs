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

/// Strip the psql-only `\set ON_ERROR_STOP on` directive at the top
/// of the smoke scripts. Everything else in each file is real SQL
/// that sqlx executes verbatim. sqlx already errors on the first
/// failed statement, so the directive is redundant under the cargo
/// harness.
fn strip_psql_metacommands(sql: &str) -> String {
    sql.lines()
        .filter(|line| !line.trim_start().starts_with("\\set"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `tools/vendor_store_smoke.sql` source, embedded so the test is
/// portable to environments where the repo `tools/` directory isn't
/// at a known path.
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
    let sql = strip_psql_metacommands(VENDOR_STORE_SMOKE_SQL);
    sqlx::raw_sql(&sql)
        .execute(&pool)
        .await
        .expect("vendor_store_smoke.sql must run without raising an exception");
}

/// `tools/inventory_move_smoke.sql` source.
const INVENTORY_MOVE_SMOKE_SQL: &str = include_str!("../../../../tools/inventory_move_smoke.sql");

/// End-to-end smoke for `handle_move_inventory_item`'s SQL primitives:
/// opens a temporary player with a stack and a single in INV_MAIN,
/// then chains split → simple-move → three-step swap and asserts
/// inventory state after each stage.
///
/// The three-step swap stage exercises the same parking-at-sentinel
/// procedure the move handler uses to dodge the `(character_id,
/// container_id, slot_id)` unique index mid-swap. A regression that
/// drops Step C (move parked source into target's slot) leaves the
/// source row at the sentinel slot — the script asserts that the
/// sentinel is empty at the end specifically to catch that shape.
///
/// Also pins:
/// - the stack-conservation invariant after split + swap (totals
///   match initial + 0 across both row count and summed stack_size),
/// - the unique-(container, slot) invariant via an explicit
///   GROUP BY HAVING duplicate-row check.
#[tokio::test]
async fn inventory_move_smoke_passes_against_seed_data() {
    let pool = require_db_or_skip!();
    let sql = strip_psql_metacommands(INVENTORY_MOVE_SMOKE_SQL);
    sqlx::raw_sql(&sql)
        .execute(&pool)
        .await
        .expect("inventory_move_smoke.sql must run without raising an exception");
}

/// `tools/progression_smoke.sql` source.
const PROGRESSION_SMOKE_SQL: &str = include_str!("../../../../tools/progression_smoke.sql");

/// End-to-end smoke for the multi-character isolation guarantees of
/// `handle_grant_cash` and `handle_grant_xp`. Both functions
/// `UPDATE sgw_player ... WHERE player_id = $N`; a refactor that
/// swaps `player_id` for `account_id` on either side would credit
/// every character on the same account.
///
/// The script seeds two characters under one account, applies a
/// cash grant to A and asserts B's naquadah is unchanged, applies
/// an XP/level/training-points grant to A and asserts B's
/// progression state is unchanged, then applies a cash grant to B
/// and asserts A's prior credit isn't disturbed. Catches the bug
/// shape regardless of which player_id the buggy WHERE happens to
/// resolve.
#[tokio::test]
async fn progression_smoke_passes_against_seed_data() {
    let pool = require_db_or_skip!();
    let sql = strip_psql_metacommands(PROGRESSION_SMOKE_SQL);
    sqlx::raw_sql(&sql)
        .execute(&pool)
        .await
        .expect("progression_smoke.sql must run without raising an exception");
}
