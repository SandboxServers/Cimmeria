//! Live-DB regression guards for the BM-02 rules: the expired window
//! (bid and cancel after `expires_at` are `AuctionGone`), the listing cap
//! (D5), the minimum increment (D6) and the immediate buyout (D8).
//!
//! Each test drives the real handler through [`Harness`] and asserts the
//! database afterwards, so the guard fails if the rule is removed from the
//! handler, not only from `validate.rs`.
//!
//! Sentinel range: TEST_BASE + 0x600 … +0x6FF.

use cimmeria_entity::inventory::{INV_AUCTION, INV_MAIN};
use sqlx::PgPool;

use super::{
    cleanup, expire_now, insert_account_and_player, insert_item, item_state, last_auction_of,
    naquadah_of, status_of, Harness, Session, ITEM_DEF_ID, TEST_BASE,
};
use crate::base::world_entry::methods::black_market::types::{auction_status, MAX_ACTIVE_LISTINGS};
use crate::test_support::require_db_or_skip;

const BASE: i32 = TEST_BASE + 0x600;

async fn current_bid(pool: &PgPool, seq: i32) -> (i32, Option<i32>) {
    sqlx::query_as("SELECT current_bid, current_bidder FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Seller, two bidders, all online; the seller lists one item (start 100,
/// `buyout`). Returns the sessions, the item and the auction.
async fn listed(pool: &PgPool, off: i32, buyout: i32) -> ([Session; 3], i32, i32, Harness) {
    let s: [Session; 3] = [
        (0x7000_A960 + off as u32, BASE + off, BASE + off + 1),
        (0x7000_A961 + off as u32, BASE + off + 2, BASE + off + 3),
        (0x7000_A962 + off as u32, BASE + off + 4, BASE + off + 5),
    ];
    cleanup(pool, &[s[0].1, s[1].1, s[2].1], &[s[0].2, s[1].2, s[2].2]).await;
    insert_account_and_player(pool, s[0].1, s[0].2, 0).await;
    insert_account_and_player(pool, s[1].1, s[1].2, 10_000).await;
    insert_account_and_player(pool, s[2].1, s[2].2, 10_000).await;
    let h = Harness::new(pool, &s);
    let item = insert_item(pool, s[0].2, ITEM_DEF_ID).await;
    h.create(s[0], item, 100, buyout, 5).await;
    let seq = last_auction_of(pool, s[0].2).await;
    (s, item, seq, h)
}

async fn done(pool: &PgPool, s: &[Session; 3]) {
    cleanup(pool, &[s[0].1, s[1].1, s[2].1], &[s[0].2, s[1].2, s[2].2]).await;
}

/// The expired-window guard: between `expires_at` and the next sweep pass
/// the row is still ACTIVE, but a bid is refused (no cash moves) and so is
/// the seller's cancel (the item stays in escrow for the sweep). Bug shape:
/// without the `expires_at > now` check both go through on a closed auction.
#[tokio::test]
async fn bid_and_cancel_after_expiry_are_refused() {
    let pool = require_db_or_skip!();
    let (s, item, seq, h) = listed(&pool, 0x00, 0).await;
    h.bid(s[1], seq, 200).await;
    expire_now(&pool, seq).await;

    h.bid(s[2], seq, 500).await;
    assert_eq!(
        current_bid(&pool, seq).await,
        (200, Some(s[1].2)),
        "bid refused"
    );
    assert_eq!(naquadah_of(&pool, s[2].2).await, 10_000, "nothing charged");

    h.cancel(s[0], seq).await;
    assert_eq!(
        status_of(&pool, seq).await,
        auction_status::ACTIVE,
        "left for the sweep"
    );
    assert_eq!(
        item_state(&pool, item).await.map(|i| i.1),
        Some(INV_AUCTION)
    );
    assert_eq!(
        naquadah_of(&pool, s[1].2).await,
        10_000 - 200,
        "bid still held"
    );

    done(&pool, &s).await;
}

/// D6: 5% over the standing bid, at least 1. 104 over 100 is refused,
/// 105 accepted.
#[tokio::test]
async fn bids_below_the_five_percent_increment_are_refused() {
    let pool = require_db_or_skip!();
    let (s, _item, seq, h) = listed(&pool, 0x10, 0).await;
    h.bid(s[1], seq, 100).await;
    h.bid(s[2], seq, 104).await;
    assert_eq!(current_bid(&pool, seq).await, (100, Some(s[1].2)));
    assert_eq!(naquadah_of(&pool, s[2].2).await, 10_000);
    h.bid(s[2], seq, 105).await;
    assert_eq!(current_bid(&pool, seq).await, (105, Some(s[2].2)));
    done(&pool, &s).await;
}

/// D5: a seller with 20 active listings cannot open a 21st, and the item
/// stays in the bag. Bug shape: without the cap the listing opens.
#[tokio::test]
async fn the_twenty_first_listing_is_refused() {
    let pool = require_db_or_skip!();
    let (s, _item, _seq, h) = listed(&pool, 0x20, 0).await;
    // One listed through the handler; fill the rest directly.
    sqlx::query(
        "INSERT INTO sgw_auction (seller_id, item_id, item_def_id, starting_price, \
                                  auction_length, created_at, expires_at, status) \
         SELECT $1, 0, $2, 10, 5, 0, 2000000000, 0 FROM generate_series(2, $3)",
    )
    .bind(s[0].2)
    .bind(ITEM_DEF_ID)
    .bind(MAX_ACTIVE_LISTINGS as i32)
    .execute(&pool)
    .await
    .unwrap();

    let extra = insert_item(&pool, s[0].2, ITEM_DEF_ID).await;
    h.create(s[0], extra, 100, 0, 5).await;
    let active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1 AND status = 0")
            .bind(s[0].2)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active, MAX_ACTIVE_LISTINGS);
    assert_eq!(item_state(&pool, extra).await.map(|i| i.1), Some(INV_MAIN));
    done(&pool, &s).await;
}

/// D8: a bid at or over the buyout price settles at once. The buyer pays
/// the buyout price (not their higher bid), the prior bidder is refunded,
/// the listed row moves to the buyer's bags, the seller is mailed the cash.
/// Bug shape: the branch left a buyout bid standing until expiry.
#[tokio::test]
async fn a_buyout_settles_immediately() {
    let pool = require_db_or_skip!();
    let (s, item, seq, h) = listed(&pool, 0x30, 1_000).await;
    h.bid(s[1], seq, 200).await;
    h.bid(s[2], seq, 5_000).await;

    assert_eq!(status_of(&pool, seq).await, auction_status::SOLD);
    assert_eq!(current_bid(&pool, seq).await, (1_000, Some(s[2].2)));
    assert_eq!(
        naquadah_of(&pool, s[2].2).await,
        10_000 - 1_000,
        "charged the buyout"
    );
    assert_eq!(
        naquadah_of(&pool, s[1].2).await,
        10_000,
        "outbid bidder refunded"
    );
    assert_eq!(
        item_state(&pool, item).await.map(|i| (i.0, i.1)),
        Some((s[2].2, INV_MAIN))
    );
    let paid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND cash = 1000",
    )
    .bind(s[0].2)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(paid, 1, "seller mailed the buyout");
    done(&pool, &s).await;
}

/// A buyout into full bags is refused before anything is charged.
#[tokio::test]
async fn a_buyout_into_full_bags_is_refused() {
    let pool = require_db_or_skip!();
    let (s, item, seq, h) = listed(&pool, 0x40, 1_000).await;
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
                                    bound, durability, charges) \
         SELECT $1, $2, 1, s, b, false, 1, 0 \
         FROM (VALUES (1, 40), (15, 100)) AS bags(b, n), generate_series(0, 99) AS s \
         WHERE s < n",
    )
    .bind(s[2].2)
    .bind(ITEM_DEF_ID)
    .execute(&pool)
    .await
    .unwrap();

    h.bid(s[2], seq, 1_000).await;
    assert_eq!(status_of(&pool, seq).await, auction_status::ACTIVE);
    assert_eq!(naquadah_of(&pool, s[2].2).await, 10_000);
    assert_eq!(
        item_state(&pool, item).await.map(|i| i.1),
        Some(INV_AUCTION)
    );
    done(&pool, &s).await;
}
