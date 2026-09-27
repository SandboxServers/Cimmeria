//! Live-DB integration tests for `BMSearch`: the `clientKey` views and
//! their scoping to the caller (S2, S3), the filters, the cursor and the
//! page bound with the full `totalResults` (S7), and seller names from the
//! database (S8).
//!
//! Skip cleanly when `DATABASE_URL` is unset (via `require_db_or_skip!`).
//! The payload itself is pinned byte for byte by `wire/tests.rs`; these call
//! [`search::run_search`], the query seam `handle_search` sends from, so they
//! can assert on rows without decrypting a Mercury packet. Every test but
//! one scopes itself with the My Auctions or My Bids view, so rows another
//! test left behind cannot leak in.
//!
//! Sentinel range: TEST_BASE + 0x500 … +0x5FF.

use sqlx::PgPool;

use super::{
    cleanup, insert_account_and_player, insert_item, last_auction_of, Harness, ITEM_DEF_ID,
    TEST_BASE,
};
use crate::base::black_market::helpers::now_unix_secs;
use crate::base::black_market::search::{self, SEARCH_PAGE_ROWS};
use crate::base::black_market::types::{auction_status, BMSearchOptions};
use crate::base::black_market::wire::BMError;
use crate::test_support::{require_db_or_skip, LogCapture};

const SEARCH_BASE: i32 = TEST_BASE + 0x500;

fn view(client_key: i32) -> BMSearchOptions {
    BMSearchOptions {
        client_key,
        quality: 2000,
        ..Default::default()
    }
}

/// Insert an open listing for `seller` straight into `sgw_auction` (search
/// needs no item row). Returns its id.
async fn insert_listing(pool: &PgPool, seller: i32, expires_at: i32, status: i16) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_auction \
            (seller_id, item_id, item_def_id, stack_size, durability, charges, \
             starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, 0, $2, 1, 100, 0, 10, 0, 0, NULL, 5, 0, $3, $4) \
         RETURNING sequence_id",
    )
    .bind(seller)
    .bind(ITEM_DEF_ID)
    .bind(expires_at)
    .bind(status)
    .fetch_one(pool)
    .await
    .expect("insert listing")
}

async fn page(pool: &PgPool, player: i32, opts: &BMSearchOptions) -> search::SearchPage {
    search::run_search(pool, player, opts, now_unix_secs())
        .await
        .expect("query runs")
        .expect("valid view")
}

/// S3 regression guard: My Auctions is the **caller's** listings. The
/// client's `sellerName` names someone else here, and is ignored. Bug shape:
/// the branch returned every listing (and a seller-name filter would trust
/// the client).
#[tokio::test]
async fn my_auctions_is_scoped_to_the_caller() {
    let pool = require_db_or_skip!();
    let (acc_a, acc_b) = (SEARCH_BASE, SEARCH_BASE + 1);
    let (a, b) = (SEARCH_BASE + 2, SEARCH_BASE + 3);
    cleanup(&pool, &[acc_a, acc_b], &[a, b]).await;
    insert_account_and_player(&pool, acc_a, a, 0).await;
    insert_account_and_player(&pool, acc_b, b, 0).await;
    let later = now_unix_secs() + 3_600;
    let mine = insert_listing(&pool, a, later, auction_status::ACTIVE).await;
    insert_listing(&pool, b, later, auction_status::ACTIVE).await;

    let mut opts = view(1);
    opts.seller_name = format!("bmp-{b}");
    let p = page(&pool, a, &opts).await;
    assert_eq!(p.total, 1);
    assert_eq!(
        p.rows
            .iter()
            .map(|(r, _)| r.sequence_id)
            .collect::<Vec<_>>(),
        vec![mine]
    );

    cleanup(&pool, &[acc_a, acc_b], &[a, b]).await;
}

