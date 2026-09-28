//! Lock order against the induction completion. A completion takes the
//! player-wide inventory key and changes expertise without locking the
//! `sgw_player` row, so every other expertise writer and reader takes that
//! key before the row. Each test holds the key in another transaction with
//! an uncommitted +1 on discipline 78, the shape of a completion in
//! flight, and checks that the writer waits and the +1 survives.

use std::time::Duration;

use cimmeria_cell_catalog::crafting::shared_crafting_catalog;
use sqlx::PgPool;

use super::handlers::grant_expertise_in_db;
use super::spend::spend_in_db;
use super::test_players::{cleanup, insert_player};
use crate::test_support::require_db_or_skip;

/// Sentinels in the crafting `0x7000_Cxxx` block: `0x7000_CFA0..0x7000_CFAF`.
const TEST_BASE: i32 = 0x7000_CFA0;

/// A player knowing 78 at expertise 50 with one ASP.
async fn player(pool: &PgPool, n: i32) -> (i32, i32) {
    let (account_id, player_id) = (TEST_BASE + 2 * n, TEST_BASE + 2 * n + 1);
    cleanup(pool, account_id, player_id).await;
    insert_player(pool, account_id, player_id).await;
    sqlx::query(
        "UPDATE sgw_player SET applied_science_points = 1, discipline_ids = '{78}' \
         WHERE player_id = $1",
    )
    .bind(player_id)
    .execute(pool)
    .await
    .expect("seed crafting columns");
    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, 78, 50)",
    )
    .bind(player_id)
    .execute(pool)
    .await
    .expect("seed expertise");
    (account_id, player_id)
}

/// Begin a transaction that holds the player-wide key and has raised 78 to
/// 51 without committing.
async fn completion_in_flight(
    pool: &PgPool,
    player_id: i32,
) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut holder = pool.begin().await.expect("begin");
    sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *holder)
        .await
        .expect("take the player-wide key");
    sqlx::query(
        "UPDATE sgw_player_discipline_expertise SET expertise = 51 \
         WHERE player_id = $1 AND discipline_id = 78",
    )
    .bind(player_id)
    .execute(&mut *holder)
    .await
    .expect("uncommitted +1");
    holder
}

async fn expertise(pool: &PgPool, player_id: i32, discipline_id: i32) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT expertise FROM sgw_player_discipline_expertise \
         WHERE player_id = $1 AND discipline_id = $2",
    )
    .bind(player_id)
    .bind(discipline_id)
    .fetch_optional(pool)
    .await
    .expect("read expertise")
}

/// A spend waits for an in-flight completion, then learns on the committed
/// state; the completion's +1 on the prerequisite survives.
#[tokio::test]
async fn spend_waits_for_a_completion_in_flight() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 0).await;
    let catalog = shared_crafting_catalog(&pool).await.expect("catalog");

    let holder = completion_in_flight(&pool, player_id).await;
    let (task_pool, task_catalog) = (pool.clone(), catalog.clone());
    let task =
        tokio::spawn(async move { spend_in_db(&task_pool, &task_catalog, player_id, 79).await });
    tokio::time::sleep(Duration::from_millis(300)).await;
    let waited = !task.is_finished();
    holder.commit().await.expect("commit");

    let outcome = task.await.expect("join").expect("no database error");
    let (e78, e79) = (
        expertise(&pool, player_id, 78).await,
        expertise(&pool, player_id, 79).await,
    );
    cleanup(&pool, account_id, player_id).await;
    assert!(waited, "the spend waits for the player-wide inventory key");
    assert!(outcome.is_ok(), "{outcome:?}");
    assert_eq!((e78, e79), (Some(51), Some(1)));
}

/// The lost-update guard: the GM expertise grant rewrites every expertise
/// row from its read. Without the key it reads 78 at 50 while the
/// completion is in flight and writes 50 back after the completion commits
/// 51.
#[tokio::test]
async fn gm_expertise_grant_keeps_a_completions_expertise() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = player(&pool, 1).await;

    let holder = completion_in_flight(&pool, player_id).await;
    let task_pool = pool.clone();
    let task =
        tokio::spawn(async move { grant_expertise_in_db(&task_pool, player_id, 79, 5).await });
    tokio::time::sleep(Duration::from_millis(300)).await;
    holder.commit().await.expect("commit");

    let granted = task.await.expect("join").expect("no database error");
    let (e78, e79) = (
        expertise(&pool, player_id, 78).await,
        expertise(&pool, player_id, 79).await,
    );
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(granted, 5);
    assert_eq!(e78, Some(51), "the completion's +1 survives the grant");
    assert_eq!(e79, Some(5));
}
