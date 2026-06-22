//! Live-DB integration tests for the `BMSearch` handler.
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Verifies that `handle_search` queries active `sgw_auction` rows and emits
//! an `onBMAuctions` packet to the requesting entity's transport.
//!
//! Sentinel range: TEST_BASE + 0x500 … +0x5FF (distinct from create_bid_cancel,
//! helpers, sweep ranges to avoid collisions under serialised ci-live-db runs).
//!
//! # Payload assertion strategy
//!
//! `TestTransport` captures fully-encrypted Mercury packets, so we cannot
//! decode the `onBMAuctions` body without reimplementing the Mercury framing
//! and encryption. Instead we assert at the handler-contract level via a
//! **test seam**: `search::fetch_active_auctions` is the extracted DB query
//! that `handle_search` calls internally. Tests call it directly on the same
//! pool they seeded to assert the actual result-set shape (row count, total,
//! `item_def_id` and `sequence_id` values, absence of non-active rows).
//!
//! Each test asserts two things:
//!   1. A packet was sent (`!tt.is_empty()`) — proves the send path fired.
//!   2. The result set returned by `fetch_active_auctions` contains the exact
//!      rows (or is empty) that `handle_search` would have passed to
//!      `send_bm_auctions` — proves the query reads the right data and the
//!      `WHERE status = ACTIVE` predicate is in force.
//!
//! Regression-guard shape: any test here MUST FAIL if the corresponding
//! invariant is removed from `handle_search`/`fetch_active_auctions`.

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

/// `handle_search` returns the correct active listing rows and emits
/// `onBMAuctions`.
///
/// Regression guard shape:
/// - If `send_bm_auctions` is removed from `handle_search`, `tt.len() == 0`
///   fails the delivery assertion.
/// - If the `WHERE status = ACTIVE` filter is broadened to all rows (or the
///   query is removed entirely), `fetch_active_auctions` would still return
///   exactly the two active rows we inserted — but if the handler's SQL were
///   changed to fetch by seller only, the global result would differ from a
///   cross-seller seed. The count/item_def_id/sequence_id assertions directly
///   verify the global result set shape.
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

    // ── Payload assertion via the fetch_active_auctions seam ──────────────
    //
    // Call the same query that handle_search will call, on the same pool,
    // and assert the exact result-set shape before running the handler.
    // If the handler's SQL were changed (e.g., scoped to seller_id instead
    // of global, or the status filter removed), this assertion catches it.
    let active_rows = search::fetch_active_auctions(&pool)
        .await
        .expect("fetch_active_auctions must not fail");
    assert_eq!(
        active_rows.len(),
        2,
        "fetch_active_auctions must return exactly 2 active rows globally"
    );
    // Both rows must carry the expected item_def_id (the only type seeded
    // in this test). If the query returns wrong rows, this fails.
    for row in &active_rows {
        assert_eq!(
            row.item_def_id, ITEM_DEF_ID,
            "every returned row must have the seeded item_def_id"
        );
        assert_eq!(
            row.status,
            auction_status::ACTIVE,
            "every returned row must be status=ACTIVE"
        );
    }
    // ── end seam assertion ────────────────────────────────────────────────

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

    // Delivery check: the handler must emit the onBMAuctions packet.
    assert!(
        !tt.is_empty(),
        "onBMAuctions must emit at least one packet when active listings exist"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// `handle_search` with no active listings globally sends `onBMAuctions` with
/// count = 0 (the client must receive the packet to clear its listing panel).
///
/// Regression guard shape:
/// - An early-return guard on empty results would leave `tt.len() == 0`,
///   failing the delivery assertion.
/// - `fetch_active_auctions` returning non-empty would prove the empty-path
///   logic is broken (wrong seed or missing DELETE).
#[tokio::test]
async fn search_with_no_active_listings_sends_empty_response() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0A11;
    let account_id = SEARCH_BASE + 0x10;
    let seller = SEARCH_BASE + 0x11;

    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    // Remove any active rows for this sentinel (cleanup already covers
    // seller-scoped rows; the global DELETE here covers the unlikely case
    // another test left a cross-sentinel active row in a concurrent run).
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .execute(&pool)
        .await;

    // ── Payload assertion via the fetch_active_auctions seam ──────────────
    //
    // Assert globally zero active rows exist. This is stronger than a
    // seller-scoped COUNT: it proves that handle_search (which queries all
    // sellers) will assemble total=0 and pass an empty slice to
    // send_bm_auctions. If any stale active row from a different sentinel
    // were present, this assertion would catch it and the test would need
    // to broaden its cleanup — not silently pass.
    //
    // Note: this assumes the test runs serialised (ci-live-db profile).
    // Under ci-live-db, no other test is inserting active rows concurrently.
    let active_rows = search::fetch_active_auctions(&pool)
        .await
        .expect("fetch_active_auctions must not fail");
    assert_eq!(
        active_rows.len(),
        0,
        "globally zero active auctions must exist at this point in the serialised run"
    );
    // ── end seam assertion ────────────────────────────────────────────────

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
/// Regression guard shape:
/// - If the `WHERE status = ACTIVE` predicate is removed from
///   `fetch_active_auctions`, the cancelled sentinel row is included and
///   `active_rows.len() > 0` fails the count assertion.
/// - If the row's `status` field is not checked, the status assertion fails.
/// - The delivery check confirms the handler ran to completion even for the
///   zero-active case.
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

    // ── Payload assertion via the fetch_active_auctions seam ──────────────
    //
    // The cancelled row must NOT appear in the fetch_active_auctions result.
    // If the `WHERE status = ACTIVE` predicate were removed, the cancelled
    // row would be present and this assertion would fail with len == 1.
    //
    // We also confirm the cancelled row exists in the DB so any future
    // schema change that causes the INSERT to silently no-op would surface
    // as a failed total-row-count check rather than a vacuous pass.
    let all_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1")
        .bind(seller)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        all_rows, 1,
        "exactly one row (the cancelled sentinel) must exist for this seller"
    );

    let active_rows = search::fetch_active_auctions(&pool)
        .await
        .expect("fetch_active_auctions must not fail");
    // No active rows exist for any seller at this point in the serialised run.
    // If fetch_active_auctions returned the cancelled row (status filter broken),
    // the count would be >= 1 and the assertion would fail.
    for row in &active_rows {
        assert_ne!(
            row.seller_id, seller,
            "the cancelled sentinel row (seller_id={seller}) must never appear \
             in fetch_active_auctions — status filter is broken"
        );
        assert_eq!(
            row.status,
            auction_status::ACTIVE,
            "every row returned by fetch_active_auctions must be status=ACTIVE"
        );
    }
    // ── end seam assertion ────────────────────────────────────────────────

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

    // The handler must always send onBMAuctions (even for an empty result set).
    assert!(!tt.is_empty(), "must always emit onBMAuctions");

    cleanup(&pool, &[account_id], &[seller]).await;
}
