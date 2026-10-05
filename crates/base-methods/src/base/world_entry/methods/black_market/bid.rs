//! `placeBid`: mail the outbid player their held bid, hold the new bid,
//! advance the auction, and settle it at once on a buyout (decision D8).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::escrow::{escrowed_item, lock_escrow};
use super::helpers::{adjust_player_cash, lock_players, now_unix_secs, CashError};
use super::payout_mail::{refund_standing_bid, Payout, PayoutReason};
use super::player_name;
use super::send::{send_bm_auction_remove, send_bm_auction_update, send_bm_error, BmNet};
use super::settle::{settle_locked, SettleCause, SettleError, SettledAuction};
use super::sweep::notify_settled;
use super::telemetry::{
    count_bm_outcome, db, item_name, log_failure, log_outbid_refund, log_transition, Actor, Failure,
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
    /// The mail that gave the outbid player their held cash back.
    pub refunded: Option<Payout>,
    /// The bid was a buyout and settled the auction.
    pub settled: Option<SettledAuction>,
}

/// Handle a `BMPlaceBid` forwarded from the cell.
///
/// All cash movement, the refund mail, the auction update and (on a
/// buyout) the settlement happen in one transaction, so a crash can't
/// strand the bidder's held funds or pay twice.
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

    if let Some(p) = &done.refunded {
        log_outbid_refund(
            &actor,
            &done.before,
            p.mail.recipient_player_id,
            done.before.current_bid,
        );
        p.log(&actor.who());
    }
    log_transition("bm.bid", &actor.who(), Some(&done.before), &done.after);
    tracing::info!(
        entity_id,
        entity_name = actor.player_name,
        account_id = actor.account_id,
        account_name = actor.account_name,
        player_id,
        player_name = actor.player_name,
        auction_id = sequence_id, // nt:id-only auctions have no name column; item_name names the listing
        item_name = item_name(done.after.item_def_id),
        bid_amount,
        charged = done.after.current_bid,
        buyout = done.settled.is_some(),
        "placeBid: bid accepted"
    );
    let outbid_entity = done
        .refunded
        .as_ref()
        .and_then(|p| net.entity_of(p.mail.recipient_player_id));
    if let Some(p) = &done.refunded {
        p.notify(pool, &net.feedback()).await;
    }

    if let Some(settled) = &done.settled {
        log_transition(
            "bm.sold",
            &actor.who(),
            Some(&settled.before),
            &settled.after,
        );
        for p in &settled.payouts {
            p.log(&actor.who());
        }
        count_bm_outcome("bid", "buyout");
        notify_settled(net, pool, settled).await;
        // The outbid player's My Bids row is gone too.
        if let Some(eid) = outbid_entity {
            send_bm_auction_remove(net, eid, sequence_id).await;
        }
        return;
    }
    count_bm_outcome("bid", "ok");

    let name = player_name(pool, done.after.seller_id).await;
    send_bm_auction_update(net, entity_id, &done.after, &name, now).await;
    // The outbid player's My Bids row changes too.
    if let Some(eid) = outbid_entity {
        send_bm_auction_update(net, eid, &done.after, &name, now).await;
    }
}

/// The bid transaction. Lock order: the auction row, then (on a buyout)
/// the seller's escrow advisory locks and the escrowed item row, then the
/// bidder's, the outbid player's and (on a buyout) the seller's
/// `sgw_player` rows in ascending `player_id`, then the mail writer.
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
        // The item goes by mail, so the buyer needs no bag space; the
        // escrow is locked here, before any player row, and a missing row
        // (`bm.escrow_missing`) refuses the buyout before anything is charged.
        lock_escrow(&mut tx, auction.seller_id)
            .await
            .map_err(db("lock_escrow"))?;
        escrowed_item(&mut tx, &auction)
            .await
            .map_err(db("escrow_row"))?
            .ok_or(BMError::Internal)?;
    }

    let self_raise = auction.current_bidder == Some(bidder);
    let mut players = vec![bidder];
    players.extend(auction.current_bidder);
    if buyout {
        players.push(auction.seller_id);
    }
    lock_players(&mut tx, &players)
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

    // Raising your own standing bid counts the held bid toward the new one.
    let held = if self_raise {
        auction.escrowed_cash()
    } else {
        0
    };
    validate_bid(&auction, bidder, bid_amount, balance + held, now)?;
    let charge = if buyout {
        auction.buyout_price
    } else {
        bid_amount
    };

    // Someone else's held bid goes back to them by mail. A self-raise has
    // nothing to mail: only the difference is charged below.
    let refunded = if self_raise {
        None
    } else {
        refund_standing_bid(&mut tx, &auction, PayoutReason::Outbid)
            .await
            .map_err(|e| Failure::Db {
                stage: "refund",
                error: format!("{} ({e})", e.reason()),
            })?
    };
    match adjust_player_cash(&mut tx, bidder, held - i64::from(charge)).await {
        Ok(_) => {}
        Err(CashError::InsufficientFunds | CashError::NoSuchPlayer) => {
            return Err(BMError::NotEnoughFunds.into())
        }
        Err(e) => {
            return Err(Failure::Db {
                stage: "hold",
                error: e.to_string(),
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
            .map_err(|e| match e {
                // The row is locked and ACTIVE and its escrow was checked
                // above: this cannot happen short of a bug, and paying
                // without the status gate would be worse.
                SettleError::Gone | SettleError::EscrowMissing => {
                    Failure::Refused(BMError::Internal)
                }
                other => Failure::Db {
                    stage: "settle",
                    error: format!("{} ({other})", other.reason()),
                },
            })?;
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
