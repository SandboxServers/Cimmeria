//! Live-DB integration tests for the Black Market state machine + sweep.
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Against the bundled local Postgres they exercise: createAuction (row insert
//! + escrow), placeBid (current_bid update + prior-bidder refund + bid-history
//! row), cancelAuction (item return + bidder refund), and the expiry sweep
//! (sold → two mail rows + status SOLD; unsold → item return + status EXPIRED).
//!
//! Sentinels fit in i32 and cleanup deletes by exact sentinel — never by range.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::types::auction_status;
use crate::base::ConnectedClientState;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;

/// Sentinel base for Black Market live-DB tests. Distinct from prior live-DB
/// sentinels (vendor sell +0x800 was the highest documented).
const TEST_BASE: i32 = 0x7000_0900;

/// design_id 21 exists in `resources.items` (used by the vendor sell tests as a
/// known-good type), so `escrow_item` / `return_item`'s INSERT…SELECT against
/// `resources.items` resolves a real row.
const ITEM_DEF_ID: i32 = 21;

async fn cleanup(pool: &PgPool, account_ids: &[i32], player_ids: &[i32]) {
    for &pid in player_ids {
        let _ = sqlx::query("DELETE FROM sgw_auction_bid WHERE bidder_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1 OR current_bidder = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_gate_mail WHERE character_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(pid)
            .execute(pool)
            .await;
    }
    for &aid in account_ids {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(aid)
            .execute(pool)
            .await;
    }
}

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32, naquadah: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("bm-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("bmp-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert_item(pool: &PgPool, player_id: i32, type_id: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, 0, 0, false, 100, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn naquadah_of(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn inventory_count(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn make_state(
    entity_id: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}

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

    super::create::handle_create_auction(
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

    super::create::handle_create_auction(
        entity_id, seller, item, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();

    // Bidder1 bids 200 (>= starting 100).
    super::bid::handle_place_bid(
        entity_id, bidder1, seq, 200, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    assert_eq!(
        naquadah_of(&pool, bidder1).await,
        10_000 - 200,
        "bidder1's cash must be held"
    );

    // Bidder2 outbids at 500. Prior bidder (bidder1) must be refunded their 200.
    super::bid::handle_place_bid(
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

    super::create::handle_create_auction(
        entity_id, seller, item, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();
    super::bid::handle_place_bid(
        entity_id, bidder, seq, 300, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    assert_eq!(naquadah_of(&pool, bidder).await, 10_000 - 300);
    assert_eq!(inventory_count(&pool, seller).await, 0, "item escrowed");

    super::cancel::handle_cancel_auction(entity_id, seller, seq, &db_pool, &transport, &conn, &e2a)
        .await;

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

// ── expiry sweep ────────────────────────────────────────────────────────────

/// Force an auction's `expires_at` into the past.
async fn expire_now(pool: &PgPool, seq: i32) {
    sqlx::query("UPDATE sgw_auction SET expires_at = 1 WHERE sequence_id = $1")
        .bind(seq)
        .execute(pool)
        .await
        .unwrap();
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

    super::create::handle_create_auction(
        entity_id, seller, item, 100, 0, 1, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    let seq: i32 = sqlx::query_scalar("SELECT sequence_id FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();
    super::bid::handle_place_bid(
        entity_id, bidder, seq, 750, &db_pool, &transport, &conn, &e2a,
    )
    .await;
    expire_now(&pool, seq).await;

    // Bug-shape pre-condition: nothing has settled yet.
    let pre_status: i16 =
        sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
            .bind(seq)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(pre_status, auction_status::ACTIVE, "active until swept");

    let settled = super::sweep::settle_expired_once(&pool).await.unwrap();
    assert!(
        settled.iter().any(|s| s.sequence_id == seq && s.sold),
        "sweep must report this auction as sold"
    );

    let status: i16 = sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, auction_status::SOLD, "status → SOLD after sweep");

    // Seller mailed cash (the winning bid).
    let seller_cash_mail: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND cash = 750",
    )
    .bind(seller)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(seller_cash_mail, 1, "seller mailed the winning cash");

    // Buyer mailed an item.
    let buyer_item_mail: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND item_id IS NOT NULL",
    )
    .bind(bidder)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(buyer_item_mail, 1, "buyer mailed the item");

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

    super::create::handle_create_auction(
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

    let settled = super::sweep::settle_expired_once(&pool).await.unwrap();
    assert!(
        settled.iter().any(|s| s.sequence_id == seq && !s.sold),
        "sweep must report this auction as unsold"
    );

    let status: i16 = sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
        .bind(seq)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        status,
        auction_status::EXPIRED,
        "status → EXPIRED after sweep"
    );

    // Unsold item returned via mail attachment to the seller.
    let seller_item_mail: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1 AND item_id IS NOT NULL",
    )
    .bind(seller)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(seller_item_mail, 1, "seller mailed the returned item");

    cleanup(&pool, &[account_id], &[seller]).await;
}
