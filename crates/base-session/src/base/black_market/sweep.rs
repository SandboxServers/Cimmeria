//! Expiry sweep — a periodic background task that settles auctions whose
//! `expires_at` has passed, through [`super::settle::settle_locked`].
//!
//! Settlement runs in one transaction per auction so a crash mid-settlement
//! can't double-deliver. Each settled auction is returned so the caller can
//! push `onBMAuctionRemove` (and the item update) to the online parties.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::helpers::now_unix_secs;
use super::send::{send_bm_auction_remove, send_item_placed, BmNet};
use super::settle::{settle_locked, SettleCause};
use super::telemetry::{count_bm_outcome, log_transition};
use super::types::{auction_columns, auction_status, AuctionRow};
use crate::base::ConnectedClientState;

pub use super::settle::SettledAuction;

/// How often the expiry sweep runs.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

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
    let due: Vec<AuctionRow> = sqlx::query_as(concat!(
        "SELECT ",
        auction_columns!(),
        " FROM sgw_auction WHERE status = $1 AND expires_at <= $2"
    ))
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
    let locked: Option<AuctionRow> = sqlx::query_as(concat!(
        "SELECT ",
        auction_columns!(),
        " FROM sgw_auction WHERE sequence_id = $1 FOR UPDATE"
    ))
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

    let Some(settled) = settle_locked(&mut tx, &live, SettleCause::Expired).await? else {
        tx.rollback().await?;
        return Ok(None);
    };
    tx.commit().await?;

    let event = if settled.sold {
        "bm.sold"
    } else {
        "bm.expired"
    };
    let account_id = account_of(pool, settled.seller_id).await;
    log_transition(
        event,
        account_id,
        settled.seller_id,
        Some(&settled.before),
        &settled.after,
    );
    count_bm_outcome("settle", event.trim_start_matches("bm."));
    Ok(Some(settled))
}

/// The account a player belongs to, for a settlement row's `account_id`
/// (the sweep has no session). `None` if the lookup fails.
async fn account_of(pool: &PgPool, player_id: i32) -> Option<u32> {
    sqlx::query_scalar::<_, i32>("SELECT account_id FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|a| u32::try_from(a).ok())
}

/// Push `onBMAuctionRemove` to the seller and buyer (when online) for each
/// settled auction, and `onUpdateItem` to whoever got the item. The auction
/// left the active set, so any client showing it must drop the row.
pub(super) async fn notify_settled(net: BmNet<'_>, pool: &PgPool, s: &SettledAuction) {
    let recipient = s.buyer_id.unwrap_or(s.seller_id);
    let mut targets = vec![s.seller_id];
    targets.extend(s.buyer_id);
    for player_id in targets {
        let Some(eid) = net.entity_of(player_id) else {
            continue;
        };
        send_bm_auction_remove(net, eid, s.sequence_id).await;
        if player_id == recipient {
            if let Some(placed) = s.placed {
                send_item_placed(net, pool, eid, player_id, placed.item_id).await;
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
        let net = BmNet {
            transport: &transport,
            connected: &connected,
            entity_to_addr: &entity_to_addr,
        };
        run_sweep_pass(&pool, net).await;
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        ticker.tick().await; // skip the immediate first tick — handled above
        loop {
            ticker.tick().await;
            run_sweep_pass(&pool, net).await;
        }
    });
}

/// One sweep pass: settle what is due, then notify the online parties.
/// Every 30 s, so an info span is well under the hot-path threshold.
#[tracing::instrument(
    name = "black_market.sweep_pass",
    level = "info",
    skip_all,
    fields(settled = tracing::field::Empty)
)]
async fn run_sweep_pass(pool: &PgPool, net: BmNet<'_>) {
    match settle_expired_once(pool).await {
        Ok(settled) if !settled.is_empty() => {
            tracing::Span::current().record("settled", settled.len());
            tracing::info!(
                count = settled.len(),
                "black_market: settled expired auctions"
            );
            for s in &settled {
                notify_settled(net, pool, s).await;
            }
        }
        Ok(_) => {}
        Err(e) => tracing::warn!("black_market: sweep failed: {e}"),
    }
}
