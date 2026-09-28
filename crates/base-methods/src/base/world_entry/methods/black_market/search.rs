//! `BMSearch`: query open listings and reply with one page of
//! `onBMAuctions`.
//!
//! - **View (S2, S3).** `clientKey` is the `UIAuctionView` the reply fills
//!   and is echoed back. Search Results (0) is every open listing; My
//!   Auctions (1) is the caller's own listings and My Bids (2) the listings
//!   the caller has bid on, both keyed on the **caller's `player_id`**. The
//!   client's `sellerName` / `bidderName` strings are ignored: they only
//!   ever carry the client's own name, and trusting them would let any
//!   client read anyone's bids. Any other key is refused `InvalidSortType`.
//! - **Filters.** `itemName` (a case-insensitive substring of the item
//!   name) and `minTC` / `maxTC` (tech competency, 0 = no bound). `quality`,
//!   `sortId` and the eleventh field are logged, not applied: the shipped UI
//!   sends fixed values for them.
//! - **Paging (S7).** `sequenceId` is the cursor (the last auction id the
//!   client saw; 0 = the start) and `bForward` the direction. At most
//!   [`SEARCH_PAGE_ROWS`] rows are read, and the reply is cut to what fits
//!   one message (`wire::AUCTIONS_ARG_BUDGET`). `totalResults` is the full
//!   match count, not the page size.
//! - **Names (S8).** Seller names come from `sgw_player`, so an offline
//!   seller still shows.
//!
//! "Open" means `ACTIVE` and not yet due: a row past `expires_at` that the
//! sweep has not reached is not shown.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::black_market::UIAuctionView;
use sqlx::PgPool;

use super::helpers::now_unix_secs;
use super::send::{send_bm_auctions, send_bm_error, BmNet};
use super::telemetry::{count_bm_outcome, log_failure, Actor, Failure};
use super::types::{auction_status, AuctionRow, BMSearchOptions};
use super::wire::{serialize_on_bm_auctions, BMError};
use crate::base::ConnectedClientState;

/// Most rows one search reads. The reply is then cut by size, which at
/// typical row sizes keeps about 23 of them.
pub const SEARCH_PAGE_ROWS: i64 = 50;

/// The match condition every search shares: open, the view's scope and the
/// filters. `$1` now, `$2` view, `$3` caller, `$4` name pattern, `$5` / `$6`
/// tech competency bounds.
macro_rules! search_match {
    () => {
        " FROM sgw_auction a \
          LEFT JOIN resources.items ri ON ri.item_id = a.item_def_id \
          WHERE a.status = 0 AND a.expires_at > $1 \
            AND ($2 <> 1 OR a.seller_id = $3) \
            AND ($2 <> 2 OR EXISTS (SELECT 1 FROM sgw_auction_bid b \
                                    WHERE b.sequence_id = a.sequence_id AND b.bidder_id = $3)) \
            AND ($4::text IS NULL OR ri.name ILIKE $4) \
            AND ($5::int IS NULL OR ri.tech_comp >= $5) \
            AND ($6::int IS NULL OR ri.tech_comp <= $6)"
    };
}

#[derive(sqlx::FromRow)]
struct SearchRow {
    #[sqlx(flatten)]
    auction: AuctionRow,
    seller_name: Option<String>,
}

/// One page of a search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPage {
    pub view: UIAuctionView,
    /// The page, in ascending `sequence_id`, with each seller's name.
    pub rows: Vec<(AuctionRow, String)>,
    /// Every match, ignoring the cursor.
    pub total: i64,
}

/// `%needle%` for `ILIKE`, with the pattern characters escaped.
fn like_pattern(needle: &str) -> Option<String> {
    if needle.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(needle.len() + 2);
    out.push('%');
    for c in needle.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    Some(out)
}

