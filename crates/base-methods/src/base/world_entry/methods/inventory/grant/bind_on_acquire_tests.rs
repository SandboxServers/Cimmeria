//! Live-DB regression guards for issue #914 (BIND_ON_ACQUIRE): `persist.rs`
//! sets a new instance's `bound` column from `resources.items.flags & 4`
//! instead of hardcoding `false`, and a bound grant skips the stack-merge
//! fast path so two bound grants of the same design never launder into one
//! shared stack. A non-bound grant is unaffected: two grants of the same
//! stackable design still merge into one row, same as before.
//!
//! Sentinels: accounts/players `0x7000_C440..=0x7000_C445`, entities
//! `0x7000_C4EE..=0x7000_C4EF`, synthetic item types `0x7000_C4F5`
//! (BIND_ON_ACQUIRE, stackable) and `0x7000_C4F6` (not bound, stackable).

use super::fall_through_tests::{bags, cleanup, insert_account_and_player, state};
use super::*;
use crate::test_support::require_db_or_skip;

const BIND_TYPE_ID: i32 = 0x7000_C4F5;
const UNBOUND_TYPE_ID: i32 = 0x7000_C4F6;
/// `resources.items.flags` bit for BIND_ON_ACQUIRE
/// (`cimmeria_cell_catalog::crafting::ItemFlags::BIND_ON_ACQUIRE`);
/// duplicated as a plain literal so this test needs no extra dependency.
const BIND_ON_ACQUIRE: i32 = 4;

async fn insert_stackable_type(pool: &PgPool, item_id: i32, flags: i32) {
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(item_id)
        .execute(pool)
        .await;
    sqlx::query(
        "INSERT INTO resources.items \
            (item_id, description, name, quality_id, tech_comp, tier, \
             max_stack_size, container_sets, flags) \
         VALUES ($1, '', 'SS-914 fixture', 'ITEM_QUALITY_Normal', 0, 1, 10, '{1}', $2)",
    )
    .bind(item_id)
    .bind(flags)
    .execute(pool)
    .await
    .expect("insert item type");
}

async fn cleanup_type(pool: &PgPool, item_id: i32) {
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(item_id)
        .execute(pool)
        .await;
}

/// Two grants of a BIND_ON_ACQUIRE, stackable design land as two separate
/// bound rows — the merge fast path never runs for a bound grant, so a
/// bound quantity can never be laundered into (or conflated with) another
/// bound or unbound row of the same type. Reverting the SS-914 fix (the
/// `is_bound` gate in `persist.rs`) makes this fail two ways: the rows
/// merge into one (`bags` sums to one row of stack 2), and `bound` reads
/// `false`.
#[tokio::test]
async fn live_db_bind_on_acquire_grant_lands_bound_and_skips_merge() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C440, 0x7000_C441, 0x7000_C4EE_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_stackable_type(&pool, BIND_TYPE_ID, BIND_ON_ACQUIRE).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_item(
        entity_id,
        player_id,
        BIND_TYPE_ID,
        1,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    handle_grant_item(
        entity_id,
        player_id,
        BIND_TYPE_ID,
        1,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        bags(&pool, player_id).await,
        vec![(1, 2, 2)],
        "two bound grants must land as two separate rows (2 rows, stack 1 each), \
         never merged into one row of stack 2"
    );
    let bound_flags: Vec<bool> = sqlx::query_scalar(
        "SELECT bound FROM sgw_inventory WHERE character_id = $1 ORDER BY slot_id",
    )
    .bind(player_id)
    .fetch_all(&pool)
    .await
    .expect("read bound column");
    assert_eq!(bound_flags, vec![true, true], "both rows must be bound");

    cleanup(&pool, account_id, player_id, entity_id).await;
    cleanup_type(&pool, BIND_TYPE_ID).await;
}

/// A non-bound, stackable grant is unaffected by the SS-914 change: two
/// grants of the same design still merge into one unbound row. Guards
/// against an `is_bound` regression that accidentally disables merging for
/// every grant, bound or not.
#[tokio::test]
async fn live_db_non_bind_grant_still_merges_into_one_stack() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (0x7000_C442, 0x7000_C443, 0x7000_C4EF_u32);
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_stackable_type(&pool, UNBOUND_TYPE_ID, 0).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_item(
        entity_id,
        player_id,
        UNBOUND_TYPE_ID,
        1,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    handle_grant_item(
        entity_id,
        player_id,
        UNBOUND_TYPE_ID,
        1,
        1,
        false,
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        bags(&pool, player_id).await,
        vec![(1, 1, 2)],
        "two non-bound grants of the same design must still merge into one row"
    );
    let bound: bool = sqlx::query_scalar("SELECT bound FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .expect("read bound column");
    assert!(!bound, "an unflagged design must land unbound");

    cleanup(&pool, account_id, player_id, entity_id).await;
    cleanup_type(&pool, UNBOUND_TYPE_ID).await;
}
