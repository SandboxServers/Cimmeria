//! Expiry sweep — a periodic background task that settles auctions whose
//! `expires_at` has passed.
//!
//! - **Sold** (a current bidder exists): the seller is mailed the winning cash
//!   and the buyer is mailed the item; status → `SOLD`.
//! - **Unsold** (no bidder): the escrowed item is mailed back to the seller;
//!   status → `EXPIRED`.
//!
//! Settlement runs in one transaction per auction so a crash mid-settlement
//! can't double-deliver. Each settled auction's `sequence_id` is returned so the
//! caller can push `onBMAuctionRemove` to any interested online clients.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::helpers::{now_unix_secs, send_mail_to_player};
use super::send::send_bm_auction_remove;
use super::types::{auction_status, AuctionRow};
use crate::base::ConnectedClientState;

/// How often the expiry sweep runs.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// Mail subject/body for a sold auction's seller payout.
const SOLD_SELLER_SUBJECT: &str = "Auction Sold";
const SOLD_SELLER_BODY: &str = "Your Black Market auction sold. The winning bid is attached.";
/// Mail subject/body for a sold auction's buyer delivery.
const SOLD_BUYER_SUBJECT: &str = "Auction Won";
const SOLD_BUYER_BODY: &str = "You won a Black Market auction. Your item is attached.";
/// Mail subject/body for an unsold auction returned to the seller.
const UNSOLD_SUBJECT: &str = "Auction Expired";
const UNSOLD_BODY: &str = "Your Black Market auction expired with no bids. Your item is returned.";
/// Mail `sender_name` used for all system-generated auction mail.
const BM_SENDER_NAME: &str = "Black Market";

/// One settled auction's outcome, for caller-side notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledAuction {
    pub sequence_id: i32,
    pub seller_id: i32,
    pub buyer_id: Option<i32>,
    pub sold: bool,
}

/// Settle every auction that is `ACTIVE` and past its `expires_at`. Returns the
/// list of settled auctions. Idempotent: an already-settled auction (status not
/// `ACTIVE`) is skipped because the status guard is part of the UPDATE.
///
/// This is the unit of work the background loop runs; it is also called directly
/// by the live-DB sweep test, so it carries no transport state.
pub async fn settle_expired_once(pool: &PgPool) -> Result<Vec<SettledAuction>, sqlx::Error> {
    let now = now_unix_secs();

    // Snapshot the due auctions up front. We re-lock each row inside its own
    // transaction before mutating, so a row that another worker settles between
    // the snapshot and the lock is harmlessly skipped by the status guard.
    let due = sqlx::query_as::<_, AuctionRow>(
        "SELECT sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
                charges, starting_price, buyout_price, current_bid, current_bidder, \
                auction_length, created_at, expires_at, status \
         FROM sgw_auction WHERE status = $1 AND expires_at <= $2",
    )
    .bind(auction_status::ACTIVE)
    .bind(now)
    .fetch_all(pool)
    .await?;

    let mut settled = Vec::new();
    for auction in due {
        if let Some(s) = settle_one(pool, &auction).await? {
            settled.push(s);
        }
    }
    Ok(settled)
}