/// Run a search for `player_id`. `Ok(Err(_))` is a refusal (an unknown
/// `clientKey`). The page is at most [`SEARCH_PAGE_ROWS`] rows; the caller
/// cuts it to the message budget.
pub async fn run_search(
    pool: &PgPool,
    player_id: i32,
    opts: &BMSearchOptions,
    now: i32,
) -> Result<Result<SearchPage, BMError>, sqlx::Error> {
    let Ok(view) = UIAuctionView::try_from(opts.client_key) else {
        return Ok(Err(BMError::InvalidSortType));
    };
    let pattern = like_pattern(&opts.item_name);
    let min_tc = (opts.min_tc > 0).then_some(opts.min_tc);
    let max_tc = (opts.max_tc > 0).then_some(opts.max_tc);
    let forward = opts.b_forward != 0 || opts.sequence_id <= 0;

    let total: i64 = sqlx::query_scalar(concat!("SELECT COUNT(*)", search_match!()))
        .bind(now)
        .bind(view as i32)
        .bind(player_id)
        .bind(&pattern)
        .bind(min_tc)
        .bind(max_tc)
        .fetch_one(pool)
        .await?;

    let rows: Vec<SearchRow> = sqlx::query_as(concat!(
        "SELECT a.sequence_id, a.seller_id, a.item_id, a.item_def_id, a.stack_size, \
                a.durability, a.charges, a.starting_price, a.buyout_price, a.current_bid, \
                a.current_bidder, a.auction_length, a.created_at, a.expires_at, a.status, \
                (SELECT p.player_name FROM sgw_player p WHERE p.player_id = a.seller_id) \
                    AS seller_name",
        search_match!(),
        " AND ($7 <= 0 OR ($8 AND a.sequence_id > $7) OR (NOT $8 AND a.sequence_id < $7)) \
          ORDER BY CASE WHEN $8 THEN a.sequence_id ELSE -a.sequence_id END \
          LIMIT $9"
    ))
    .bind(now)
    .bind(view as i32)
    .bind(player_id)
    .bind(&pattern)
    .bind(min_tc)
    .bind(max_tc)
    .bind(opts.sequence_id)
    .bind(forward)
    .bind(SEARCH_PAGE_ROWS)
    .fetch_all(pool)
    .await?;

    // A backward page was read nearest-first; the page itself is sent in
    // ascending order.
    let mut rows: Vec<(AuctionRow, String)> = rows
        .into_iter()
        .map(|r| (r.auction, r.seller_name.unwrap_or_default()))
        .collect();
    if !forward {
        rows.reverse();
    }
    debug_assert!(rows.iter().all(|(r, _)| r.status == auction_status::ACTIVE));
    Ok(Ok(SearchPage { view, rows, total }))
}

/// Handle a `BMSearch` forwarded from the cell.
#[tracing::instrument(
    name = "black_market.search",
    level = "info",
    skip_all,
    fields(entity_id, account_id = tracing::field::Empty, player_id)
)]
pub async fn handle_search(
    entity_id: u32,
    player_id: i32,
    options: BMSearchOptions,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let actor = Actor::resolve(entity_id, player_id, connected, entity_to_addr);
    let net = BmNet {
        transport,
        connected,
        entity_to_addr,
    };
    let Some(pool) = db_pool else {
        let f = Failure::Refused(BMError::BMUnavailable);
        log_failure("search", &actor, None, &f);
        send_bm_error(net, entity_id, f.error()).await;
        return;
    };

    let now = now_unix_secs();
    let page = match run_search(pool, player_id, &options, now).await {
        Ok(Ok(page)) => page,
        Ok(Err(e)) => {
            log_failure("search", &actor, None, &Failure::Refused(e));
            send_bm_error(net, entity_id, e).await;
            return;
        }
        Err(e) => {
            let f = Failure::Db {
                stage: "search",
                error: e.to_string(),
            };
            log_failure("search", &actor, None, &f);
            send_bm_error(net, entity_id, f.error()).await;
            return;
        }
    };

    // Backward pages keep the rows nearest the cursor, which are last.
    let rows: &[(AuctionRow, String)] = if options.b_forward == 0 && options.sequence_id > 0 {
        let fit = super::wire::rows_that_fit(page.rows.iter().rev(), now);
        &page.rows[page.rows.len() - fit..]
    } else {
        &page.rows
    };
    let total = page.total.min(i64::from(i32::MAX)) as i32;
    let (args, sent) = serialize_on_bm_auctions(rows, total, options.client_key, now);
    tracing::info!(
        event = "bm.search",
        entity_id,
        account_id = actor.account_id,
        player_id,
        client_key = options.client_key,
        view = ?page.view,
        item_name = %options.item_name,
        min_tc = options.min_tc,
        max_tc = options.max_tc,
        quality = options.quality,
        sort_id = options.sort_id,
        filter_flags = options.filter_flags,
        cursor = options.sequence_id,
        forward = options.b_forward != 0,
        rows_read = page.rows.len(),
        rows_returned = sent,
        total_results = total,
        "search: returning listings"
    );
    count_bm_outcome("search", "ok");
    send_bm_auctions(net, entity_id, &args, sent).await;
}
