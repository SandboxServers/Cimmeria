//! `cancelAuction`: seller reclaim. The escrowed row moves from container 18
//! back into the seller's bags, the current bidder is refunded, and the
//! auction is marked cancelled.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::escrow::{deliver_from_escrow, DeliveryRefused, Placed};
use super::helpers::{adjust_player_cash, now_unix_secs, CashError};
use super::send::{send_bm_auction_remove, send_bm_error, send_item_placed, BmNet};
use super::telemetry::{
    count_bm_outcome, db, log_failure, log_outbid_refund, log_transition, Actor, Failure,
};
use super::types::{auction_columns, auction_status, AuctionRow};
use super::validate::validate_cancel;
use super::wire::BMError;
use crate::base::ConnectedClientState;

/// A committed cancellation.
#[derive(Debug)]
pub(super) struct Cancelled {
    pub before: AuctionRow,
    pub after: AuctionRow,
    pub placed: Placed,
    pub refunded: Option<(i32, i32)>,
}

/// Handle a `BMCancelAuction` forwarded from the cell.
#[tracing::instrument(
    name = "black_market.cancel_auction",
    level = "info",
    skip_all,
    fields(entity_id, account_id = tracing::field::Empty, player_id, sequence_id)
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
    let actor = Actor::resolve(entity_id, player_id, connected, entity_to_addr);
    let net = BmNet {
        transport,
        connected,
        entity_to_addr,
    };
    let Some(pool) = db_pool else {
        let f = Failure::Refused(BMError::BMUnavailable);
        log_failure("cancel", &actor, Some(sequence_id), &f);
        send_bm_error(net, entity_id, f.error()).await;
        return;
    };

    match cancel_auction(pool, &actor, sequence_id, now_unix_secs()).await {
        Ok(c) => {
            if let Some((to, amount)) = c.refunded {
                log_outbid_refund(&actor, &c.before, to, amount);
            }
            log_transition(
                "bm.cancelled",
                actor.account_id,
                player_id,
                Some(&c.before),
                &c.after,
            );
            tracing::info!(
                entity_id,
                account_id = actor.account_id,
                player_id,
                sequence_id,
                returned_item_id = c.placed.item_id,
                container_id = c.placed.container_id,
                "cancelAuction: cancelled"
            );
            count_bm_outcome("cancel", "ok");
            send_bm_auction_remove(net, entity_id, sequence_id).await;
            send_item_placed(net, pool, entity_id, player_id, c.placed.item_id).await;
            if let Some(eid) = c.refunded.and_then(|(to, _)| net.entity_of(to)) {
                send_bm_auction_remove(net, eid, sequence_id).await;
            }
        }
        Err(f) => {
            log_failure("cancel", &actor, Some(sequence_id), &f);
            send_bm_error(net, entity_id, f.error()).await;
        }
    }
}

/// The cancel transaction. Lock order: the auction row, the seller's escrow
/// and bag advisory locks and the escrowed row (inside the return), then
/// the bidder's `sgw_player` row for the refund.
pub(super) async fn cancel_auction(
    pool: &PgPool,
    actor: &Actor,
    sequence_id: i32,
    now: i32,
) -> Result<Cancelled, Failure> {
    let mut tx = pool.begin().await.map_err(db("begin"))?;
    let auction: AuctionRow = sqlx::query_as(concat!(
        "SELECT ",
        auction_columns!(),
        " FROM sgw_auction WHERE sequence_id = $1 FOR UPDATE"
    ))
    .bind(sequence_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db("lock_auction"))?
    .ok_or(BMError::AuctionGone)?;
    validate_cancel(&auction, actor.player_id, now)?;

    // Back to the seller's bags; a full bag refuses the cancel, so nothing
    // is left half-done.
    let placed = deliver_from_escrow(&mut tx, &auction, auction.seller_id, false)
        .await
        .map_err(db("return_item"))?
        .map_err(|refused| match refused {
            DeliveryRefused::BagFull => BMError::BagFull,
            DeliveryRefused::EscrowMissing => BMError::Internal,
        })?;

    let mut refunded = None;
    if let Some(bidder) = auction.current_bidder.filter(|_| auction.current_bid > 0) {
        match adjust_player_cash(&mut tx, bidder, i64::from(auction.current_bid)).await {
            Ok(_) => refunded = Some((bidder, auction.current_bid)),
            Err(CashError::NoSuchPlayer) => tracing::warn!(
                sequence_id,
                bidder,
                amount = auction.current_bid,
                reason = "bidder_missing",
                "cancelAuction: bidder row missing, cannot refund"
            ),
            Err(e) => {
                return Err(Failure::Db {
                    stage: "refund",
                    error: e.to_string(),
                })
            }
        }
    }

    let after: AuctionRow = sqlx::query_as(concat!(
        "UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2 AND status = $3 RETURNING ",
        auction_columns!()
    ))
    .bind(auction_status::CANCELLED)
    .bind(sequence_id)
    .bind(auction_status::ACTIVE)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db("status"))?
    .ok_or(BMError::AuctionGone)?;

    tx.commit().await.map_err(db("commit"))?;
    Ok(Cancelled {
        before: auction,
        after,
        placed,
        refunded,
    })
}
