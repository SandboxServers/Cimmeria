//! Live-DB integration tests for the expiry sweep (`settle_expired_once`).
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Covers: a sold auction (seller cash mail + buyer item mail + status SOLD),
//! an unsold auction (item returned to seller + status EXPIRED), multiple
//! expired auctions settled in one pass, and the phantom-bidder edge where a
//! row has `current_bidder` set but `current_bid = 0` — which must settle as
//! UNSOLD, not SOLD.
//!
//! Shared fixtures live in the parent `tests` module.

use std::sync::Arc;

use sqlx::PgPool;

use super::{
    cleanup, insert_account_and_player, insert_item, inventory_count, make_state, ITEM_DEF_ID,
    TEST_BASE,
};
use crate::base::black_market::types::auction_status;
use crate::base::black_market::{bid, create, sweep};
use crate::test_support::require_db_or_skip;

/// Force an auction's `expires_at` into the past.
async fn expire_now(pool: &PgPool, seq: i32) {
    sqlx::query("UPDATE sgw_auction SET expires_at = 1 WHERE sequence_id = $1")
        .bind(seq)
        .execute(pool)
        .await
        .unwrap();
}

/// Count `sgw_gate_mail` rows for a recipient that carry an item attachment.
async fn item_mail_count(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND item_id IS NOT NULL",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn status_of(pool: &PgPool, seq: i32) -> i16 {
    sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Sweep settles a sold auction: seller is mailed the cash, buyer is mailed the
/// item, status → SOLD. Bug-shape guard: BEFORE the sweep runs the auction is
/// still ACTIVE and no mail exists — so a disabled/no-op sweep fails the
/// post-sweep assertions.
#[tokio::test]
async fn sweep_settles_sold_auction_with_two_mails() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0931;
    let acc_seller = TEST_BASE + 300;
    let acc_bidder = TEST_BASE + 301;
    let seller = TEST_BASE + 310;
    let bidder = TEST_BASE + 311;
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
        entity_id, bidder, seq, 750, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    expire_now(&pool, seq).await;

    // Bug-shape pre-condition: nothing has settled yet.
    assert_eq!(
        status_of(&pool, seq).await,
        auction_status::ACTIVE,
        "active until swept"
    );

    let settled = sweep::settle_expired_once(&pool).await.unwrap();
    assert!(
        settled.iter().any(|s| s.sequence_id == seq && s.sold),
        "sweep must report this auction as sold"
    );

    assert_eq!(
        status_of(&pool, seq).await,
        auction_status::SOLD,
        "status → SOLD after sweep"
    );

    // Seller mailed cash (the winning bid).
    let seller_cash_mail: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND cash = 750",
    )
    .bind(seller)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(seller_cash_mail, 1, "seller mailed the winning cash");
    assert_eq!(
        item_mail_count(&pool, bidder).await,
        1,
        "buyer mailed the item"
    );

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
}

