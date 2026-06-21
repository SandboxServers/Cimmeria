//! Live-DB integration tests for the reusable persistence helpers in
//! [`crate::base::black_market::helpers`]: `adjust_player_cash`,
//! `escrow_item`, `send_mail_to_player`, and `return_item`.
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! These pin the error/edge branches the create/bid/cancel handlers depend on:
//! overdraw rejection, missing-player disambiguation, not-owned escrow, and the
//! mail/return round-trip. Shared fixtures live in the parent `tests` module.

use super::{
    cleanup, insert_account_and_player, insert_item, inventory_count, ITEM_DEF_ID, TEST_BASE,
};
use crate::base::black_market::helpers::{
    adjust_player_cash, escrow_item, return_item, send_mail_to_player, CashError,
};
use crate::test_support::require_db_or_skip;

/// A debit larger than the balance is rejected with `InsufficientFunds`, and
/// the balance is left untouched (the guard is enforced in SQL atomically).
/// Bug shape: removing the `WHERE naquadah + $1 >= 0` guard would let the
/// balance go negative and return `Ok`.
#[tokio::test]
async fn adjust_player_cash_rejects_overdraw() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 700;
    let player = TEST_BASE + 710;
    cleanup(&pool, &[account_id], &[player]).await;
    insert_account_and_player(&pool, account_id, player, 100).await;

    let mut conn = pool.acquire().await.unwrap();
    let err = adjust_player_cash(&mut conn, player, -101)
        .await
        .expect_err("overdraw must be rejected");
    assert_eq!(err, CashError::InsufficientFunds);
    drop(conn);

    let bal: i64 =
        sqlx::query_scalar("SELECT naquadah::bigint FROM sgw_player WHERE player_id = $1")
            .bind(player)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(bal, 100, "rejected debit must not change the balance");

    cleanup(&pool, &[account_id], &[player]).await;
}

/// A debit that exactly clears the balance is accepted and returns the new
/// balance (0). This pins the boundary the overdraw guard allows (`>= 0`).
#[tokio::test]
async fn adjust_player_cash_allows_exact_zero() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 720;
    let player = TEST_BASE + 730;
    cleanup(&pool, &[account_id], &[player]).await;
    insert_account_and_player(&pool, account_id, player, 100).await;

    let mut conn = pool.acquire().await.unwrap();
    let new_bal = adjust_player_cash(&mut conn, player, -100)
        .await
        .expect("debit to exactly zero must be accepted");
    assert_eq!(new_bal, 0, "balance lands on zero, returned to caller");

    cleanup(&pool, &[account_id], &[player]).await;
}

/// Adjusting a non-existent player is `NoSuchPlayer`, distinct from
/// `InsufficientFunds`. Bug shape: the existence probe is what separates the
/// two — if it were dropped, a missing player would masquerade as an overdraw.
#[tokio::test]
async fn adjust_player_cash_missing_player_is_no_such_player() {
    let pool = require_db_or_skip!();
    let ghost = TEST_BASE + 740; // never inserted
                                 // Defensive: ensure the row really is absent.
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(ghost)
        .execute(&pool)
        .await;

    let mut conn = pool.acquire().await.unwrap();
    // Use a credit (+10) so the guard (`naquadah + delta >= 0`) cannot be the
    // reason for the miss — the only reason is the absent row.
    let err = adjust_player_cash(&mut conn, ghost, 10)
        .await
        .expect_err("missing player must error");
    assert_eq!(err, CashError::NoSuchPlayer);
}

/// `escrow_item` for an item the player does not own returns `Ok(None)` — the
/// DELETE matches no row. This is the ownership check `validate_create` relies
/// on. Bug shape: a query that ignored `character_id` could delete another
/// player's instance and return `Some`.
#[tokio::test]
async fn escrow_item_not_owned_returns_none() {
    let pool = require_db_or_skip!();
    let acc_owner = TEST_BASE + 750;
    let acc_thief = TEST_BASE + 751;
    let owner = TEST_BASE + 760;
    let thief = TEST_BASE + 761;
    cleanup(&pool, &[acc_owner, acc_thief], &[owner, thief]).await;
    insert_account_and_player(&pool, acc_owner, owner, 0).await;
    insert_account_and_player(&pool, acc_thief, thief, 0).await;
    let item = insert_item(&pool, owner, ITEM_DEF_ID).await;

    // Thief tries to escrow the owner's instance.
    let escrowed = escrow_item(&pool, thief, item)
        .await
        .expect("query runs cleanly");
    assert!(
        escrowed.is_none(),
        "escrow of an unowned item must match no row"
    );

    // The owner still has the item — it was not deleted.
    assert_eq!(
        inventory_count(&pool, owner).await,
        1,
        "owner's item must be untouched by a foreign escrow attempt"
    );

    cleanup(&pool, &[acc_owner, acc_thief], &[owner, thief]).await;
}

/// `escrow_item` for an owned item returns the auctionable snapshot and removes
/// the instance. Confirms the `Some` branch + the DELETE side effect.
#[tokio::test]
async fn escrow_item_owned_returns_snapshot_and_deletes() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 770;
    let owner = TEST_BASE + 780;
    cleanup(&pool, &[account_id], &[owner]).await;
    insert_account_and_player(&pool, account_id, owner, 0).await;
    let item = insert_item(&pool, owner, ITEM_DEF_ID).await;

    let snap = escrow_item(&pool, owner, item)
        .await
        .expect("query runs")
        .expect("owned item must escrow");
    assert_eq!(snap.item_id, item);
    assert_eq!(
        snap.item_def_id, ITEM_DEF_ID,
        "snapshot carries the type_id"
    );
    assert_eq!(snap.stack_size, 1);
    assert_eq!(
        inventory_count(&pool, owner).await,
        0,
        "escrow deletes the instance"
    );

    cleanup(&pool, &[account_id], &[owner]).await;
}

/// `send_mail_to_player` followed by `return_item` is the full unsold-return
/// round-trip the sweep uses. Pins that the mail row carries the cash + item
/// fields and that `return_item` lands a fresh instance at slot >= 0 (never the
/// `-1` swap sentinel called out in the helper doc).
#[tokio::test]
async fn send_mail_then_return_item_round_trip() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 790;
    let player = TEST_BASE + 800;
    cleanup(&pool, &[account_id], &[player]).await;
    insert_account_and_player(&pool, account_id, player, 0).await;

    // Return an item instance (def 21) to the empty inventory.
    let new_item = return_item(&pool, player, ITEM_DEF_ID, 1, 100, 0)
        .await
        .expect("return_item inserts an instance");
    assert_eq!(
        inventory_count(&pool, player).await,
        1,
        "returned item lands in inventory"
    );
    let slot: i32 = sqlx::query_scalar("SELECT slot_id FROM sgw_inventory WHERE item_id = $1")
        .bind(new_item)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(slot >= 0, "must never insert at the -1 swap sentinel slot");

    // Attach that instance to a mail with a cash payload.
    let mail_id = send_mail_to_player(
        &pool,
        player,
        555,
        Some(new_item),
        0,
        "Subject",
        "Body",
        "Black Market",
    )
    .await
    .expect("mail insert returns a mail_id");

    let (cash, item_id, sender): (i64, Option<i32>, Option<String>) =
        sqlx::query_as("SELECT cash, item_id, sender_name FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cash, 555, "cash payload persisted");
    assert_eq!(item_id, Some(new_item), "item attachment persisted");
    assert_eq!(
        sender.as_deref(),
        Some("Black Market"),
        "sender_name persisted"
    );

    cleanup(&pool, &[account_id], &[player]).await;
}
