//! Live-DB guard for `resources.resource_versions.snapshot`.
//!
//! `resources.resource_update_trigger` fires on every edit to a resource
//! table and stores `pg_current_snapshot()` in that column. The snapshot's
//! text lists every in-progress transaction id on the server, in any
//! database, so it grows with the number of concurrent writers. The column
//! was `varchar(100)`: with a dozen other transactions open, any insert into
//! `resources.items` failed with "value too long for type character
//! varying(100)". The parallel live-DB tier hit it; so would a busy server.
//!
//! Sentinel: item `0x7000_F0E0`.

use sqlx::postgres::PgPoolOptions;

use crate::test_support::{database_url, require_db_or_skip};

const ITEM: i32 = 0x7000_F0E0;
/// The old column width, which the snapshot text must exceed for the test
/// to reproduce the failure.
const OLD_WIDTH: usize = 100;
/// Upper bound on the transactions held open, well inside Postgres's
/// default `max_connections` of 100.
const MAX_OTHERS: u32 = 48;

#[tokio::test]
async fn live_db_a_resource_edit_succeeds_under_many_concurrent_transactions() {
    let pool = require_db_or_skip!();
    let others = PgPoolOptions::new()
        .max_connections(MAX_OTHERS)
        .connect(&database_url().expect("the gate resolved the URL"))
        .await
        .expect("connect the pool that holds the other transactions");

    // Open transactions that each hold a transaction id until every new
    // snapshot's text is longer than the old column. Fresh databases have
    // short ids, so the count adapts rather than being fixed.
    let mut held = Vec::new();
    let mut snapshot = String::new();
    while snapshot.len() <= OLD_WIDTH + 10 && held.len() < MAX_OTHERS as usize {
        let mut tx = others.begin().await.unwrap();
        sqlx::query("SELECT pg_current_xact_id()")
            .execute(&mut *tx)
            .await
            .unwrap();
        held.push(tx);
        // A snapshot lists only running ids below the newest completed
        // one, so complete a transaction after the held ones: on a live
        // server other writers do this all the time.
        sqlx::query("SELECT pg_current_xact_id()")
            .execute(&pool)
            .await
            .unwrap();
        snapshot = sqlx::query_scalar("SELECT pg_current_snapshot()::text")
            .fetch_one(&pool)
            .await
            .unwrap();
    }
    assert!(
        snapshot.len() > OLD_WIDTH,
        "setup must push the snapshot past {OLD_WIDTH} chars: {snapshot}"
    );

    let inserted = sqlx::query(
        "INSERT INTO resources.items (\
            item_id, description, name, quality_id, tech_comp, tier, \
            max_stack_size, container_sets \
         ) VALUES ($1, '', 'snapshot-width', 'ITEM_QUALITY_Normal', 0, 1, 1, '{1}') \
         ON CONFLICT (item_id) DO NOTHING",
    )
    .bind(ITEM)
    .execute(&pool)
    .await;
    drop(held); // rolls every held transaction back
    let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
        .bind(ITEM)
        .execute(&pool)
        .await;
    inserted.expect("a resource edit must not overflow resource_versions.snapshot");
}
