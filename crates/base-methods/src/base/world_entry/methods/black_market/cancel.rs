//! `cancelAuction`: seller reclaim. The escrowed row is mailed back to the
//! seller out of container 18, the standing bid is mailed back to its
//! bidder, and the auction is marked cancelled (decision D-BM10: cancel
//! returns by mail, like expiry, so a full bag never blocks it).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::mail::SystemMailError;
use super::escrow::{escrowed_item, lock_escrow};
use super::helpers::{lock_players, now_unix_secs};
use super::payout_mail::{mail_payout, refund_standing_bid, Payout, PayoutReason, PayoutRole};
use super::send::{send_bm_auction_remove, send_bm_error, BmNet};
use super::telemetry::{
    count_bm_outcome, db, item_name, log_failure, log_outbid_refund, log_transition, Actor, Failure,
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
    /// The item back to the seller, then the refund to the bidder if any.
    pub payouts: Vec<Payout>,
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
            let refunded = c.payouts.iter().find(|p| p.role == PayoutRole::Bidder);
            if let Some(p) = refunded {
                log_outbid_refund(
                    &actor,
                    &c.before,
                    p.mail.recipient_player_id,
                    c.before.current_bid,
                );
            }
            log_transition("bm.cancelled", &actor.who(), Some(&c.before), &c.after);
            for p in &c.payouts {
                p.log(&actor.who());
            }
            tracing::info!(
                entity_id,
                entity_name = actor.player_name,
                account_id = actor.account_id,
                account_name = actor.account_name,
                player_id,
                player_name = actor.player_name,
                auction_id = sequence_id, // nt:id-only auctions have no name column; item_name names the listing
                returned_item_id = c.after.item_id,
                returned_item_name = item_name(c.after.item_def_id),
                mail_id = c.payouts.first().map(|p| p.mail.mail_id), // nt:id-only mail rows carry no name, only a subject
                "cancelAuction: cancelled, item mailed back"
            );
            count_bm_outcome("cancel", "ok");
            send_bm_auction_remove(net, entity_id, sequence_id).await;
            if let Some(eid) = refunded.and_then(|p| net.entity_of(p.mail.recipient_player_id)) {
                send_bm_auction_remove(net, eid, sequence_id).await;
            }
            let ctx = net.feedback();
            for p in &c.payouts {
                p.notify(pool, &ctx).await;
            }
        }
        Err(f) => {
            log_failure("cancel", &actor, Some(sequence_id), &f);
            send_bm_error(net, entity_id, f.error()).await;
        }
    }
}

/// A refused settlement mail as a request failure. The writer logged the
/// refusal (`mail.system_refused`); the client gets `Internal`.
fn mail_failure(stage: &'static str) -> impl FnOnce(SystemMailError) -> Failure {
    move |e| Failure::Db {
        stage,
        error: format!("{} ({e})", e.reason()),
    }
}

/// The cancel transaction. Lock order: the auction row, the seller's
/// escrow advisory locks and the escrowed row, then the seller's and the
/// bidder's `sgw_player` rows in ascending `player_id`, then the mail
/// writer. The status write comes first, conditional on `ACTIVE`, so the
/// mails can never be written twice for one auction.
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

    lock_escrow(&mut tx, auction.seller_id)
        .await
        .map_err(db("lock_escrow"))?;
    // A missing row is logged `bm.escrow_missing`; nothing is minted.
    let item = escrowed_item(&mut tx, &auction)
        .await
        .map_err(db("escrow_row"))?
        .ok_or(BMError::Internal)?;
    let mut players = vec![auction.seller_id];
    players.extend(auction.current_bidder);
    lock_players(&mut tx, &players)
        .await
        .map_err(db("lock_players"))?;

    let mut payouts = vec![mail_payout(
        &mut tx,
        &auction,
        auction.seller_id,
        PayoutReason::Cancelled,
        PayoutRole::Seller,
        0,
        item,
    )
    .await
    .map_err(mail_failure("return_item"))?];
    payouts.extend(
        refund_standing_bid(&mut tx, &auction, PayoutReason::Cancelled)
            .await
            .map_err(mail_failure("refund"))?,
    );

    tx.commit().await.map_err(db("commit"))?;
    Ok(Cancelled {
        before: auction,
        after,
        payouts,
    })
}
