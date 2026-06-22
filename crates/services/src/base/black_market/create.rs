//! `createAuction` — escrow an item out of inventory and open a listing.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::helpers::{escrow_item, now_unix_secs};
use super::player_name_for_entity;
use super::send::{send_bm_auction_update, send_bm_error};
use super::types::{auction_status, AuctionRow};
use super::validate::validate_create;
use super::wire::{auction_length_seconds, error_code};
use crate::base::ConnectedClientState;

/// Handle a `BMCreateAuction` forwarded from the cell.
///
/// Validates ownership by attempting to escrow (DELETE) the item instance,
/// computes `expires_at` from the duration enum, inserts the `sgw_auction`
/// row inside the same transaction, and replies with `onBMAuctionUpdate`
/// (or `onBMError` on failure).
#[tracing::instrument(
    name = "black_market.create_auction",
    level = "info",
    skip_all,
    fields(entity_id, player_id, item_id)
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
    let Some(pool) = db_pool else {
        tracing::debug!(entity_id, player_id, "createAuction: no DB pool");
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(entity_id, player_id, "createAuction: begin tx failed: {e}");
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    // Escrow = DELETE the instance out of inventory. A matched row both proves
    // ownership and hands back the item snapshot to record on the auction.
    let escrowed = match escrow_item(&mut *tx, player_id, item_id).await {
        Ok(opt) => opt,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                item_id,
                "createAuction: escrow failed: {e}"
            );
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    if let Err(code) = validate_create(escrowed.is_some(), starting_price, buyout_price) {
        let _ = tx.rollback().await;
        tracing::info!(
            entity_id,
            player_id,
            item_id,
            code,
            "createAuction: rejected"
        );
        send_bm_error(entity_id, code, transport, connected, entity_to_addr).await;
        return;
    }
    let item = escrowed.expect("validated Some");

    let now = now_unix_secs();
    let expires_at = (now as i64).saturating_add(auction_length_seconds(auction_length));
    let expires_at = expires_at.min(i32::MAX as i64) as i32;

    let row = match sqlx::query_as::<_, AuctionRow>(
        "INSERT INTO sgw_auction \
            (seller_id, item_id, item_def_id, stack_size, durability, charges, \
             starting_price, buyout_price, current_bid, current_bidder, \
             auction_length, created_at, expires_at, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, NULL, $9, $10, $11, $12) \
         RETURNING sequence_id, seller_id, item_id, item_def_id, stack_size, \
                   durability, charges, starting_price, buyout_price, current_bid, \
                   current_bidder, auction_length, created_at, expires_at, status",
    )
    .bind(player_id)
    .bind(item.item_id)
    .bind(item.item_def_id)
    .bind(item.stack_size)
    .bind(item.durability)
    .bind(item.charges)
    .bind(starting_price)
    .bind(buyout_price)
    .bind(auction_length as i16)
    .bind(now)
    .bind(expires_at)
    .bind(auction_status::ACTIVE)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                entity_id,
                player_id,
                item_id,
                "createAuction: insert failed: {e}"
            );
            send_bm_error(
                entity_id,
                error_code::INTERNAL,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
    };

    if let Err(e) = tx.commit().await {
        tracing::error!(entity_id, player_id, "createAuction: commit failed: {e}");
        send_bm_error(
            entity_id,
            error_code::INTERNAL,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

    tracing::info!(
        entity_id,
        player_id,
        sequence_id = row.sequence_id,
        item_id,
        expires_at,
        "createAuction: listing opened"
    );

    let seller_name = player_name_for_entity(entity_id, connected, entity_to_addr);
    send_bm_auction_update(
        entity_id,
        &row,
        &seller_name,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}
