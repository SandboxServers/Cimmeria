//! Live-DB guards for BM-02b's settlement rules that are not one outcome's
//! happy path: a settlement pays exactly once, and one auction that cannot
//! settle is quarantined without stopping the sweep pass (plan §5.2).
//!
//! The per-outcome mail guards are beside the handlers they drive: sold,
//! unsold and the phantom bidder in `sweep.rs`, outbid and cancelled in
//! `create_bid_cancel.rs`, the buyout in `buyout_and_rules.rs`.
//!
//! Sentinel range: TEST_BASE + 0x900 … +0x9FF.

use cimmeria_entity::inventory::INV_AUCTION;
use sqlx::PgPool;
use tracing::Level;

use super::{
    bm_mails, cleanup, expire_now, insert_account_and_player, insert_item, item_state,
    last_auction_of, naquadah_of, status_of, Harness, Session, ITEM_DEF_ID, TEST_BASE,
};
use crate::base::world_entry::methods::black_market::settle::{
    settle_locked, SettleCause, SettleError,
};
use crate::base::world_entry::methods::black_market::sweep;
use crate::base::world_entry::methods::black_market::types::{
    auction_columns, auction_status, AuctionRow,
};
use crate::test_support::{require_db_or_skip, LogCapture};

const BASE: i32 = TEST_BASE + 0x900;

async fn auction_row(pool: &PgPool, seq: i32) -> AuctionRow {
    sqlx::query_as(concat!(
        "SELECT ",
        auction_columns!(),
        " FROM sgw_auction WHERE sequence_id = $1"
    ))
    .bind(seq)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Settle `stale` in a transaction of its own, as a sweep or buyout that
/// read the row while it was still ACTIVE would.
async fn settle_from(pool: &PgPool, stale: &AuctionRow) -> Result<(), SettleError> {
    let mut tx = pool.begin().await.unwrap();
    settle_locked(&mut tx, stale, SettleCause::Expired).await?;
    tx.commit().await.unwrap();
    Ok(())
}

/// A second settlement of the same auction, from a snapshot taken while it
/// was still ACTIVE, writes nothing: the conditional status write is the
/// first step and matches no row (`Gone`). The mail writer mints cash and
/// items on every call, so this gate is all that stops a double payout.
///
/// Bug shape: without `AND status = ACTIVE` the boot-seed listing (which
/// mails a new instance) pays its buyer twice. The player listing is
/// covered too, although its escrowed row can only move once anyway.
#[tokio::test]
async fn live_db_a_second_settlement_pays_nothing() {
    let pool = require_db_or_skip!();
    let (acc_seller, seller, acc_buyer, buyer) = (BASE, BASE + 1, BASE + 2, BASE + 3);
    let seed_seq = BASE + 0x50;
    cleanup(&pool, &[acc_seller, acc_buyer], &[seller, buyer]).await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_buyer, buyer, 10_000).await;

    // A seed-shaped listing (no instance) with a winning bid.
    sqlx::query(
        "INSERT INTO sgw_auction \
            (sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
             charges, starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, $2, 0, $3, 1, 100, 0, 100, 0, 400, $4, 1, 1, 1, $5)",
    )
    .bind(seed_seq)
    .bind(seller)
    .bind(ITEM_DEF_ID)
    .bind(buyer)
    .bind(auction_status::ACTIVE)
    .execute(&pool)
    .await
    .unwrap();
    let stale = auction_row(&pool, seed_seq).await;
    settle_from(&pool, &stale).await.expect("first settlement");
    let second = settle_from(&pool, &stale).await;
    assert!(matches!(second, Err(SettleError::Gone)), "{second:?}");
    assert_eq!(status_of(&pool, seed_seq).await, auction_status::SOLD);
    let won = bm_mails(&pool, buyer).await;
    assert_eq!(won.len(), 1, "one minted item, not two: {won:?}");
    assert!(won[0].2.is_some());
    assert!(
        bm_mails(&pool, seller).await.is_empty(),
        "a seed listing pays no seller"
    );

    // A player's listing, sold.
    let s: Session = (0x7000_AA90, acc_seller, seller);
    let b: Session = (0x7000_AA91, acc_buyer, buyer);
    let h = Harness::new(&pool, &[s, b]);
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;
    h.create(s, item, 100, 0, 5).await;
    // Not `last_auction_of`: the seed row's sentinel id is the larger one.
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE item_id = $1")
        .bind(item)
        .fetch_one(&pool)
        .await
        .unwrap();
    h.bid(b, seq, 750).await;
    let stale = auction_row(&pool, seq).await;
    settle_from(&pool, &stale).await.expect("first settlement");
    let second = settle_from(&pool, &stale).await;
    assert!(matches!(second, Err(SettleError::Gone)), "{second:?}");
    let paid = bm_mails(&pool, seller).await;
    assert_eq!(
        paid.iter().map(|m| (m.1, m.2)).collect::<Vec<_>>(),
        vec![(750, None)],
        "the seller is paid once"
    );
    assert_eq!(
        bm_mails(&pool, buyer).await.len(),
        2,
        "seed item + this one"
    );

    cleanup(&pool, &[acc_seller, acc_buyer], &[seller, buyer]).await;
}