/// Sweep settles an unsold auction: the escrowed item is returned (mailed) to
/// the seller and status → EXPIRED.
#[tokio::test]
async fn sweep_settles_unsold_auction_returns_item() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0941;
    let account_id = TEST_BASE + 400;
    let seller = TEST_BASE + 410;
    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
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
    expire_now(&pool, seq).await;
    assert_eq!(
        inventory_count(&pool, seller).await,
        0,
        "escrowed pre-sweep"
    );

    let settled = sweep::settle_expired_once(&pool).await.unwrap();
    assert!(
        settled.iter().any(|s| s.sequence_id == seq && !s.sold),
        "sweep must report this auction as unsold"
    );

    assert_eq!(
        status_of(&pool, seq).await,
        auction_status::EXPIRED,
        "status → EXPIRED after sweep"
    );
    assert_eq!(
        item_mail_count(&pool, seller).await,
        1,
        "seller mailed the returned item"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// One sweep pass settles *every* due auction, not just the first. Two auctions
/// expire together — one sold, one unsold — and both must be settled in a single
/// `settle_expired_once` call with the correct per-auction outcome. Bug shape:
/// a `break`-instead-of-`continue` (or a single-row query) in the sweep loop
/// would settle only one and leave the other ACTIVE.
#[tokio::test]
async fn sweep_settles_multiple_expired_in_one_pass() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0951;
    let acc_seller = TEST_BASE + 500;
    let acc_bidder = TEST_BASE + 501;
    let seller = TEST_BASE + 510;
    let bidder = TEST_BASE + 511;
    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_bidder, bidder, 10_000).await;
    let item_sold = insert_item(&pool, seller, ITEM_DEF_ID).await;
    let item_unsold = insert_item(&pool, seller, ITEM_DEF_ID).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    // Auction A: gets a bid → should settle SOLD.
    create::handle_create_auction(
        entity_id, seller, item_sold, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    // Auction B: no bid → should settle EXPIRED.
    create::handle_create_auction(
        entity_id,
        seller,
        item_unsold,
        100,
        0,
        1,
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let seqs: Vec<i32> = sqlx::query_scalar(
        "SELECT sequence_id FROM sgw_auction WHERE seller_id = $1 ORDER BY sequence_id",
    )
    .bind(seller)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(seqs.len(), 2, "two auctions created");
    let (seq_sold, seq_unsold) = (seqs[0], seqs[1]);

    bid::handle_place_bid(
        entity_id, bidder, seq_sold, 400, &db_pool, &transport, &conn, &e2a,
    )
    .await;

    expire_now(&pool, seq_sold).await;
    expire_now(&pool, seq_unsold).await;

    let settled = sweep::settle_expired_once(&pool).await.unwrap();
    assert_eq!(settled.len(), 2, "exactly two auctions settled in one pass");
    assert!(
        settled.iter().any(|s| s.sequence_id == seq_sold && s.sold),
        "the bid auction settled SOLD"
    );
    assert!(
        settled
            .iter()
            .any(|s| s.sequence_id == seq_unsold && !s.sold),
        "the no-bid auction settled UNSOLD"
    );

    assert_eq!(status_of(&pool, seq_sold).await, auction_status::SOLD);
    assert_eq!(status_of(&pool, seq_unsold).await, auction_status::EXPIRED);

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
}

/// A row with `current_bidder` set but `current_bid = 0` must settle as UNSOLD.
/// The sweep computes `sold = current_bidder.is_some() && current_bid > 0`.
/// Bug shape: dropping the `&& current_bid > 0` conjunct would treat this
/// phantom bidder as a winner — mailing the seller cash=0 and the (non-paying)
/// bidder the item. We assert the seller gets the item back, the bidder gets
/// nothing, and the status is EXPIRED. The row is INSERTed directly so the
/// inconsistent state can be staged precisely.
#[tokio::test]
async fn sweep_phantom_bidder_zero_bid_settles_unsold() {
    let pool = require_db_or_skip!();
    let acc_seller = TEST_BASE + 600;
    let acc_bidder = TEST_BASE + 601;
    let seller = TEST_BASE + 610;
    let bidder = TEST_BASE + 611;
    let seq = TEST_BASE + 620;
    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    // Also clear any stray row at our hand-picked sequence_id.
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .execute(&pool)
        .await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_bidder, bidder, 10_000).await;

    // Stage an expired auction with a bidder but a zero bid (inconsistent).
    sqlx::query(
        "INSERT INTO sgw_auction \
            (sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
             charges, starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, $2, 0, $3, 1, 100, 0, 100, 0, 0, $4, 1, 1, 1, $5)",
    )
    .bind(seq)
    .bind(seller)
    .bind(ITEM_DEF_ID)
    .bind(bidder)
    .bind(auction_status::ACTIVE)
    .execute(&pool)
    .await
    .expect("insert phantom-bidder auction");

    let settled = sweep::settle_expired_once(&pool).await.unwrap();
    let this = settled
        .iter()
        .find(|s| s.sequence_id == seq)
        .expect("phantom-bidder auction must be settled");
    assert!(
        !this.sold,
        "zero current_bid must NOT count as sold even with a current_bidder"
    );
    assert_eq!(this.buyer_id, None, "no buyer recorded for a phantom bid");

    assert_eq!(
        status_of(&pool, seq).await,
        auction_status::EXPIRED,
        "phantom-bidder auction settles EXPIRED, not SOLD"
    );
    assert_eq!(
        item_mail_count(&pool, seller).await,
        1,
        "the item is returned to the seller"
    );
    assert_eq!(
        item_mail_count(&pool, bidder).await,
        0,
        "the phantom bidder receives no item"
    );

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .execute(&pool)
        .await;
}
