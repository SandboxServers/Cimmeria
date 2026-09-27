//! Live-DB guards for D-BM09: deleting a character is never blocked by the
//! Black Market, and nobody else loses money when it happens.
//!
//! - `sgw_auction.seller_id` is ON DELETE CASCADE and `current_bidder` ON
//!   DELETE SET NULL (they were RESTRICT, which blocked every character that
//!   had ever listed or won, because settled rows are kept).
//! - The BEFORE DELETE trigger `bm_player_before_delete()` refunds the
//!   standing bidders of the deleted seller's open auctions, and reopens the
//!   open auctions the deleted character was winning.
//!
//! Sentinel range: TEST_BASE + 0x800 … +0x8FF.

use sqlx::PgPool;

use super::{
    cleanup, insert_account_and_player, insert_item, last_auction_of, naquadah_of, Harness,
    Session, ITEM_DEF_ID, TEST_BASE,
};
use crate::base::black_market::types::auction_status;
use crate::test_support::require_db_or_skip;

const BASE: i32 = TEST_BASE + 0x800;

async fn delete_player(pool: &PgPool, player_id: i32) {
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await
        .expect("the Black Market must not block a character delete");
}

async fn auction_count(pool: &PgPool, seq: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A seller with an open, bid-on listing and a settled one is deleted: the
/// delete goes through, both listings go with the character, and the
/// standing bidder gets their held cash back. Bug shapes: RESTRICT fails the
/// delete; a cascade without the trigger keeps the bidder's 300.
#[tokio::test]
async fn deleting_a_seller_refunds_the_standing_bidder() {
    let pool = require_db_or_skip!();
    let seller: Session = (0x7000_AAA1, BASE, BASE + 1);
    let bidder: Session = (0x7000_AAA2, BASE + 2, BASE + 3);
    cleanup(&pool, &[seller.1, bidder.1], &[seller.2, bidder.2]).await;
    insert_account_and_player(&pool, seller.1, seller.2, 0).await;
    insert_account_and_player(&pool, bidder.1, bidder.2, 10_000).await;
    let h = Harness::new(&pool, &[seller, bidder]);
    for _ in 0..2 {
        let item = insert_item(&pool, seller.2, ITEM_DEF_ID).await;
        h.create(seller, item, 100, 0, 5).await;
    }
    let open = last_auction_of(&pool, seller.2).await;
    let settled = open - 1;
    sqlx::query("UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2")
        .bind(auction_status::EXPIRED)
        .bind(settled)
        .execute(&pool)
        .await
        .unwrap();
    h.bid(bidder, open, 300).await;
    assert_eq!(naquadah_of(&pool, bidder.2).await, 10_000 - 300);

    delete_player(&pool, seller.2).await;

    assert_eq!(
        naquadah_of(&pool, bidder.2).await,
        10_000,
        "bidder refunded"
    );
    assert_eq!(auction_count(&pool, open).await, 0);
    assert_eq!(auction_count(&pool, settled).await, 0);

    cleanup(&pool, &[seller.1, bidder.1], &[seller.2, bidder.2]).await;
}

/// The standing bidder is deleted: the delete goes through, the open auction
/// reopens with no bid (no phantom bid left for the sweep), and a settled
/// auction they won keeps its row with no buyer.
#[tokio::test]
async fn deleting_a_bidder_reopens_the_auction() {
    let pool = require_db_or_skip!();
    let seller: Session = (0x7000_AAB1, BASE + 0x10, BASE + 0x11);
    let bidder: Session = (0x7000_AAB2, BASE + 0x12, BASE + 0x13);
    cleanup(&pool, &[seller.1, bidder.1], &[seller.2, bidder.2]).await;
    insert_account_and_player(&pool, seller.1, seller.2, 0).await;
    insert_account_and_player(&pool, bidder.1, bidder.2, 10_000).await;
    let h = Harness::new(&pool, &[seller, bidder]);
    for _ in 0..2 {
        let item = insert_item(&pool, seller.2, ITEM_DEF_ID).await;
        h.create(seller, item, 100, 0, 5).await;
    }
    let open = last_auction_of(&pool, seller.2).await;
    let won = open - 1;
    h.bid(bidder, open, 200).await;
    h.bid(bidder, won, 200).await;
    sqlx::query("UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2")
        .bind(auction_status::SOLD)
        .bind(won)
        .execute(&pool)
        .await
        .unwrap();

    delete_player(&pool, bidder.2).await;

    let row = |seq| {
        sqlx::query_as::<_, (i32, Option<i32>, i16)>(
            "SELECT current_bid, current_bidder, status FROM sgw_auction WHERE sequence_id = $1",
        )
        .bind(seq)
        .fetch_one(&pool)
    };
    assert_eq!(row(open).await.unwrap(), (0, None, auction_status::ACTIVE));
    let (bid, buyer, status) = row(won).await.unwrap();
    assert_eq!(
        (buyer, status),
        (None, auction_status::SOLD),
        "history kept"
    );
    assert_eq!(bid, 200);

    cleanup(&pool, &[seller.1, bidder.1], &[seller.2, bidder.2]).await;
}