/// S3: My Bids is every open auction the caller has bid on, still shown
/// after they are outbid; someone else's bids are not.
#[tokio::test]
async fn my_bids_is_what_the_caller_bid_on() {
    let pool = require_db_or_skip!();
    let (acc_s, acc_c, acc_d) = (SEARCH_BASE + 0x10, SEARCH_BASE + 0x11, SEARCH_BASE + 0x12);
    let (seller, c, d) = (SEARCH_BASE + 0x13, SEARCH_BASE + 0x14, SEARCH_BASE + 0x15);
    cleanup(&pool, &[acc_s, acc_c, acc_d], &[seller, c, d]).await;
    insert_account_and_player(&pool, acc_s, seller, 0).await;
    insert_account_and_player(&pool, acc_c, c, 10_000).await;
    insert_account_and_player(&pool, acc_d, d, 10_000).await;
    let h = Harness::new(
        &pool,
        &[
            (0x7000_AA41, acc_c, c),
            (0x7000_AA42, acc_d, d),
            (0x7000_AA43, acc_s, seller),
        ],
    );
    let item = insert_item(&pool, seller, ITEM_DEF_ID).await;
    h.create((0x7000_AA43, acc_s, seller), item, 100, 0, 5)
        .await;
    let seq = last_auction_of(&pool, seller).await;
    h.bid((0x7000_AA41, acc_c, c), seq, 100).await;
    h.bid((0x7000_AA42, acc_d, d), seq, 200).await;

    let for_c = page(&pool, c, &view(2)).await;
    assert_eq!(for_c.total, 1, "C was outbid but still bid on it");
    assert_eq!(for_c.rows[0].0.sequence_id, seq);
    assert_eq!(for_c.rows[0].0.current_bidder, Some(d));
    let for_seller = page(&pool, seller, &view(2)).await;
    assert_eq!(for_seller.total, 0, "the seller bid on nothing");

    cleanup(&pool, &[acc_s, acc_c, acc_d], &[seller, c, d]).await;
}

/// A cancelled row and a row past `expires_at` that the sweep has not
/// reached are not offered.
#[tokio::test]
async fn closed_and_due_listings_are_not_offered() {
    let pool = require_db_or_skip!();
    let (acc, seller) = (SEARCH_BASE + 0x20, SEARCH_BASE + 0x21);
    cleanup(&pool, &[acc], &[seller]).await;
    insert_account_and_player(&pool, acc, seller, 0).await;
    let now = now_unix_secs();
    let open = insert_listing(&pool, seller, now + 3_600, auction_status::ACTIVE).await;
    insert_listing(&pool, seller, now + 3_600, auction_status::CANCELLED).await;
    insert_listing(&pool, seller, now - 1, auction_status::ACTIVE).await;

    let p = page(&pool, seller, &view(1)).await;
    assert_eq!(p.total, 1);
    assert_eq!(p.rows[0].0.sequence_id, open);

    cleanup(&pool, &[acc], &[seller]).await;
}

/// S7 regression guard: the read is bounded by [`SEARCH_PAGE_ROWS`], the
/// cursor pages both ways, and `total` stays the full match count. Bug
/// shape: the branch read every row and reported the page size as total.
#[tokio::test]
async fn search_pages_with_a_cursor_and_keeps_the_full_total() {
    let pool = require_db_or_skip!();
    let (acc, seller) = (SEARCH_BASE + 0x30, SEARCH_BASE + 0x31);
    cleanup(&pool, &[acc], &[seller]).await;
    insert_account_and_player(&pool, acc, seller, 0).await;
    let later = now_unix_secs() + 3_600;
    let n = SEARCH_PAGE_ROWS as usize + 10;
    let mut ids = Vec::new();
    for _ in 0..n {
        ids.push(insert_listing(&pool, seller, later, auction_status::ACTIVE).await);
    }

    let first = page(&pool, seller, &view(1)).await;
    assert_eq!(first.total, n as i64, "total counts every match");
    assert_eq!(
        first.rows.len(),
        SEARCH_PAGE_ROWS as usize,
        "the read is bounded"
    );
    assert_eq!(first.rows[0].0.sequence_id, ids[0]);

    let mut fwd = view(1);
    fwd.sequence_id = ids[SEARCH_PAGE_ROWS as usize - 1];
    fwd.b_forward = 1;
    let next = page(&pool, seller, &fwd).await;
    assert_eq!(
        next.rows
            .iter()
            .map(|(r, _)| r.sequence_id)
            .collect::<Vec<_>>(),
        ids[SEARCH_PAGE_ROWS as usize..].to_vec()
    );
    assert_eq!(next.total, n as i64);

    let mut back = view(1);
    back.sequence_id = ids[3];
    back.b_forward = 0;
    let prev = page(&pool, seller, &back).await;
    assert_eq!(
        prev.rows
            .iter()
            .map(|(r, _)| r.sequence_id)
            .collect::<Vec<_>>(),
        ids[..3].to_vec(),
        "a backward page is the rows before the cursor, in ascending order"
    );

    cleanup(&pool, &[acc], &[seller]).await;
}

