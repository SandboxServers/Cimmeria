//! `placeBid`: refund the prior bidder, hold the new bid, advance the
//! auction, and settle it at once on a buyout (decision D8).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::escrow::{free_bag_slot, lock_for_delivery};
use super::helpers::{adjust_player_cash, now_unix_secs, CashError};
use super::player_name;
use super::send::{send_bm_auction_update, send_bm_error, BmNet};
use super::settle::{settle_locked, SettleCause, SettledAuction};
use super::sweep::notify_settled;
use super::telemetry::{
    count_bm_outcome, db, log_failure, log_outbid_refund, log_transition, Actor, Failure,
};
use super::types::{auction_columns, AuctionRow};
use super::validate::{is_buyout, is_open, validate_bid};
use super::wire::BMError;
use crate::base::ConnectedClientState;

/// A committed bid.
#[derive(Debug)]
pub(super) struct BidDone {
    pub before: AuctionRow,
    pub after: AuctionRow,
    /// The outbid player and the held cash they got back.
    pub refunded: Option<(i32, i32)>,
    /// The bid was a buyout and settled the auction.
    pub settled: Option<SettledAuction>,
}

/// Handle a `BMPlaceBid` forwarded from the cell.
///
/// All cash movement, the auction update and (on a buyout) the settlement
/// happen in one transaction, so a crash can't strand the bidder's held
/// funds or pay twice.
#[tracing::instrument(
    name = "black_market.place_bid",
    level = "info",
    skip_all,
    fields(entity_id, account_id = tracing::field::Empty, player_id, sequence_id, bid_amount)
)]
pub async fn handle_place_bid(
    entity_id: u32,
    player_id: i32,
    sequence_id: i32,
    bid_amount: i32,
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
        log_failure("bid", &actor, Some(sequence_id), &f);
        send_bm_error(net, entity_id, f.error()).await;
        return;
    };

    let now = now_unix_secs();
    let done = match place_bid(pool, &actor, sequence_id, bid_amount, now).await {
        Ok(done) => done,
        Err(f) => {
            log_failure("bid", &actor, Some(sequence_id), &f);
            send_bm_error(net, entity_id, f.error()).await;
            return;
        }
    };

    if let Some((to, amount)) = done.refunded {
        log_outbid_refund(&actor, &done.before, to, amount);
    }
    log_transition(
        "bm.bid",
        actor.account_id,
        player_id,
        Some(&done.before),
        &done.after,
    );
    tracing::info!(
        entity_id,
        account_id = actor.account_id,
        player_id,
        sequence_id,
        bid_amount,
        charged = done.after.current_bid,
        buyout = done.settled.is_some(),
        "placeBid: bid accepted"
    );

    if let Some(settled) = &done.settled {
        log_transition(
            "bm.sold",
            actor.account_id,
            player_id,
            Some(&settled.before),
            &settled.after,
        );
        count_bm_outcome("bid", "buyout");
        notify_settled(net, pool, settled).await;
        return;
    }
    count_bm_outcome("bid", "ok");

    let name = player_name(pool, done.after.seller_id).await;
    send_bm_auction_update(net, entity_id, &done.after, &name, now).await;
    // The outbid player's My Bids row changes too.
    if let Some(eid) = done
        .refunded
        .and_then(|(to, _)| (to != player_id).then_some(to))
        .and_then(|to| net.entity_of(to))
    {
        send_bm_auction_update(net, eid, &done.after, &name, now).await;
    }
}

