//! Live-DB guard: a starter kit that can't be placed rolls the whole
//! character back.
//!
//! Bug shape (adversarial review of PR #1218, finding 1): the `sgw_player`
//! INSERT committed on its own and each starter item was a separate
//! autocommit INSERT whose failure was logged and skipped. A kit that failed
//! to place left a character that exists for good without its pistol, and
//! the client was told the creation succeeded. Now the row and the kit are
//! one transaction and the client gets error code 3.

use std::net::SocketAddr;
use std::sync::Arc;

use super::live_db_tests::{
    build_create_character_payload, cleanup, insert_account, make_connected,
};
use super::seed_parity_live_db_tests::default_choices;
use super::*;
use crate::test_support::{require_db_or_skip, TestTransport};

/// Next sentinel after the seed-parity tests' `0x7000_1E00`/`0x7000_1E01`.
const ROLLBACK_ACCOUNT: i32 = 0x7000_1E02;

/// SGU Asgard Male: the char_def `live_db_tests` already creates.
const CHAR_DEF: i32 = 9;

/// An item no bag accepts (`container_sets = '{}'`), so placing it fails
/// with `no_valid_container`. Read from the seed rather than hard-coded.
async fn unplaceable_item(pool: &PgPool) -> i32 {
    sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE container_sets = '{}' ORDER BY item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the seed has an item with no container")
}

async fn remove_kit_row(pool: &PgPool, item_id: i32) {
    let _ = sqlx::query(
        "DELETE FROM resources.char_creation_items WHERE char_def_id = $1 AND item_id = $2",
    )
    .bind(CHAR_DEF)
    .bind(item_id)
    .execute(pool)
    .await;
}

/// **Regression guard.** A kit item that can't be placed leaves no character
/// row and no inventory rows behind. Reverting to the per-statement
/// autocommit writes (or skipping the failed item) leaves the `sgw_player`
/// row and fails `no character row`.
#[tokio::test]
async fn unplaceable_kit_rolls_the_character_back_live_db() {
    let pool = require_db_or_skip!();
    let item = unplaceable_item(&pool).await;
    cleanup(&pool, ROLLBACK_ACCOUNT).await;
    remove_kit_row(&pool, item).await;
    insert_account(&pool, ROLLBACK_ACCOUNT, 0).await;
    sqlx::query(
        "INSERT INTO resources.char_creation_items (char_def_id, item_id, stack_size) \
         VALUES ($1, $2, 1)",
    )
    .bind(CHAR_DEF)
    .bind(item)
    .execute(&pool)
    .await
    .expect("add the unplaceable kit row");

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55812".parse().unwrap();
    let connected = make_connected(addr, ROLLBACK_ACCOUNT as u32, 0);
    let db_pool = Some(Arc::new(pool.clone()));
    let choices = default_choices(&pool, CHAR_DEF).await;
    let payload = build_create_character_payload("Kit Rollback", "", CHAR_DEF, &choices, 0);

    let result = handle_create_character(
        &dyn_transport,
        addr,
        [0u8; 32],
        ROLLBACK_ACCOUNT as u32,
        &payload,
        &connected,
        &db_pool,
    )
    .await;

    // Undo the seed change before asserting, so a failure doesn't leak it.
    remove_kit_row(&pool, item).await;
    assert!(result.is_ok(), "the handler answers the client: {result:?}");

    let players: i64 = sqlx::query_scalar("SELECT count(*) FROM sgw_player WHERE account_id = $1")
        .bind(ROLLBACK_ACCOUNT)
        .fetch_one(&pool)
        .await
        .expect("count players");
    let items: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id \
         WHERE p.account_id = $1",
    )
    .bind(ROLLBACK_ACCOUNT)
    .fetch_one(&pool)
    .await
    .expect("count items");
    cleanup(&pool, ROLLBACK_ACCOUNT).await;

    assert_eq!(
        players, 0,
        "no character row: a kit that can't be placed must roll the character back"
    );
    assert_eq!(items, 0, "no orphan starter items");
}
