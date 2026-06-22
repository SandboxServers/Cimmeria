//! Live-DB integration tests for the `BMSearch` handler.
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! Verifies that `handle_search` queries active `sgw_auction` rows and emits
//! the correct `onBMAuctions` packet to the requesting entity's transport.
//!
//! Sentinel range: TEST_BASE + 0x500 … +0x5FF (distinct from create_bid_cancel,
//! helpers, sweep ranges to avoid collisions under serialised ci-live-db runs).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::{cleanup, insert_account_and_player, insert_item, ITEM_DEF_ID, TEST_BASE};
use crate::base::black_market::types::{auction_status, BMSearchOptions};
use crate::base::black_market::{create, search};
use crate::base::ConnectedClientState;
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, TestTransport};

const SEARCH_BASE: i32 = TEST_BASE + 0x500;

// ── helpers ───────────────────────────────────────────────────────────────

/// Build the transport + session maps with a concrete `Arc<TestTransport>` so
/// tests can call `.drain()` / `.len()` without downcasting.
fn make_test_state(
    entity_id: u32,
) -> (
    Arc<TestTransport>,
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let tt = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = tt.clone();
    let fake_addr: SocketAddr = "127.0.0.1:65534".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
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
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let item = insert_item(pool, seller, ITEM_DEF_ID).await;
    let db_pool = Some(Arc::new(pool.clone()));
    create::handle_create_auction(
        entity_id, seller, item, 10, 500, 0, &db_pool, transport, connected, entity_to_addr,
    )
    .await;
}

// ── tests ─────────────────────────────────────────────────────────────────

/// `handle_search` emits `onBMAuctions` (method 92) when active listings exist.
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

    // Open two auctions so we can assert count > 0 in the response.
    open_one_auction(&pool, entity_id, seller, &transport, &conn, &e2a).await;
    open_one_auction(&pool, entity_id, seller, &transport, &conn, &e2a).await;

    // Flush create replies so only the search response counts.
    tt.clear();

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

    // At least one packet must have been sent.
    assert!(tt.len() > 0, "onBMAuctions must emit at least one packet");

    // The packet must carry ON_BM_AUCTIONS (method 92). The method index is
    // encoded in little-endian by build_player_entity_method_packet; we scan
    // for those two bytes anywhere in the payload rather than hard-coding the
    // Mercury frame offset.
    let method_le = (method_idx::ON_BM_AUCTIONS as u16).to_le_bytes();
    let packets = tt.drain();
    let has_method = packets
        .iter()
        .any(|(_, bytes)| bytes.windows(2).any(|w| w == method_le));
    assert!(
        has_method,
        "emitted packet must carry ON_BM_AUCTIONS method index (92)"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// `handle_search` with no active listings still sends `onBMAuctions` with
/// count = 0 (the client must receive the packet to clear its listing panel).
#[tokio::test]
async fn search_with_no_active_listings_sends_empty_response() {
    let pool = require_db_or_skip!();
    let entity_id: u32 = 0x7000_0A11;
    let account_id = SEARCH_BASE + 0x10;
    let seller = SEARCH_BASE + 0x11;

    cleanup(&pool, &[account_id], &[seller]).await;
    insert_account_and_player(&pool, account_id, seller, 0).await;
    // Ensure no stray active auctions for this sentinel seller.
    let _ = sqlx::query("DELETE FROM sgw_auction WHERE seller_id = $1 AND status = 0")
        .bind(seller)
        .execute(&pool)
        .await;

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

    assert!(
        tt.len() > 0,
        "handle_search must send onBMAuctions even for an empty result set"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}

/// Cancelled or expired listings are NOT returned by search.
///
/// Regression guard for the `WHERE status = ACTIVE` predicate: removing the
/// filter causes the cancelled row to appear in results, the count byte at the
/// start of the args becomes non-zero, and the check below fails.
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

    // Insert a cancelled auction row directly.
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

    assert!(tt.len() > 0, "must always emit onBMAuctions");

    // `serialize_on_bm_auctions` writes [u32 count] as the first 4 bytes of
    // the args slice. `build_player_entity_method_packet` prepends a Mercury
    // header; the args start after the header. We look for the pattern
    // [count=0 LE, view LE, total LE] — the simplest assertion is that the
    // very first 4 bytes of the args region are all zero (count = 0 LE).
    // Because the method index (92 = 0x5C) appears in the header, we can
    // locate the args by finding the method index bytes, then reading 4 bytes
    // forward.
    let method_le = (method_idx::ON_BM_AUCTIONS as u16).to_le_bytes();
    let packets = tt.drain();
    let count_zero = packets.iter().any(|(_, bytes)| {
        // Find the method index in the packet, then read 4 bytes after it
        // as the count field of the args.
        bytes.windows(2).enumerate().any(|(i, w)| {
            if w != method_le {
                return false;
            }
            // args immediately follow the method index (2 bytes) within the
            // Mercury payload; the count is the first 4 bytes of args.
            let args_start = i + 2;
            if bytes.len() < args_start + 4 {
                return false;
            }
            let count =
                u32::from_le_bytes([bytes[args_start], bytes[args_start + 1], bytes[args_start + 2], bytes[args_start + 3]]);
            count == 0
        })
    });
    assert!(
        count_zero,
        "onBMAuctions count must be 0 — cancelled listing must not appear"
    );

    cleanup(&pool, &[account_id], &[seller]).await;
}
