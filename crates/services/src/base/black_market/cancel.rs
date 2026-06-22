//! `cancelAuction` — seller reclaim: return the escrowed item, refund the
//! current bidder, mark the auction cancelled.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::helpers::{adjust_player_cash, return_item, CashError};
use super::send::{send_bm_auction_remove, send_bm_error};
use super::types::{auction_status, AuctionRow};
use super::validate::validate_cancel;
use super::wire::error_code;
use crate::base::ConnectedClientState;

/// Handle a `BMCancelAuction` forwarded from the cell.
#[tracing::instrument(
    name = "black_market.cancel_auction",
    level = "info",
    skip_all,
    fields(entity_id, player_id, sequence_id)
)]
pub async fn handle_cancel_auction(
    entity_id: u32,
    player_id: i32,
    sequence_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        tracing::debug!(entity_id, player_id, "cancelAuction: no DB pool");
        return;
    };

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(entity_id, player_id, "cancelAuction: begin tx failed: {e}");
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

    let auction = match sqlx::query_as::<_, AuctionRow>(
        "SELECT sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
                charges, starting_price, buyout_price, current_bid, current_bidder, \
                auction_length, created_at, expires_at, status \
         FROM sgw_auction WHERE sequence_id = $1 FOR UPDATE",
    )
    .bind(sequence_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(a)) => a,
        Ok(None) => {
            let _ = tx.rollback().await;
            send_bm_error(
                entity_id,
                error_code::AUCTION_GONE,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(entity_id, sequence_id, "cancelAuction: fetch failed: {e}");
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

    if let Err(code) = validate_cancel(&auction, player_id) {
        let _ = tx.rollback().await;
        tracing::info!(
            entity_id,
            player_id,
            sequence_id,
            code,
            "cancelAuction: rejected"
        );
        send_bm_error(entity_id, code, transport, connected, entity_to_addr).await;
        return;
    }

    // Return the escrowed item to the seller.
    if let Err(e) = return_item(
        &mut *tx,
        auction.seller_id,
        auction.item_def_id,
        auction.stack_size,
        auction.durability,
        auction.charges,
    )
    .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            entity_id,
            sequence_id,
            "cancelAuction: item return failed: {e}"
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

    // Refund the current bidder, if any.
    if let Some(bidder) = auction.current_bidder {
        if auction.current_bid > 0 {
            match adjust_player_cash(&mut tx, bidder, auction.current_bid as i64).await {
                Ok(_) => {}
                Err(CashError::NoSuchPlayer) => {
                    tracing::warn!(
                        sequence_id,
                        bidder,
                        amount = auction.current_bid,
                        "cancelAuction: bidder row missing, cannot refund"
                    );
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    tracing::error!(sequence_id, bidder, "cancelAuction: refund failed: {e}");
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
            }
        }
    }

    if let Err(e) = sqlx::query("UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2")
        .bind(auction_status::CANCELLED)
        .bind(sequence_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            entity_id,
            sequence_id,
            "cancelAuction: status update failed: {e}"
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

    if let Err(e) = tx.commit().await {
        tracing::error!(entity_id, sequence_id, "cancelAuction: commit failed: {e}");
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
        sequence_id,
        "cancelAuction: cancelled"
    );
    send_bm_auction_remove(entity_id, sequence_id, transport, connected, entity_to_addr).await;
}