/// Settle a single auction in its own transaction. Returns `Ok(None)` if the
/// row was no longer `ACTIVE` when re-locked (someone else settled it).
async fn settle_one(
    pool: &PgPool,
    auction: &AuctionRow,
) -> Result<Option<SettledAuction>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Re-lock and re-read the live row so concurrent sweeps don't double-settle
    // and so the sold/unsold decision uses the post-lock current_bid /
    // current_bidder (not the pre-lock snapshot which may be stale).
    let locked = sqlx::query_as::<_, AuctionRow>(
        "SELECT sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
                charges, starting_price, buyout_price, current_bid, current_bidder, \
                auction_length, created_at, expires_at, status \
         FROM sgw_auction WHERE sequence_id = $1 FOR UPDATE",
    )
    .bind(auction.sequence_id)
    .fetch_optional(&mut *tx)
    .await?;
    let live = match locked {
        Some(row) if row.status == auction_status::ACTIVE => row,
        _ => {
            tx.rollback().await?;
            return Ok(None);
        }
    };

    let sold = live.current_bidder.is_some() && live.current_bid > 0;

    if let Some(buyer_id) = live.current_bidder.filter(|_| sold) {
        // Sold: seller gets cash mail, buyer gets item mail.
        send_mail_to_player(
            &mut *tx,
            live.seller_id,
            live.current_bid as i64,
            None,
            0,
            SOLD_SELLER_SUBJECT,
            SOLD_SELLER_BODY,
            BM_SENDER_NAME,
        )
        .await?;
        // The buyer's held cash was already deducted at bid time; deliver the
        // item by re-materialising the instance and attaching it to the mail.
        let item_id = super::helpers::return_item(
            &mut *tx,
            buyer_id,
            live.item_def_id,
            live.stack_size,
            live.durability,
            live.charges,
        )
        .await?;
        send_mail_to_player(
            &mut *tx,
            buyer_id,
            0,
            Some(item_id),
            0,
            SOLD_BUYER_SUBJECT,
            SOLD_BUYER_BODY,
            BM_SENDER_NAME,
        )
        .await?;

        sqlx::query("UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2")
            .bind(auction_status::SOLD)
            .bind(live.sequence_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(Some(SettledAuction {
            sequence_id: live.sequence_id,
            seller_id: live.seller_id,
            buyer_id: Some(buyer_id),
            sold: true,
        }))
    } else {
        // Unsold: mail the escrowed item back to the seller.
        let item_id = super::helpers::return_item(
            &mut *tx,
            live.seller_id,
            live.item_def_id,
            live.stack_size,
            live.durability,
            live.charges,
        )
        .await?;
        send_mail_to_player(
            &mut *tx,
            live.seller_id,
            0,
            Some(item_id),
            0,
            UNSOLD_SUBJECT,
            UNSOLD_BODY,
            BM_SENDER_NAME,
        )
        .await?;

        sqlx::query("UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2")
            .bind(auction_status::EXPIRED)
            .bind(live.sequence_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(Some(SettledAuction {
            sequence_id: live.sequence_id,
            seller_id: live.seller_id,
            buyer_id: None,
            sold: false,
        }))
    }
}

/// Push `onBMAuctionRemove` to the seller and buyer (when online) for each
/// settled auction. The auction left the active set, so any client showing it
/// must drop the row.
async fn notify_settled(
    settled: &[SettledAuction],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    for s in settled {
        let mut targets = vec![s.seller_id];
        if let Some(b) = s.buyer_id {
            targets.push(b);
        }
        for player_id in targets {
            if let Some(eid) = super::entity_id_for_player_id(player_id, connected) {
                send_bm_auction_remove(eid, s.sequence_id, transport, connected, entity_to_addr)
                    .await;
            }
        }
    }
}

/// Spawn the periodic expiry sweep background task.
///
/// Mirrors the outbox drainer pattern: a `tokio::spawn` with a startup pass
/// (settle anything already due from before this process started) followed by
/// an interval ticker.
pub fn spawn_sweep(
    pool: Arc<PgPool>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tokio::spawn(async move {
        tracing::debug!("black_market expiry sweep started");
        run_sweep_pass(&pool, &transport, &connected, &entity_to_addr).await;
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        ticker.tick().await; // skip the immediate first tick — handled above
        loop {
            ticker.tick().await;
            run_sweep_pass(&pool, &transport, &connected, &entity_to_addr).await;
        }
    });
}

async fn run_sweep_pass(
    pool: &PgPool,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    match settle_expired_once(pool).await {
        Ok(settled) if !settled.is_empty() => {
            tracing::info!(
                count = settled.len(),
                "black_market: settled expired auctions"
            );
            notify_settled(&settled, transport, connected, entity_to_addr).await;
        }
        Ok(_) => {}
        Err(e) => tracing::warn!("black_market: sweep failed: {e}"),
    }
}
