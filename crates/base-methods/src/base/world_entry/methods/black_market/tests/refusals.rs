//! Negative-log tests for every base refusal seam (plan §5.1, BM-02): each
//! refusal logs `event = bm.refused` with `reason` equal to the
//! `BMError::reason` of the id the client is sent, `error_id`, and the
//! actor's `account_id` / `player_id`, and each sends one `onBMError`.
//!
//! The cell's own refusal (`not_at_auctioneer`, `watch_unavailable`) is
//! pinned in `cimmeria-cell-methods`' black_market tests.
//!
//! Sentinel range: TEST_BASE + 0x700 … +0x7FF.

use cimmeria_entity::inventory::INV_BANK;
use tracing::Level;

use super::{
    cleanup, expire_now, insert_account_and_player, insert_item, insert_item_in, last_auction_of,
    status_of, Harness, Session, ITEM_DEF_ID, TEST_BASE,
};
use crate::base::world_entry::methods::black_market::types::{auction_status, BMSearchOptions};
use crate::base::world_entry::methods::black_market::wire::BMError;
use crate::test_support::{require_db_or_skip, LogCapture, LogCaptureGuard};

const BASE: i32 = TEST_BASE + 0x700;

/// The refusal row for `error` exists, carries the id and the actor, and
/// one `onBMError` reached the wire for it.
fn assert_refused(capture: &LogCaptureGuard, error: BMError, actor: Session) {
    let ev = capture
        .find_event(Level::INFO, "Black Market request refused", error.reason())
        .unwrap_or_else(|| panic!("no refusal row for {error:?}: {:#?}", capture.all()));
    assert!(ev.has_field("error_id", &error.id().to_string()), "{ev:?}");
    assert!(ev.has_field("account_id", &actor.1.to_string()), "{ev:?}");
    assert!(ev.has_field("player_id", &actor.2.to_string()), "{ev:?}");
}

/// How many `onBMError` sends reached the wire.
fn errors_sent(capture: &LogCaptureGuard) -> usize {
    capture
        .all()
        .iter()
        .filter(|e| {
            e.message_contains("Black Market client send")
                && e.has_field("method", "onBMError")
                && e.has_field("sent", "true")
        })
        .count()
}

#[tokio::test]
async fn live_db_every_create_refusal_logs_its_reason_and_answers() {
    let pool = require_db_or_skip!();
    let seller: Session = (0x7000_AA71, BASE, BASE + 1);
    let other: Session = (0x7000_AA72, BASE + 2, BASE + 3);
    cleanup(&pool, &[seller.1, other.1], &[seller.2, other.2]).await;
    insert_account_and_player(&pool, seller.1, seller.2, 0).await;
    insert_account_and_player(&pool, other.1, other.2, 0).await;
    let h = Harness::new(&pool, &[seller, other]);
    let item = insert_item(&pool, seller.2, ITEM_DEF_ID).await;
    let banked = insert_item_in(&pool, seller.2, ITEM_DEF_ID, INV_BANK, false).await;
    let bound = insert_item_in(&pool, seller.2, ITEM_DEF_ID, 1, true).await;
    let not_mine = insert_item(&pool, other.2, ITEM_DEF_ID).await;

    let capture = LogCapture::install();
    h.create(seller, item, 0, 0, 5).await; // start below 1
    h.create(seller, item, 100, 50, 5).await; // buyout below start
    h.create(seller, banked, 100, 0, 5).await; // not a carried bag
    h.create(seller, not_mine, 100, 0, 5).await; // someone else's
    h.create(seller, bound, 100, 0, 5).await;

    assert_refused(&capture, BMError::InvalidPrice, seller);
    assert_refused(&capture, BMError::InvalidItem, seller);
    assert_refused(&capture, BMError::ItemBound, seller);
    assert_eq!(errors_sent(&capture), 5, "one onBMError per refusal");

    cleanup(&pool, &[seller.1, other.1], &[seller.2, other.2]).await;
}

