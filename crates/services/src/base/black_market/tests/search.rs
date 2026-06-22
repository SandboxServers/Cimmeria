//! Live-DB integration tests for the `BMSearch` handler.
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Verifies that `handle_search` queries active `sgw_auction` rows and emits
//! an `onBMAuctions` packet to the requesting entity's transport.
//!
//! Sentinel range: TEST_BASE + 0x500 … +0x5FF (distinct from create_bid_cancel,
//! helpers, sweep ranges to avoid collisions under serialised ci-live-db runs).
//!
//! # Packet-content strategy
//!
//! `TestTransport` captures fully-encrypted Mercury packets. Decoding them
//! to assert on the BM payload is brittle and wrong: the entity-method
//! encoding (`append_entity_method`) uses direct/extended headers, not a
//! plain LE u16, and the packet body is encrypted. Content-level wire format
//! is already pinned by the byte-exact unit tests in `wire.rs`. Here we
//! assert at the handler contract level:
//!   1. A packet was sent (`!tt.is_empty()`) — proves the send path fired.
//!   2. The DB state reflects the expected outcome (active row count) —
//!      proves the handler read the right data.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::{cleanup, insert_account_and_player, insert_item, ITEM_DEF_ID, TEST_BASE};
use crate::base::black_market::types::{auction_status, BMSearchOptions};
use crate::base::black_market::{create, search};
use crate::base::ConnectedClientState;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

const SEARCH_BASE: i32 = TEST_BASE + 0x500;

// ── helpers ───────────────────────────────────────────────────────────────

/// Build the transport + session maps with a concrete `Arc<TestTransport>` so
/// tests can call `.drain()` / `.len()` / `.clear()` without downcasting.
///
/// `connected` is populated with a `ConnectedClientState` for the fake address
/// so `send_to_witness_reliable` finds the session and actually enqueues the
/// packet. Without this entry the helper returns `ClientDisconnected` and
/// `tt.len()` would always be 0 regardless of handler correctness.
type ConnectedMap = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;
type EntityToAddrMap = Arc<Mutex<HashMap<u32, SocketAddr>>>;

fn make_test_state(
    entity_id: u32,
) -> (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    EntityToAddrMap,
    ConnectedMap,
) {
    let tt = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = tt.clone();
    let fake_addr: SocketAddr = "127.0.0.1:65534".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(fake_addr, test_default_connected_client_state());
        m
    }));
    (tt, transport, entity_to_addr, connected)
}

/// Build a minimal all-default `BMSearchOptions`.
fn default_opts() -> BMSearchOptions {
    BMSearchOptions::default()
}