/// The bid transaction. Lock order: the auction row, then (on a buyout) the
/// seller's escrow and the buyer's bags' advisory locks, then the two
/// bidders' `sgw_player` rows in ascending `player_id`, then the escrowed
/// item row inside the settlement.
pub(super) async fn place_bid(
    pool: &PgPool,
    actor: &Actor,
    sequence_id: i32,
    bid_amount: i32,
    now: i32,
) -> Result<BidDone, Failure> {
    let bidder = actor.player_id;
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

    // Refusals that need no balance come first, so a closed auction or a
    // seller's own bid takes no inventory locks.
    if !is_open(&auction, now) {
        return Err(BMError::AuctionGone.into());
    }
    if auction.seller_id == bidder {
        return Err(BMError::IsSeller.into());
    }
    let buyout = is_buyout(&auction, bid_amount);
    if buyout {
        // The buyer must have room before anything is charged.
        lock_for_delivery(&mut tx, auction.seller_id, bidder)
            .await
            .map_err(db("lock_delivery"))?;
        if free_bag_slot(&mut tx, bidder)
            .await
            .map_err(db("free_slot"))?
            .is_none()
        {
            return Err(BMError::BagFull.into());
        }
    }

    let mut players = vec![bidder];
    players.extend(auction.current_bidder.filter(|&p| p != bidder));
    players.sort_unstable();
    sqlx::query("SELECT 1 FROM sgw_player WHERE player_id = ANY($1) ORDER BY player_id FOR UPDATE")
        .bind(&players)
        .execute(&mut *tx)
        .await
        .map_err(db("lock_players"))?;
    let balance: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT naquadah::bigint FROM sgw_player WHERE player_id = $1",
    )
    .bind(bidder)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db("balance"))?
    .ok_or(BMError::NotEnoughFunds)?;

    // Raising your own standing bid refunds it first, so it counts.
    let effective = if auction.current_bidder == Some(bidder) {
        balance + auction.escrowed_cash()
    } else {
        balance
    };
    validate_bid(&auction, bidder, bid_amount, effective, now)?;
    let charge = if buyout {
        auction.buyout_price
    } else {
        bid_amount
    };

    let mut refunded = None;
    if let Some(prev) = auction.current_bidder.filter(|_| auction.current_bid > 0) {
        match adjust_player_cash(&mut tx, prev, i64::from(auction.current_bid)).await {
            Ok(_) => refunded = Some((prev, auction.current_bid)),
            // The prior bidder's character is gone; their held cash went
            // with it. The delete trigger normally clears such a bid.
            Err(CashError::NoSuchPlayer) => tracing::warn!(
                sequence_id,
                prev_bidder = prev,
                amount = auction.current_bid,
                reason = "prior_bidder_missing",
                "placeBid: prior bidder row missing, cannot refund"
            ),
            Err(e) => {
                return Err(Failure::Db {
                    stage: "refund",
                    error: e.to_string(),
                })
            }
        }
    }
    match adjust_player_cash(&mut tx, bidder, -i64::from(charge)).await {
        Ok(_) => {}
        Err(CashError::InsufficientFunds | CashError::NoSuchPlayer) => {
            return Err(BMError::NotEnoughFunds.into())
        }
        Err(CashError::Db(e)) => {
            return Err(Failure::Db {
                stage: "hold",
                error: e,
            })
        }
    }

    let after: AuctionRow = sqlx::query_as(concat!(
        "UPDATE sgw_auction SET current_bid = $1, current_bidder = $2 WHERE sequence_id = $3 \
         RETURNING ",
        auction_columns!()
    ))
    .bind(charge)
    .bind(bidder)
    .bind(sequence_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db("update_auction"))?;
    sqlx::query(
        "INSERT INTO sgw_auction_bid (sequence_id, bidder_id, amount, created_at) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(sequence_id)
    .bind(bidder)
    .bind(i64::from(charge))
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(db("bid_history"))?;

    let settled = if buyout {
        let s = settle_locked(&mut tx, &after, SettleCause::Buyout)
            .await
            .map_err(db("settle"))?
            // The row is locked and ACTIVE: the conditional write cannot
            // miss short of a bug, and paying without it would be worse.
            .ok_or(BMError::Internal)?;
        Some(s)
    } else {
        None
    };

    tx.commit().await.map_err(db("commit"))?;
    Ok(BidDone {
        before: auction,
        after,
        refunded,
        settled,
    })
}
