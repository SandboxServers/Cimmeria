//! `createAuction`: move an item into escrow (container 18) and open a
//! listing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::escrow::{list_into_escrow, EscrowedItem};
use super::helpers::now_unix_secs;
use super::player_name;
use super::send::{send_bm_auction_update, send_bm_error, send_item_removed, BmNet};
use super::telemetry::{count_bm_outcome, db, log_failure, log_transition, Actor, Failure};
use super::types::{auction_columns, auction_status, AuctionRow};
use super::validate::{validate_listing_cap, validate_prices};
use super::wire::{auction_length_seconds, clamp_auction_length, BMError};
use crate::base::ConnectedClientState;

/// What the client asked to list.
#[derive(Debug, Clone, Copy)]
pub struct CreateRequest {
    pub item_id: i32,
    pub starting_price: i32,
    pub buyout_price: i32,
    /// The raw `auctionLength` byte (1-based `UIAuctionTime`).
    pub auction_length: u8,
}

/// Handle a `BMCreateAuction` forwarded from the cell.
///
/// Checks the prices, moves the item row into the seller's container 18,
/// enforces the listing cap, and inserts the `sgw_auction` row, all in one
/// transaction. Replies `onRemoveItem` (the item left the bags) and
/// `onBMAuctionUpdate`, or `onBMError`.
#[tracing::instrument(
    name = "black_market.create_auction",
    level = "info",
    skip_all,
    fields(entity_id, account_id = tracing::field::Empty, player_id, item_id)
)]
pub async fn handle_create_auction(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    starting_price: i32,
    buyout_price: i32,
    auction_length: u8,
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
        log_failure("create", &actor, None, &f);
        send_bm_error(net, entity_id, f.error()).await;
        return;
    };
    let req = CreateRequest {
        item_id,
        starting_price,
        buyout_price,
        auction_length,
    };

    let now = now_unix_secs();
    match create_listing(pool, &actor, req, now).await {
        Ok((row, item)) => {
            log_transition("bm.listed", actor.account_id, player_id, None, &row);
            tracing::info!(
                entity_id,
                account_id = actor.account_id,
                player_id,
                sequence_id = row.sequence_id,
                item_id,
                from_container = item.container_id,
                expires_at = row.expires_at,
                "createAuction: listing opened"
            );
            count_bm_outcome("create", "ok");
            send_item_removed(net, entity_id, item.item_id).await;
            let name = player_name(pool, player_id).await;
            send_bm_auction_update(net, entity_id, &row, &name, now).await;
        }
        Err(f) => {
            log_failure("create", &actor, None, &f);
            send_bm_error(net, entity_id, f.error()).await;
        }
    }
}

/// The listing transaction. Lock order: the seller's inventory advisory
/// locks, the item row, then the seller's `sgw_player` row, which also
/// serializes two creates racing for the last listing slot (D5).
pub(super) async fn create_listing(
    pool: &PgPool,
    actor: &Actor,
    req: CreateRequest,
    now: i32,
) -> Result<(AuctionRow, EscrowedItem), Failure> {
    validate_prices(req.starting_price, req.buyout_price)?;
    let (tier, clamped) = clamp_auction_length(req.auction_length);
    if clamped {
        tracing::info!(
            event = "bm.length_clamped",
            account_id = actor.account_id,
            player_id = actor.player_id,
            raw = req.auction_length,
            tier = tier as u8,
            "createAuction: auctionLength out of range, clamped to the nearest tier"
        );
    }

    let mut tx = pool.begin().await.map_err(db("begin"))?;
    let item = list_into_escrow(&mut tx, actor.player_id, req.item_id)
        .await
        .map_err(db("escrow"))??;

    sqlx::query("SELECT 1 FROM sgw_player WHERE player_id = $1 FOR UPDATE")
        .bind(actor.player_id)
        .execute(&mut *tx)
        .await
        .map_err(db("lock_seller"))?;
    let active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1 AND status = $2")
            .bind(actor.player_id)
            .bind(auction_status::ACTIVE)
            .fetch_one(&mut *tx)
            .await
            .map_err(db("count_listings"))?;
    validate_listing_cap(active)?;

    let expires_at =
        (i64::from(now) + auction_length_seconds(tier)).min(i64::from(i32::MAX)) as i32;
    let row: AuctionRow = sqlx::query_as(concat!(
        "INSERT INTO sgw_auction \
            (seller_id, item_id, item_def_id, stack_size, durability, charges, \
             starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, NULL, $9, $10, $11, $12) \
         RETURNING ",
        auction_columns!()
    ))
    .bind(actor.player_id)
    .bind(item.item_id)
    .bind(item.item_def_id)
    .bind(item.stack_size)
    .bind(item.durability)
    .bind(item.charges)
    .bind(req.starting_price)
    .bind(req.buyout_price)
    .bind(i16::from(tier as u8))
    .bind(now)
    .bind(expires_at)
    .bind(auction_status::ACTIVE)
    .fetch_one(&mut *tx)
    .await
    .map_err(db("insert"))?;

    tx.commit().await.map_err(db("commit"))?;
    Ok((row, item))
}