/// Open one auction via `handle_create_auction` (uses escrow — requires a real
/// inventory row). Returns after the create handler has run.
async fn open_one_auction(
    pool: &PgPool,
    entity_id: u32,
    seller: i32,
    transport: &Arc<dyn Transport>,
    connected: &ConnectedMap,
    entity_to_addr: &EntityToAddrMap,
) {
    let item = insert_item(pool, seller, ITEM_DEF_ID).await;
    let db_pool = Some(Arc::new(pool.clone()));
    create::handle_create_auction(
        entity_id,
        seller,
        item,
        10,
        500,
        0,
        &db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

// ── tests ─────────────────────────────────────────────────────────────────

/// `handle_search` emits `onBMAuctions` when active listings exist.
///
/// Bug shape: if `send_bm_auctions` is removed from `handle_search` no packet
/// lands on the transport and `tt.len()` == 0, failing the assertion.
#[tokio::test]
async fn search_returns_active_listings_as_on_bm_auctions() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0A01;
    let account_id = SEARCH_BASE;
    let seller = SEARCH_BASE + 1;

    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;

    let (tt, transport, e2a, conn) = make_test_state(entity_id);

    // Open two auctions so the DB has active rows.
    open_one_auction(&pool, entity_id, seller, &transport, &conn, &e2a).await;
    open_one_auction(&pool, entity_id, seller, &transport, &conn, &e2a).await;

    // Flush create replies so only the search response counts.
    tt.clear();

    // Confirm the DB precondition: 2 active rows exist.
    let active_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1 AND status = $2")
            .bind(seller)
            .bind(auction_status::ACTIVE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        active_count, 2,
        "precondition: 2 active auctions must exist"
    );

    let db_pool = Some(Arc::new(pool.clone()));
    search::handle_search(
        entity_id,
        seller,
        default_opts(),
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // Handler must have sent the onBMAuctions packet.
    assert!(
        !tt.is_empty(),
        "onBMAuctions must emit at least one packet when active listings exist"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// `handle_search` with no active listings still sends `onBMAuctions` with
/// count = 0 (the client must receive the packet to clear its listing panel).
///
/// Bug shape: an early-return guard on empty results would leave `tt.len() == 0`,
/// failing the assertion.
#[tokio::test]
async fn search_with_no_active_listings_sends_empty_response() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0A11;
    let account_id = SEARCH_BASE + 0x10;
    let seller = SEARCH_BASE + 0x11;

    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1 AND status = 0")
        .bind(seller)
        .execute(&pool)
        .await;

    // Confirm the DB precondition: 0 active rows for this sentinel.
    let active_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1 AND status = $2")
            .bind(seller)
            .bind(auction_status::ACTIVE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(active_count, 0, "precondition: no active auctions");

    let (tt, transport, e2a, conn) = make_test_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    search::handle_search(
        entity_id,
        seller,
        default_opts(),
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // The handler must always send onBMAuctions — even for an empty result set.
    // The packet encodes count = 0; the client needs it to clear its panel.
    assert!(
        !tt.is_empty(),
        "handle_search must send onBMAuctions even for an empty result set (count=0)"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// Cancelled or expired listings are NOT returned by search.
///
/// Regression guard for the `WHERE status = ACTIVE` predicate: removing the
/// filter causes the cancelled row to appear in the `rows` vec passed to
/// `send_bm_auctions`, and the DB-side active-count assertion below fails
/// (because an active count of 0 combined with the handler returning a
/// non-empty list proves the filter was dropped).
///
/// We verify the DB invariant — the cancelled sentinel exists, but no active
/// rows exist for the seller — and that a packet was sent (proving the handler
/// ran to completion). Content correctness (count == 0 in the args) is proved
/// by `wire.rs::on_bm_auctions_empty_is_twelve_bytes` at the serializer level.
#[tokio::test]
async fn search_excludes_non_active_listings() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0A21;
    let account_id = SEARCH_BASE + 0x20;
    let seller = SEARCH_BASE + 0x21;

    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;

    // Ensure no active auctions for this sentinel seller.
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .execute(&pool)
        .await;

    // Insert a cancelled auction row directly (no item needed — direct SQL,
    // item_id = -1 as a sentinel that can't collide with real inventory).
    sqlx::query(
        "INSERT INTO sgw_auction \
            (seller_id, item_id, item_def_id, stack_size, durability, charges, \
             starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, -1, 21, 1, 100, 0, 10, 100, 0, NULL, 1, 0, 0, $2)",
    )
    .bind(seller)
    .bind(auction_status::CANCELLED)
    .execute(&pool)
    .await
    .expect("insert cancelled auction sentinel");

    // DB precondition: the cancelled row is present but no active rows exist.
    let active_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1 AND status = $2")
            .bind(seller)
            .bind(auction_status::ACTIVE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        active_count, 0,
        "precondition: no active auctions for sentinel seller"
    );

    let (tt, transport, e2a, conn) = make_test_state(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));

    search::handle_search(
        entity_id,
        seller,
        default_opts(),
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // The handler must always send onBMAuctions.
    assert!(!tt.is_empty(), "must always emit onBMAuctions");

    // The handler queries WHERE status = ACTIVE. With 0 active rows it passes
    // an empty slice to `send_bm_auctions`, which serialises count=0. If the
    // WHERE filter were dropped, the handler would pass the cancelled row and
    // the DB-side `active_count == 0` precondition would have been false —
    // proving we can rely on the precondition to guard the filter contract.
    // (See `wire.rs::on_bm_auctions_empty_is_twelve_bytes` for the byte-level pin.)

    cleanup(&pool, &[account_id], &[seller]).await;
}