/// `itemName` matches a substring of the item's name, case-insensitively;
/// `minTC` / `maxTC` bound the tech competency.
#[tokio::test]
async fn item_name_and_tech_competency_filter() {
    let pool = require_db_or_skip!();
    let (acc, seller) = (SEARCH_BASE + 0x40, SEARCH_BASE + 0x41);
    cleanup(&pool, &[acc], &[seller]).await;
    insert_account_and_player(&pool, acc, seller, 0).await;
    insert_listing(
        &pool,
        seller,
        now_unix_secs() + 3_600,
        auction_status::ACTIVE,
    )
    .await;
    let (name, tc): (String, i32) =
        sqlx::query_as("SELECT name, tech_comp FROM resources.items WHERE item_id = $1")
            .bind(ITEM_DEF_ID)
            .fetch_one(&pool)
            .await
            .unwrap();

    let mut opts = view(1);
    opts.item_name = name
        .chars()
        .skip(1)
        .take(3)
        .collect::<String>()
        .to_uppercase();
    assert_eq!(
        page(&pool, seller, &opts).await.total,
        1,
        "substring of {name:?}"
    );
    opts.item_name = "zz%no such item%".into();
    assert_eq!(page(&pool, seller, &opts).await.total, 0);

    let mut opts = view(1);
    opts.min_tc = tc + 1;
    assert_eq!(page(&pool, seller, &opts).await.total, 0);
    opts.min_tc = tc;
    opts.max_tc = tc;
    assert_eq!(page(&pool, seller, &opts).await.total, 1);

    cleanup(&pool, &[acc], &[seller]).await;
}

/// S2: a `clientKey` that names no view is refused, not served.
#[tokio::test]
async fn unknown_client_key_is_refused() {
    let pool = require_db_or_skip!();
    let res = search::run_search(&pool, SEARCH_BASE + 0x50, &view(7), now_unix_secs())
        .await
        .unwrap();
    assert_eq!(res, Err(BMError::InvalidSortType));
}

/// S8: an offline seller's name comes from the database, and the handler
/// sends `onBMAuctions` with the search telemetry row.
#[tokio::test]
async fn offline_seller_names_come_from_the_db() {
    let pool = require_db_or_skip!();
    let (acc, seller) = (SEARCH_BASE + 0x60, SEARCH_BASE + 0x61);
    let (acc_c, caller) = (SEARCH_BASE + 0x62, SEARCH_BASE + 0x63);
    cleanup(&pool, &[acc, acc_c], &[seller, caller]).await;
    insert_account_and_player(&pool, acc, seller, 0).await;
    insert_account_and_player(&pool, acc_c, caller, 0).await;
    insert_listing(
        &pool,
        seller,
        now_unix_secs() + 3_600,
        auction_status::ACTIVE,
    )
    .await;

    let p = page(&pool, seller, &view(1)).await;
    assert_eq!(p.rows[0].1, format!("bmp-{seller}"), "no session needed");

    let capture = LogCapture::install();
    let me = (0x7000_AA61, acc, seller);
    let h = Harness::new(&pool, &[me]);
    h.search(me, view(1)).await;
    assert!(!h.tt.is_empty(), "onBMAuctions sent");
    let ev = capture
        .find_message(tracing::Level::INFO, "search: returning listings")
        .expect("search row");
    for (k, v) in [
        ("client_key", "1".to_string()),
        ("total_results", "1".to_string()),
        ("rows_returned", "1".to_string()),
        ("player_id", seller.to_string()),
        ("account_id", acc.to_string()),
    ] {
        assert!(ev.has_field(k, &v), "{k}: {:?}", ev.fields);
    }

    cleanup(&pool, &[acc, acc_c], &[seller, caller]).await;
}
