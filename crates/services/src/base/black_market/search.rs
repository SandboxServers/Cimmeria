//! `search` — query active listings and reply with `onBMAuctions`.
//!
//! Phase 1: returns **all** active listings (status = 0) ordered by
//! `sequence_id`. The `BMSearchOptions` fields (name filters, price range,
//! quality, pagination cursor) are parsed and forwarded but not yet applied as
//! SQL predicates — the full filter pass is deferred until the client-side
//! result shape is confirmed via x64dbg (pending D.7).
//!
//! The `view` field echoed in the reply is the `sort_id` byte from the search
//! request, promoted to `i32` — the client's pagination UI uses it to decide
//! which column header is highlighted.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::player_name_for_player_id;
use super::send::send_bm_auctions;
use super::types::{auction_status, AuctionRow, BMSearchOptions};
use crate::base::ConnectedClientState;

/// Fetch all globally active auction rows ordered by `sequence_id`.
///
/// This is the DB query seam used by `handle_search` and exposed to tests so
/// integration tests can assert on the exact rows (count, `item_def_id`,
/// `sequence_id`, absence of non-active rows) without decoding an encrypted
/// Mercury packet.
///
/// Tests call this function directly on the same pool they seeded, letting
/// them assert the real `WHERE status = ACTIVE` result set rather than a
/// seller-scoped COUNT precondition.
pub(crate) async fn fetch_active_auctions(pool: &PgPool) -> Result<Vec<AuctionRow>, sqlx::Error> {
    sqlx::query_as::<_, AuctionRow>(
        "SELECT sequence_id, seller_id, item_id, item_def_id, stack_size, \
                durability, charges, starting_price, buyout_price, current_bid, \
                current_bidder, auction_length, created_at, expires_at, status \
         FROM sgw_auction \
         WHERE status = $1 \
         ORDER BY sequence_id",
    )
    .bind(auction_status::ACTIVE)
    .fetch_all(pool)
    .await
}

/// Handle a `BMSearch` forwarded from the cell.
///
/// Queries `sgw_auction` for all `status = ACTIVE` rows via
/// [`fetch_active_auctions`], resolves each seller's display name (online
/// players resolved from the session map; offline sellers get an empty name as
/// a cosmetic placeholder), then sends `onBMAuctions` back to the requesting
/// entity.
///
/// The `options` fields are available for future filter predicates (Phase 2).
/// For now only `sort_id` is used — it is echoed back as the `view` argument
/// so the client's sort-column highlight tracks correctly.
#[tracing::instrument(
    name = "black_market.search",
    level = "info",
    skip_all,
    fields(entity_id, player_id)
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
    let Some(pool) = db_pool else {
        tracing::debug!(entity_id, player_id, "search: no DB pool");
        return;
    };

    let rows = match fetch_active_auctions(pool).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(entity_id, player_id, "search: query failed: {e}");
            return;
        }
    };

    let total = rows.len() as i32;
    tracing::info!(
        entity_id,
        player_id,
        total,
        "search: returning active listings"
    );

    // Resolve seller display names. Online players resolve from the session map;
    // offline sellers fall back to an empty string (cosmetic only — the client
    // shows the name field but does not gate behaviour on it).
    let pairs: Vec<(AuctionRow, String)> = rows
        .into_iter()
        .map(|row| {
            let name = player_name_for_player_id(row.seller_id, connected, entity_to_addr)
                .unwrap_or_default();
            (row, name)
        })
        .collect();

    // Echo sort_id as the view index (the client uses this to keep the sort
    // column highlighted across a search round-trip).
    let view = options.sort_id as i32;

    send_bm_auctions(
        entity_id,
        &pairs,
        view,
        total,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}
