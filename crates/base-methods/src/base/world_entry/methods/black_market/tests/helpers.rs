//! Live-DB integration tests for the reusable persistence helpers:
//! [`crate::base::world_entry::methods::black_market::helpers::adjust_player_cash`], the escrow
//! moves in [`crate::base::world_entry::methods::black_market::escrow`], and the mail payout
//! writer [`crate::base::world_entry::methods::black_market::payout_mail::send_mail_to_player`].
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! These pin the error/edge branches the create/bid/cancel handlers depend on:
//! overdraw rejection, missing-player disambiguation, a listing of someone
//! else's row, the move into container 18 and back, the snapshot mint for a
//! listing with no row, and a full bag. Shared fixtures live in the parent
//! `tests` module.

use cimmeria_entity::inventory::{INV_AUCTION, INV_BANK, INV_MAIN};

use super::{
    cleanup, insert_account_and_player, insert_item, insert_item_in, inventory_count, item_state,
    ITEM_DEF_ID, TEST_BASE,
};
use crate::base::world_entry::methods::black_market::escrow::{
    deliver_from_escrow, list_into_escrow, DeliveryRefused,
};
use crate::base::world_entry::methods::black_market::helpers::{adjust_player_cash, CashError};
use crate::base::world_entry::methods::black_market::payout_mail::send_mail_to_player;
use crate::base::world_entry::methods::black_market::types::{auction_status, AuctionRow};
use crate::base::world_entry::methods::black_market::wire::BMError;
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

/// An auction row over `item_id` for `seller`, not inserted: the escrow
/// helpers only read its item fields.
fn auction_over(seller: i32, item_id: i32) -> AuctionRow {
    AuctionRow {
        sequence_id: 1,
        seller_id: seller,
        item_id,
        item_def_id: ITEM_DEF_ID,
        stack_size: 1,
        durability: 55,
        charges: 2,
        starting_price: 1,
        buyout_price: 0,
        current_bid: 0,
        current_bidder: None,
        auction_length: 5,
        created_at: 0,
        expires_at: 0,
        status: auction_status::ACTIVE,
    }
}

/// Listing someone else's row is refused and touches nothing. Bug shape: a
/// move keyed on `item_id` alone would take another player's item.
#[tokio::test]
async fn list_into_escrow_refuses_a_row_the_seller_does_not_own() {
    let pool = require_db_or_skip!();
    let (acc_owner, acc_thief) = (TEST_BASE + 750, TEST_BASE + 751);
    let (owner, thief) = (TEST_BASE + 760, TEST_BASE + 761);
    cleanup(&pool, &[acc_owner, acc_thief], &[owner, thief]).await;
    insert_account_and_player(&pool, acc_owner, owner, 0).await;
    insert_account_and_player(&pool, acc_thief, thief, 0).await;
    let item = insert_item(&pool, owner, ITEM_DEF_ID).await;

    let mut conn = pool.acquire().await.unwrap();
    let res = list_into_escrow(&mut conn, thief, item).await.unwrap();
    assert_eq!(res, Err(BMError::InvalidItem));
    drop(conn);
    assert_eq!(
        item_state(&pool, item).await.map(|s| (s.0, s.1)),
        Some((owner, INV_MAIN))
    );

    cleanup(&pool, &[acc_owner, acc_thief], &[owner, thief]).await;
}