#[tokio::test]
async fn live_db_every_bid_and_cancel_refusal_logs_its_reason_and_answers() {
    let pool = require_db_or_skip!();
    let seller: Session = (0x7000_AA81, BASE + 0x10, BASE + 0x11);
    let poor: Session = (0x7000_AA82, BASE + 0x12, BASE + 0x13);
    let rich: Session = (0x7000_AA83, BASE + 0x14, BASE + 0x15);
    let ids = [seller.1, poor.1, rich.1];
    let players = [seller.2, poor.2, rich.2];
    cleanup(&pool, &ids, &players).await;
    insert_account_and_player(&pool, seller.1, seller.2, 0).await;
    insert_account_and_player(&pool, poor.1, poor.2, 50).await;
    insert_account_and_player(&pool, rich.1, rich.2, 10_000).await;
    let h = Harness::new(&pool, &[seller, poor, rich]);
    let item = insert_item(&pool, seller.2, ITEM_DEF_ID).await;
    h.create(seller, item, 100, 0, 5).await;
    let seq = last_auction_of(&pool, seller.2).await;

    let capture = LogCapture::install();
    h.bid(rich, i32::MAX, 500).await; // no such auction
    h.bid(seller, seq, 500).await; // own auction
    h.bid(rich, seq, 99).await; // below start
    h.bid(poor, seq, 100).await; // cannot cover it
    h.cancel(rich, seq).await; // not the seller
    assert_refused(&capture, BMError::AuctionGone, rich);
    assert_refused(&capture, BMError::IsSeller, seller);
    assert_refused(&capture, BMError::BidTooLow, rich);
    assert_refused(&capture, BMError::NotEnoughFunds, poor);
    assert_refused(&capture, BMError::NotSeller, rich);
    assert_eq!(errors_sent(&capture), 5);
    drop(capture);

    // The expired window reports AuctionGone too, for the seller's cancel.
    let capture = LogCapture::install();
    expire_now(&pool, seq).await;
    h.cancel(seller, seq).await;
    assert_refused(&capture, BMError::AuctionGone, seller);

    cleanup(&pool, &ids, &players).await;
}

#[tokio::test]
async fn live_db_cap_and_client_key_refusals_log_their_reason() {
    let pool = require_db_or_skip!();
    let seller: Session = (0x7000_AA91, BASE + 0x20, BASE + 0x21);
    cleanup(&pool, &[seller.1], &[seller.2]).await;
    insert_account_and_player(&pool, seller.1, seller.2, 0).await;
    let h = Harness::new(&pool, &[seller]);
    let item = insert_item(&pool, seller.2, ITEM_DEF_ID).await;
    h.create(seller, item, 100, 0, 5).await;
    let seq = last_auction_of(&pool, seller.2).await;

    let capture = LogCapture::install();
    // Full bags no longer refuse a cancel (D-BM10): the item goes back by
    // mail. The cancelled listing no longer counts toward the cap below.
    sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
                                    bound, durability, charges) \
         SELECT $1, $2, 1, s, b, false, 1, 0 \
         FROM (VALUES (1, 40), (15, 100)) AS bags(b, n), generate_series(0, 99) AS s \
         WHERE s < n",
    )
    .bind(seller.2)
    .bind(ITEM_DEF_ID)
    .execute(&pool)
    .await
    .unwrap();
    h.cancel(seller, seq).await;
    assert_eq!(status_of(&pool, seq).await, auction_status::CANCELLED);

    // The cap: 20 open listings.
    sqlx::query(
        "INSERT INTO sgw_auction (seller_id, item_id, item_def_id, starting_price, \
                                  auction_length, created_at, expires_at, status) \
         SELECT $1, 0, $2, 10, 5, 0, 2000000000, 0 FROM generate_series(1, 20)",
    )
    .bind(seller.2)
    .bind(ITEM_DEF_ID)
    .execute(&pool)
    .await
    .unwrap();
    let one_more = insert_item(&pool, seller.2, ITEM_DEF_ID).await;
    h.create(seller, one_more, 100, 0, 5).await;
    assert_refused(&capture, BMError::TooManyListings, seller);

    h.search(
        seller,
        BMSearchOptions {
            client_key: 3,
            ..Default::default()
        },
    )
    .await;
    assert_refused(&capture, BMError::InvalidSortType, seller);

    cleanup(&pool, &[seller.1], &[seller.2]).await;
}
