//! Live-DB integration tests for the expiry sweep (`settle_expired_once`).
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Covers: a sold auction (the escrowed row is mailed to the buyer, the
//! winning bid to the seller, status SOLD), an unsold auction (the row is
//! mailed back to the seller, status EXPIRED), multiple expired auctions
//! settled in one pass, and the phantom-bidder edge where a row has
//! `current_bidder` set but `current_bid = 0`, which must settle as UNSOLD.
//! Every move is system mail (BM-02b): the item leaves `sgw_inventory` for
//! `sgw_gate_mail_item`, whole, and the cash sits on the mail until taken.
//!
//! Shared fixtures live in the parent `tests` module.

use std::sync::Arc;

use cimmeria_entity::inventory::INV_AUCTION;

use super::{
    bm_mails, cleanup, expire_now, insert_account_and_player, insert_item, insert_item_in,
    item_state, mail_escrow_of, make_state, naquadah_of, status_of, ITEM_DEF_ID, TEST_BASE,
};
use crate::base::world_entry::methods::black_market::types::auction_status;
use crate::base::world_entry::methods::black_market::{bid, create, sweep};
use crate::test_support::require_db_or_skip;

/// Sweep settles a sold auction: the listed row itself moves out of the
/// seller's container 18 into a mail to the buyer (every column kept), the
/// seller is mailed the cash, status → SOLD. Bug-shape guard: BEFORE the
/// sweep runs the auction is still ACTIVE and the row still in escrow, so a
/// no-op sweep fails; a settlement that bypassed the mail writer leaves no
/// `sgw_gate_mail_item` row.
#[tokio::test]
async fn live_db_sweep_settles_sold_auction() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_A931;
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
        entity_id, seller, item, 100, 0, 5, &db_pool, &transport, &conn, &e2a,
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

    assert_eq!(status_of(&pool, seq).await, auction_status::ACTIVE);
    assert_eq!(
        item_state(&pool, item).await.map(|s| s.1),
        Some(INV_AUCTION)
    );

    let report = sweep::settle_expired_once(&pool).await.unwrap();
    let settled = report
        .settled
        .iter()
        .find(|s| s.sequence_id == seq)
        .expect("sweep must report this auction");
    assert!(settled.sold);
    assert_eq!(status_of(&pool, seq).await, auction_status::SOLD);

    let seller_mail = bm_mails(&pool, seller).await;
    assert_eq!(seller_mail.len(), 1, "one mail to the seller");
    assert_eq!((seller_mail[0].1, seller_mail[0].2), (750, None), "the bid");
    let buyer_mail = bm_mails(&pool, bidder).await;
    assert_eq!(buyer_mail.len(), 1, "one mail to the buyer");
    assert_eq!((buyer_mail[0].1, buyer_mail[0].2), (0, Some(item)));
    assert_eq!(item_state(&pool, item).await, None, "out of sgw_inventory");
    assert_eq!(
        mail_escrow_of(&pool, item).await,
        Some((buyer_mail[0].0, 77, 3, seller)),
        "the same instance, whole, escrowed on the buyer's mail"
    );
    assert_eq!(
        naquadah_of(&pool, seller).await,
        0,
        "cash waits on the mail"
    );
    assert_eq!(naquadah_of(&pool, bidder).await, 10_000 - 750);
    let mail_ids: Vec<i32> = settled.payouts.iter().map(|p| p.mail.mail_id).collect();
    assert_eq!(mail_ids, vec![buyer_mail[0].0, seller_mail[0].0]);

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
}

