//! Live-DB regression guard for the load → mutate → save → reload
//! round-trip. Self-skips when `DATABASE_URL` is unset via
//! `require_db_or_skip!`.

use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for the crafting persistence tests, in the crafting
/// campaign's `0x7000_Cxxx` block. The tests use `TEST_BASE ..
/// TEST_BASE + 0xB55`; `handlers.rs` starts at `0x7000_CC00`. (It was
/// `0x7000_2000`, which `engine_loader.rs` also uses; issue #800.)
const TEST_BASE: i32 = 0x7000_C000;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    // Expertise rows cascade on sgw_player delete, but we delete
    // explicitly to support partial-cleanup paths if a test bails
    // before inserting the player row.
    let _ = sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_minimal_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("craft-test-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("craft-test-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// Round-trip regression guard: load defaults, mutate every field,
/// save, reload, and check every field matches. Bug shape this
/// catches: a forgotten field in the UPDATE statement (or in the
/// SELECT), a column rename without updating both queries, or a
/// transaction rollback that leaves the DB in a half-saved state.
#[tokio::test]
async fn round_trip_persists_all_fields() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    // 1. Load the freshly-inserted player — should be all defaults.
    let mut state = load_crafting_state(&pool, player_id)
        .await
        .expect("initial load");
    assert!(
        state.discipline_ids.is_empty(),
        "fresh player has no disciplines",
    );
    assert!(state.expertise.is_empty(), "fresh player has no expertise");
    assert!(
        state.blueprint_ids.is_empty(),
        "fresh player has no blueprints",
    );
    assert_eq!(state.applied_science_points, 0, "fresh player has 0 ASP",);

    // 2. Mutate every field. discipline 7 with expertise 42,
    // discipline 13 with expertise 100 (the cap — verifies the
    // CHECK constraint doesn't reject).
    state.discipline_ids = vec![7, 13];
    state.expertise.insert(7, 42);
    state.expertise.insert(13, 100);
    state.blueprint_ids = vec![200, 201, 202];
    state.applied_science_points = 5;
    state.racial_paradigm_levels.insert(1, 3);
    state.racial_paradigm_levels.insert(2, 1);
    state.racial_paradigm_levels.insert(3, 5);

    // 3. Save.
    save_crafting_state(&pool, player_id, &state)
        .await
        .expect("save");

    // 4. Reload from the DB — must round-trip exactly.
    let reloaded = load_crafting_state(&pool, player_id).await.expect("reload");

    assert_eq!(reloaded.discipline_ids, vec![7, 13]);
    assert_eq!(reloaded.blueprint_ids, vec![200, 201, 202]);
    assert_eq!(reloaded.applied_science_points, 5);
    assert_eq!(reloaded.get_expertise(7), Some(42));
    assert_eq!(reloaded.get_expertise(13), Some(100));
    assert_eq!(reloaded.racial_paradigm_levels.get(&1), Some(&3));
    assert_eq!(reloaded.racial_paradigm_levels.get(&2), Some(&1));
    assert_eq!(reloaded.racial_paradigm_levels.get(&3), Some(&5));

    cleanup(&pool, account_id, player_id).await;
}

/// Save can rewrite expertise — the second save with reduced
/// expertise rows must drop the old rows, not retain them. Bug
/// shape: an upsert without a prior DELETE would leave stale
/// rows that the next load picks up, presenting the player with
/// disciplines they don't actually know.
#[tokio::test]
async fn save_replaces_expertise_rows() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 10;
    let player_id = TEST_BASE + 11;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    // First save: 3 disciplines.
    let mut state = CraftingState::new();
    state.discipline_ids = vec![1, 2, 3];
    state.expertise.insert(1, 10);
    state.expertise.insert(2, 20);
    state.expertise.insert(3, 30);
    save_crafting_state(&pool, player_id, &state)
        .await
        .expect("first save");

    // Second save: drop discipline 2 entirely.
    state.discipline_ids = vec![1, 3];
    state.expertise.remove(&2);
    save_crafting_state(&pool, player_id, &state)
        .await
        .expect("second save");

    let reloaded = load_crafting_state(&pool, player_id).await.expect("reload");
    assert_eq!(reloaded.discipline_ids, vec![1, 3]);
    assert_eq!(
        reloaded.get_expertise(2),
        None,
        "expertise row for dropped discipline 2 must NOT survive a re-save \
         — a stale row here means the DELETE-then-INSERT pattern regressed \
         to a plain INSERT",
    );
    assert_eq!(reloaded.get_expertise(1), Some(10));
    assert_eq!(reloaded.get_expertise(3), Some(30));

    cleanup(&pool, account_id, player_id).await;
}

/// Load on a non-existent player_id returns the default state, not
/// an error. Matches the offline-mode sentinel pattern used by
/// `query_player_load_data`. Bug shape: a refactor that changed
/// `fetch_optional` to `fetch_one` would surface a hard error here
/// and break login for any account that didn't yet have crafting
/// state seeded.
#[tokio::test]
async fn load_missing_player_returns_default() {
    let pool = require_db_or_skip!();
    let bogus_player_id = TEST_BASE + 999;
    // Don't insert anything — just try to load.
    let state = load_crafting_state(&pool, bogus_player_id)
        .await
        .expect("load missing player should not error");
    assert!(state.discipline_ids.is_empty());
    assert!(state.expertise.is_empty());
    assert_eq!(state.applied_science_points, 0);
}

