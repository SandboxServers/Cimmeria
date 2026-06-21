//! Live-DB integration tests for createAuction / placeBid / cancelAuction.
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Against the bundled local Postgres they exercise: createAuction (row insert
//! plus escrow), placeBid (current_bid update, prior-bidder refund, bid-history
//! row), cancelAuction (item return plus bidder refund).
//!
//! Shared fixtures (`cleanup`, `insert_account_and_player`, `insert_item`,
//! `naquadah_of`, `inventory_count`, `make_state`, `TEST_BASE`, `ITEM_DEF_ID`)
//! live in the parent `tests` module.

use std::sync::Arc;

use super::{
    cleanup, insert_account_and_player, insert_item, inventory_count, make_state, naquadah_of,
    ITEM_DEF_ID, TEST_BASE,
};
use crate::base::black_market::types::auction_status;
use crate::base::black_market::{bid, cancel, create};
use crate::test_support::require_db_or_skip;

// ── createAuction ─────────────────────────────────────────────────────────

/// createAuction inserts an active `sgw_auction` row and escrows the item out
/// of the seller's inventory (inventory count drops to 0).
#[tokio::test]
async fn create_auction_inserts_row_and_escrows_item() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0901;
    let account_id = TEST_BASE;
    let seller = TEST_BASE + 1;
    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    create::handle_create_auction(
        entity_id, seller, item, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1 AND status = $2")
            .bind(seller)
            .bind(auction_status::ACTIVE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "one active auction row must exist");
    assert_eq!(
        inventory_count(&pool, seller).await,
        0,
        "the item must be escrowed out of inventory"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

// ── placeBid ──────────────────────────────────────────────────────────────

/// placeBid updates current_bid/current_bidder, refunds the prior bidder, holds
/// the new bidder's cash, and inserts a bid-history row.
#[tokio::test]
async fn place_bid_updates_refunds_prior_and_records_bid() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0911;
    let acc_seller = TEST_BASE + 100;
    let acc_b1 = TEST_BASE + 101;
    let acc_b2 = TEST_BASE + 102;
    let seller = TEST_BASE + 110;
    let bidder1 = TEST_BASE + 111;
    let bidder2 = TEST_BASE + 112;
    cleanup(
        &pool,
        &[acc_seller, acc_b1, acc_b2],
        &[seller, bidder1, bidder2],
    )
    .await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_b1, bidder1, 10_000).await;
    insert_account_and_player(&pool, acc_b2, bidder2, 10_000).await;
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    create::handle_create_auction(
        entity_id, seller, item, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();

    // Bidder1 bids 200 (>= starting 100).
    bid::handle_place_bid(
        entity_id, bidder1, seq, 200, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    assert_eq!(
        naquadah_of(&pool, bidder1).await,
        10_000 - 200,
        "bidder1's cash must be held"
    );

    // Bidder2 outbids at 500. Prior bidder (bidder1) must be refunded their 200.
    bid::handle_place_bid(
        entity_id, bidder2, seq, 500, &db_pool, &transport, &conn, &e2a,
    )
    .await;

    let (cur_bid, cur_bidder): (i32, Option<i32>) = sqlx::query_as(
        "SELECT current_bid, current_bidder FROM sgw_auction WHERE sequence_id = $1",
    )
    .bind(seq)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cur_bid, 500, "current_bid advances to the new high bid");
    assert_eq!(
        cur_bidder,
        Some(bidder2),
        "current_bidder is the new high bidder"
    );

    assert_eq!(
        naquadah_of(&pool, bidder1).await,
        10_000,
        "prior bidder refunded in full"
    );
    assert_eq!(
        naquadah_of(&pool, bidder2).await,
        10_000 - 500,
        "new bidder's cash held"
    );

    let bid_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction_bid WHERE sequence_id = $1")
            .bind(seq)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(bid_rows, 2, "two bid-history rows recorded");

    cleanup(
        &pool,
        &[acc_seller, acc_b1, acc_b2],
        &[seller, bidder1, bidder2],
    )
    .await;
}

// ── cancelAuction ───────────────────────────────────────────────────────────

/// cancelAuction returns the escrowed item to the seller and refunds the
/// current bidder.
#[tokio::test]
async fn cancel_auction_returns_item_and_refunds_bidder() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0921;
    let acc_seller = TEST_BASE + 200;
    let acc_bidder = TEST_BASE + 201;
    let seller = TEST_BASE + 210;
    let bidder = TEST_BASE + 211;
    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_bidder, bidder, 10_000).await;
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    create::handle_create_auction(
        entity_id, seller, item, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();
    bid::handle_place_bid(
        entity_id, bidder, seq, 300, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    assert_eq!(naquadah_of(&pool, bidder).await, 10_000 - 300);
    assert_eq!(inventory_count(&pool, seller).await, 0, "item escrowed");

    cancel::handle_cancel_auction(entity_id, seller, seq, &db_pool, &transport, &conn, &e2a).await;

    let status: i16 = sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, auction_status::CANCELLED);
    assert_eq!(
        inventory_count(&pool, seller).await,
        1,
        "escrowed item returned to seller"
    );
    assert_eq!(
        naquadah_of(&pool, bidder).await,
        10_000,
        "bidder refunded on cancel"
    );

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
}