/// The escrow move keeps the row: same instance id, same owner, now in
/// container 18, every column intact. Delivery puts that same row back in
/// a bag. Bug shape: the branch's DELETE-and-snapshot would lose the id.
#[tokio::test]
async fn escrow_moves_the_row_into_container_18_and_back() {
    let pool = require_db_or_skip!();
    let (account_id, seller) = (TEST_BASE + 770, TEST_BASE + 780);
    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;

    let mut conn = pool.acquire().await.unwrap();
    let listed = list_into_escrow(&mut conn, seller, item)
        .await
        .unwrap()
        .expect("owned row lists");
    assert_eq!((listed.item_id, listed.container_id), (item, INV_MAIN));
    assert_eq!(
        item_state(&pool, item).await,
        Some((seller, INV_AUCTION, 77, 3))
    );
    assert_eq!(
        inventory_count(&pool, seller).await,
        0,
        "gone from the bags"
    );

    let placed = deliver_from_escrow(&mut conn, &auction_over(seller, item), seller, false)
        .await
        .unwrap()
        .expect("room in the bag");
    assert_eq!(placed.item_id, item, "the same instance comes back");
    assert!(!placed.minted && !placed.overflow);
    assert_eq!(
        item_state(&pool, item).await,
        Some((seller, INV_MAIN, 77, 3))
    );
    drop(conn);

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// Only carried bags list: a banked or bound row is refused.
#[tokio::test]
async fn list_into_escrow_refuses_vault_and_bound_rows() {
    let pool = require_db_or_skip!();
    let (account_id, seller) = (TEST_BASE + 790, TEST_BASE + 800);
    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    let banked = insert_item_in(&pool, seller, ITEM_DEF_ID, INV_BANK, false).await;
    let bound = insert_item_in(&pool, seller, ITEM_DEF_ID, INV_MAIN, true).await;

    let mut conn = pool.acquire().await.unwrap();
    assert_eq!(
        list_into_escrow(&mut conn, seller, banked).await.unwrap(),
        Err(BMError::InvalidItem)
    );
    assert_eq!(
        list_into_escrow(&mut conn, seller, bound).await.unwrap(),
        Err(BMError::ItemBound)
    );
    drop(conn);
    assert_eq!(item_state(&pool, banked).await.map(|s| s.1), Some(INV_BANK));
    assert_eq!(item_state(&pool, bound).await.map(|s| s.1), Some(INV_MAIN));

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// A listing with no escrow row (the boot seed) is delivered as a new
/// instance from the snapshot; a full carried bag returns `None` unless
/// overflow is allowed.
#[tokio::test]
async fn delivery_mints_without_a_row_and_respects_full_bags() {
    let pool = require_db_or_skip!();
    let (account_id, player) = (TEST_BASE + 810, TEST_BASE + 820);
    cleanup(&pool, &[account_id], &[player]).await;
    insert_account_and_player(&pool, account_id, player, 0).await;

    let mut conn = pool.acquire().await.unwrap();
    let placed = deliver_from_escrow(&mut conn, &auction_over(player, 0), player, false)
        .await
        .unwrap()
        .expect("room");
    assert!(placed.minted);
    assert_eq!(
        item_state(&pool, placed.item_id).await,
        Some((player, INV_MAIN, 55, 2))
    );

    // Fill both carried bags.
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id,                                     bound, durability, charges)          SELECT $1, $2, 1, s, b, false, 1, 0          FROM (VALUES (1, 40), (15, 100)) AS bags(b, n), generate_series(0, 99) AS s          WHERE s < n            AND NOT EXISTS (SELECT 1 FROM sgw_inventory                            WHERE character_id = $1 AND container_id = b AND slot_id = s)",
    )
    .bind(player)
    .bind(ITEM_DEF_ID)
    .execute(&mut *conn)
    .await
    .unwrap();
    let none = deliver_from_escrow(&mut conn, &auction_over(player, 0), player, false)
        .await
        .unwrap();
    assert_eq!(
        none,
        Err(DeliveryRefused::BagFull),
        "full bags refuse a delivery"
    );
    let over = deliver_from_escrow(&mut conn, &auction_over(player, 0), player, true)
        .await
        .unwrap()
        .expect("overflow places");
    assert!(over.overflow);
    assert!(over.slot_id >= 40, "past the main bag's last slot");
    drop(conn);

    cleanup(&pool, &[account_id], &[player]).await;
}

/// `send_mail_to_player` persists the cash and sender of a settlement mail.
#[tokio::test]
async fn send_mail_persists_cash_and_sender() {
    let pool = require_db_or_skip!();
    let (account_id, player) = (TEST_BASE + 830, TEST_BASE + 840);
    cleanup(&pool, &[account_id], &[player]).await;
    insert_account_and_player(&pool, account_id, player, 0).await;

    let mail_id = send_mail_to_player(
        &pool,
        player,
        555,
        None,
        0,
        "Subject",
        "Body",
        "Black Market",
    )
    .await
    .expect("mail insert returns a mail_id");
    let (cash, sender): (i64, Option<String>) =
        sqlx::query_as("SELECT cash, sender_name FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cash, 555);
    assert_eq!(sender.as_deref(), Some("Black Market"));

    cleanup(&pool, &[account_id], &[player]).await;
}

/// A player's listing whose container-18 row is gone is refused, never
/// minted: otherwise anything that ever removed an escrowed row would let
/// the sale or cancel hand out a second copy (authority review, BM-02).
#[tokio::test]
async fn delivery_refuses_a_player_listing_with_no_escrow_row() {
    let pool = require_db_or_skip!();
    let (account_id, seller) = (TEST_BASE + 850, TEST_BASE + 860);
    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;
    let mut conn = pool.acquire().await.unwrap();
    list_into_escrow(&mut conn, seller, item)
        .await
        .unwrap()
        .unwrap();
    sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
        .bind(item)
        .execute(&mut *conn)
        .await
        .unwrap();

    let res = deliver_from_escrow(&mut conn, &auction_over(seller, item), seller, true)
        .await
        .unwrap();
    assert_eq!(res, Err(DeliveryRefused::EscrowMissing));
    drop(conn);
    assert_eq!(inventory_count(&pool, seller).await, 0, "no copy was made");

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// A refund that would pass the `integer` maximum is a named
/// `BalanceOverflow`, not a Postgres overflow error that fails every later
/// bid on the auction (authority review, BM-02).
#[tokio::test]
async fn adjust_player_cash_refuses_a_credit_past_the_integer_maximum() {
    let pool = require_db_or_skip!();
    let (account_id, player) = (TEST_BASE + 870, TEST_BASE + 880);
    cleanup(&pool, &[account_id], &[player]).await;
    insert_account_and_player(&pool, account_id, player, i32::MAX - 5).await;

    let mut conn = pool.acquire().await.unwrap();
    let err = adjust_player_cash(&mut conn, player, 10)
        .await
        .expect_err("overflow must be refused");
    assert_eq!(err, CashError::BalanceOverflow);
    assert_eq!(
        adjust_player_cash(&mut conn, player, 5).await,
        Ok(i64::from(i32::MAX))
    );
    drop(conn);

    cleanup(&pool, &[account_id], &[player]).await;
}