/// One sweep pass with three due auctions, the first two poison: one whose
/// escrowed row is gone, one whose row became bound and was won by someone
/// else (the mail writer refuses it, `item_bound`). Both are rolled back and
/// set QUARANTINED with their `reason`, nothing is mailed or minted for
/// them, the bidder's cash stays held, and the healthy third auction still
/// settles in the same pass. The next pass leaves the quarantined rows
/// alone.
///
/// Bug shape: the pre-BM-02b sweep propagated the first error with `?`, so
/// the healthy auction (ordered after the poison) stayed ACTIVE forever.
#[tokio::test]
async fn live_db_a_poison_row_is_quarantined_and_the_pass_goes_on() {
    let pool = require_db_or_skip!();
    let (acc_seller, seller, acc_bidder, bidder) =
        (BASE + 0x10, BASE + 0x11, BASE + 0x12, BASE + 0x13);
    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_bidder, bidder, 10_000).await;
    let s: Session = (0x7000_AA92, acc_seller, seller);
    let b: Session = (0x7000_AA93, acc_bidder, bidder);
    let h = Harness::new(&pool, &[s, b]);

    let mut seqs = Vec::new();
    let mut items = Vec::new();
    for _ in 0..3 {
        let item = insert_item(&pool, seller, ITEM_DEF_ID).await;
        h.create(s, item, 100, 0, 5).await;
        seqs.push(last_auction_of(&pool, seller).await);
        items.push(item);
    }
    let (missing, bound, healthy) = (seqs[0], seqs[1], seqs[2]);
    h.bid(b, bound, 300).await;
    sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
        .bind(items[0])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE sgw_inventory SET bound = true WHERE item_id = $1")
        .bind(items[1])
        .execute(&pool)
        .await
        .unwrap();
    for seq in &seqs {
        expire_now(&pool, *seq).await;
    }

    let capture = LogCapture::install();
    let report = sweep::settle_expired_once(&pool).await.unwrap();
    assert!(
        report.quarantined.contains(&(missing, "escrow_missing")),
        "{:?}",
        report.quarantined
    );
    assert!(
        report.quarantined.contains(&(bound, "item_bound")),
        "{:?}",
        report.quarantined
    );
    assert!(report.settled.iter().any(|x| x.sequence_id == healthy));
    assert_eq!(status_of(&pool, missing).await, auction_status::QUARANTINED);
    assert_eq!(status_of(&pool, bound).await, auction_status::QUARANTINED);
    assert_eq!(status_of(&pool, healthy).await, auction_status::EXPIRED);

    assert_eq!(
        bm_mails(&pool, seller)
            .await
            .iter()
            .map(|m| (m.1, m.2))
            .collect::<Vec<_>>(),
        vec![(0, Some(items[2]))],
        "only the healthy auction mailed anything"
    );
    assert!(bm_mails(&pool, bidder).await.is_empty());
    assert_eq!(item_state(&pool, items[0]).await, None, "nothing minted");
    assert_eq!(
        item_state(&pool, items[1]).await.map(|i| (i.0, i.1)),
        Some((seller, INV_AUCTION)),
        "the bound row stays in escrow for an operator"
    );
    assert_eq!(naquadah_of(&pool, bidder).await, 10_000 - 300, "bid held");

    let ev = capture
        .find_event(Level::ERROR, "quarantined", "escrow_missing")
        .unwrap_or_else(|| panic!("no bm.quarantined row: {:#?}", capture.all()));
    assert!(ev.has_field("auction_id", &missing.to_string()), "{ev:?}");
    assert!(ev.has_field("player_id", &seller.to_string()), "{ev:?}");
    assert!(
        ev.has_field("account_id", &acc_seller.to_string()),
        "{ev:?}"
    );
    let ev = capture
        .find_event(Level::ERROR, "quarantined", "item_bound")
        .unwrap_or_else(|| panic!("no bm.quarantined row: {:#?}", capture.all()));
    assert!(ev.has_field("held_cash", "300"), "{ev:?}");
    drop(capture);

    let next = sweep::settle_expired_once(&pool).await.unwrap();
    assert!(!next.quarantined.iter().any(|q| seqs.contains(&q.0)));
    assert!(!next.settled.iter().any(|x| seqs.contains(&x.sequence_id)));

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
}