/// Regression guard for the silent-data-loss fix on save: writing a
/// `CraftingState` against a player_id that doesn't exist must
/// return an error rather than commit a no-op transaction.
///
/// Bug shape this catches: revert the `rows_affected() == 0` check
/// in `save_crafting_state` and this test starts seeing Ok(())
/// instead of Err — the same way the production bug presented
/// before the fix (every "save crafting state for player N" call
/// for a stale or freshly-disconnected player would succeed
/// silently, throwing away the in-memory mutation).
///
/// We assert the error variant is `RowNotFound`; if a future
/// refactor chooses a different error idiom (custom error type,
/// anyhow context), update the assertion but keep the regression
/// guard's intent: a 0-row UPDATE must surface as a failure.
#[tokio::test]
async fn save_for_nonexistent_player_returns_error() {
    let pool = require_db_or_skip!();
    let bogus_player_id = TEST_BASE + 1999;

    // Build a non-empty state to make the bug shape realistic — the
    // caller's intent is "persist these mutations", and an empty
    // state would obscure whether anything was meant to be saved.
    let mut state = CraftingState::new();
    state.discipline_ids = vec![1, 2];
    state.expertise.insert(1, 50);
    state.expertise.insert(2, 75);
    state.applied_science_points = 3;

    // No prior insert — player_id doesn't exist. Save must fail.
    let result = save_crafting_state(&pool, bogus_player_id, &state).await;
    match result {
        Err(sqlx::Error::RowNotFound) => {} // expected
        Err(other) => panic!(
            "save_crafting_state must return RowNotFound for a missing \
             player; got a different sqlx::Error variant: {other:?}"
        ),
        Ok(()) => panic!(
            "save_crafting_state silently succeeded for a non-existent \
             player_id ({bogus_player_id}). The rows_affected() == 0 \
             guard in save_crafting_state has regressed — without it, \
             callers think they persisted state but no row exists."
        ),
    }

    // Belt-and-suspenders: nothing should have been written to either
    // table. Verifies the txn rolled back cleanly (not that we
    // half-committed expertise rows for a phantom player).
    let expertise_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_player_discipline_expertise WHERE player_id = $1",
    )
    .bind(bogus_player_id)
    .fetch_one(&pool)
    .await
    .expect("count expertise rows");
    assert_eq!(
        expertise_count, 0,
        "transaction must roll back — no expertise rows should exist \
         for a player_id whose sgw_player row was never created"
    );
}

/// Documents behavior when an expertise row exists for a discipline
/// NOT in the player's `discipline_ids` array — i.e., a "stray"
/// row that out-of-band drift or partial deletes could leave behind.
///
/// **Current contract (this test pins it):** `load_crafting_state`
/// loads every expertise row for the player regardless of whether
/// the discipline is in `discipline_ids`. Rationale: the
/// `(player_id, discipline_id)` PK already scopes the SELECT to
/// this player; filtering by `discipline_ids` would mask the drift
/// and silently drop the row. Surfacing it (it appears in
/// `state.expertise` even though `state.discipline_ids` excludes
/// it) lets a future operator-side check catch the inconsistency.
///
/// If we ever decide to filter the stray rows out of the load, this
/// test must change *deliberately* — flip the expectation and
/// document the new contract in the persistence module's docs.
#[tokio::test]
async fn load_with_stray_expertise_row_keeps_it() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 2900;
    let player_id = TEST_BASE + 2901;
    cleanup(&pool, account_id, player_id).await;
    insert_minimal_player(&pool, account_id, player_id).await;

    // Set the player's known disciplines to [7]. Then insert an
    // expertise row for discipline 99, which is NOT in the array —
    // simulating drift (e.g., a respec that removed 99 from
    // discipline_ids but left the expertise row behind, or a
    // partial migration). The FK to sgw_player is satisfied
    // (player_id exists); only the application-level invariant
    // "expertise discipline_id ∈ discipline_ids" is violated.
    sqlx::query("UPDATE sgw_player SET discipline_ids = $1 WHERE player_id = $2")
        .bind(vec![7i32])
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("seed discipline_ids");

    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise \
            (player_id, discipline_id, expertise) \
         VALUES ($1, $2, $3)",
    )
    .bind(player_id)
    .bind(99i32)
    .bind(50i32)
    .execute(&pool)
    .await
    .expect("seed stray expertise row");

    let state = load_crafting_state(&pool, player_id)
        .await
        .expect("load with stray expertise row should not error");

    assert_eq!(
        state.discipline_ids,
        vec![7],
        "discipline_ids reloads the sgw_player array verbatim"
    );
    assert_eq!(
        state.get_expertise(99),
        Some(50),
        "stray expertise row (discipline 99 not in discipline_ids) \
         is loaded into state.expertise — the load does NOT filter \
         by discipline_ids. If this contract changes, update the \
         load_crafting_state docs alongside the test flip."
    );
    // The known discipline 7 has no expertise row yet — get_expertise
    // returns None for that, which is the existing happy-path shape
    // covered by other tests.
    assert_eq!(state.get_expertise(7), None);

    cleanup(&pool, account_id, player_id).await;
}