/// Sweep settles an unsold auction: the escrowed row is mailed back to the
/// seller and status → EXPIRED.
#[tokio::test]
async fn live_db_sweep_settles_unsold_auction_returns_item() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_A941;
    let account_id = TEST_BASE + 400;
    let seller = TEST_BASE + 410;
    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;

    let (transport, e2a, conn) = make_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    create::handle_create_auction(
        entity_id, seller, item, 100, 0, 5, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();
    expire_now(&pool, seq).await;
    assert_eq!(
        item_state(&pool, item).await.map(|s| s.1),
        Some(INV_AUCTION)
    );

    let report = sweep::settle_expired_once(&pool).await.unwrap();
    assert!(
        report
            .settled
            .iter()
            .any(|s| s.sequence_id == seq && !s.sold),
        "sweep must report this auction as unsold"
    );
    assert_eq!(status_of(&pool, seq).await, auction_status::EXPIRED);
    let mail = bm_mails(&pool, seller).await;
    assert_eq!(mail.len(), 1);
    assert_eq!((mail[0].1, mail[0].2), (0, Some(item)), "item, no cash");
    assert_eq!(item_state(&pool, item).await, None);
    assert_eq!(
        mail_escrow_of(&pool, item).await,
        Some((mail[0].0, 77, 3, seller)),
        "the same row, whole, on the seller's mail"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// One sweep pass settles *every* due auction, not just the first. Two auctions
/// expire together — one sold, one unsold — and both must be settled in a single
/// `settle_expired_once` call with the correct per-auction outcome. Bug shape:
/// a `break`-instead-of-`continue` (or a single-row query) in the sweep loop
/// would settle only one and leave the other ACTIVE.
#[tokio::test]
async fn live_db_sweep_settles_multiple_expired_in_one_pass() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_A951;
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
    for item in [item_sold, item_unsold] {
        create::handle_create_auction(
            entity_id, seller, item, 100, 0, 5, &db_pool, &transport, &conn, &e2a,
        )
        .await;
    }

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

    let report = sweep::settle_expired_once(&pool).await.unwrap();
    let ours: Vec<_> = report
        .settled
        .iter()
        .filter(|s| s.seller_id == seller)
        .collect();
    assert_eq!(ours.len(), 2, "exactly two auctions settled in one pass");
    assert!(ours.iter().any(|s| s.sequence_id == seq_sold && s.sold));
    assert!(ours.iter().any(|s| s.sequence_id == seq_unsold && !s.sold));
    assert_eq!(status_of(&pool, seq_sold).await, auction_status::SOLD);
    assert_eq!(status_of(&pool, seq_unsold).await, auction_status::EXPIRED);

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
}

/// A row with `current_bidder` set but `current_bid = 0` must settle as UNSOLD.
/// Bug shape: dropping the `current_bid > 0` condition would treat this
/// phantom bidder as a winner and hand them the item for nothing. We assert
/// the seller is mailed the item, the bidder gets nothing, and the status is
/// EXPIRED. The row is INSERTed directly so the inconsistent state can be
/// staged precisely.
#[tokio::test]
async fn live_db_sweep_phantom_bidder_zero_bid_settles_unsold() {
    let pool = require_db_or_skip!();
    let acc_seller = TEST_BASE + 600;
    let acc_bidder = TEST_BASE + 601;
    let seller = TEST_BASE + 610;
    let bidder = TEST_BASE + 611;
    let seq = TEST_BASE + 620;
    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .execute(&pool)
        .await;
    insert_account_and_player(&pool, acc_seller, seller, 0).await;
    insert_account_and_player(&pool, acc_bidder, bidder, 10_000).await;
    let item = insert_item_in(&pool, seller, ITEM_DEF_ID, INV_AUCTION, false).await;

    sqlx::query(
        "INSERT INTO sgw_auction \
            (sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
             charges, starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, $2, $3, $4, 1, 100, 0, 100, 0, 0, $5, 1, 1, 1, $6)",
    )
    .bind(seq)
    .bind(seller)
    .bind(item)
    .bind(ITEM_DEF_ID)
    .bind(bidder)
    .bind(auction_status::ACTIVE)
    .execute(&pool)
    .await
    .expect("insert phantom-bidder auction");

    let report = sweep::settle_expired_once(&pool).await.unwrap();
    let this = report
        .settled
        .iter()
        .find(|s| s.sequence_id == seq)
        .expect("phantom-bidder auction must be settled");
    assert!(!this.sold, "zero current_bid must NOT count as sold");
    assert_eq!(this.buyer_id, None);
    assert_eq!(status_of(&pool, seq).await, auction_status::EXPIRED);
    let mail = bm_mails(&pool, seller).await;
    assert_eq!(
        mail.iter().map(|m| m.2).collect::<Vec<_>>(),
        vec![Some(item)],
        "the item is mailed to the seller"
    );
    assert!(bm_mails(&pool, bidder).await.is_empty(), "not the phantom");

    cleanup(&pool, &[acc_seller, acc_bidder], &[seller, bidder]).await;
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .execute(&pool)
        .await;
}

/// Named-telemetry guard (Rule 6): the sweep's one-query seller lookup names a
/// stored player and login, and for a player id with no row (the join finds
/// nothing, so there is no account to read either) returns the id alone: no
/// panic, no wrong name, no empty string.
#[tokio::test]
async fn live_db_sweep_who_is_names_the_seller_and_leaves_a_missing_one_bare() {
    let pool = require_db_or_skip!();
    let acc = TEST_BASE + 320;
    let player = TEST_BASE + 321;
    let missing = TEST_BASE + 322;
    cleanup(&pool, &[acc], &[player, missing]).await;
    insert_account_and_player(&pool, acc, player, 0).await;

    let found = sweep::who_is(&pool, player).await;
    assert_eq!(found.player_id, player);
    assert_eq!(found.player_name, Some(format!("bmp-{player}").as_str()));
    assert_eq!(found.account_name, Some(format!("bm-test-{acc}").as_str()));
    assert_eq!(found.account_id, u32::try_from(acc).ok());

    let bare = sweep::who_is(&pool, missing).await;
    assert_eq!(bare.player_id, missing);
    assert!(bare.player_name.is_none() && bare.account_name.is_none());
    assert!(bare.account_id.is_none());

    cleanup(&pool, &[acc], &[player, missing]).await;
}
