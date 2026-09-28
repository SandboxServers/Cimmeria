//! Expiry sweep: a periodic background task that settles auctions whose
//! `expires_at` has passed, through [`super::settle::settle_locked`].
//!
//! Each auction settles in its own transaction, in `expires_at` order, so
//! one bad row cannot stop the pass (plan §5.2): a settlement that fails for
//! good (the escrowed row is missing, or the mail writer refuses a payout)
//! is rolled back and the auction set to `QUARANTINED` for an operator,
//! with `bm.quarantined` and its `reason`; a database error leaves it
//! `ACTIVE` for the next pass (`bm.settle_retry`). Each settled auction is
//! returned so the caller can push `onBMAuctionRemove` and the new-mail
//! notices to the online parties.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::helpers::now_unix_secs;
use super::send::{send_bm_auction_remove, BmNet};
use super::settle::{settle_locked, SettleCause, SettleError};
use super::telemetry::{count_bm_outcome, log_transition};
use super::types::{auction_columns, auction_status, AuctionRow};
use crate::base::ConnectedClientState;

pub use super::settle::SettledAuction;

/// How often the expiry sweep runs.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(30);

/// What one sweep pass did.
#[derive(Debug, Default)]
pub struct SweepReport {
    pub settled: Vec<SettledAuction>,
    /// Auctions set to `QUARANTINED` this pass, with the `reason`.
    pub quarantined: Vec<(i32, &'static str)>,
    /// Auctions left `ACTIVE` for the next pass, with the `reason`.
    pub retried: Vec<(i32, &'static str)>,
}

/// How one due auction ended.
enum Outcome {
    Settled(Box<SettledAuction>),
    /// No longer `ACTIVE` when locked: someone else settled it.
    Skipped,
    Quarantined(&'static str),
    Retry(&'static str),
}

/// Settle every auction that is `ACTIVE` and past its `expires_at`. Only
/// reading the due set can fail the pass; each auction's own failure is
/// logged and reported instead.
///
/// This is the unit of work the background loop runs; the live-DB sweep
/// tests call it directly, so it carries no transport state.
pub async fn settle_expired_once(pool: &PgPool) -> Result<SweepReport, sqlx::Error> {
    let now = now_unix_secs();

    // Snapshot the due auctions up front. Each row is re-locked inside its
    // own transaction before anything moves, so a row another worker settles
    // between the snapshot and the lock is skipped.
    let due: Vec<AuctionRow> = sqlx::query_as(concat!(
        "SELECT ",
        auction_columns!(),
        " FROM sgw_auction WHERE status = $1 AND expires_at <= $2 \
          ORDER BY expires_at, sequence_id"
    ))
    .bind(auction_status::ACTIVE)
    .bind(now)
    .fetch_all(pool)
    .await?;

    let mut report = SweepReport::default();
    for auction in due {
        match settle_one(pool, &auction).await {
            Outcome::Settled(s) => report.settled.push(*s),
            Outcome::Skipped => {}
            Outcome::Quarantined(reason) => report.quarantined.push((auction.sequence_id, reason)),
            Outcome::Retry(reason) => report.retried.push((auction.sequence_id, reason)),
        }
    }
    Ok(report)
}

/// Settle one auction in its own transaction, and log what happened.
async fn settle_one(pool: &PgPool, auction: &AuctionRow) -> Outcome {
    let settled = match try_settle(pool, auction).await {
        Ok(Some(settled)) => settled,
        Ok(None) => return Outcome::Skipped,
        Err(e) if e.is_permanent() => return quarantine(pool, auction, &e).await,
        Err(e) => {
            tracing::warn!(
                event = "bm.settle_retry",
                auction_id = auction.sequence_id,
                seller_id = auction.seller_id,
                bidder_id = auction.current_bidder,
                reason = e.reason(),
                error = %e,
                "Black Market settlement failed; the auction stays active for the next sweep pass"
            );
            count_bm_outcome("settle", "retry");
            return Outcome::Retry(e.reason());
        }
    };

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
    for payout in &settled.payouts {
        payout.log(account_id, settled.seller_id);
    }
    count_bm_outcome("settle", event.trim_start_matches("bm."));
    Outcome::Settled(Box::new(settled))
}

/// The settlement transaction. `Ok(None)` if the row was no longer
/// `ACTIVE` when locked. Dropping `tx` on an error rolls everything back.
async fn try_settle(
    pool: &PgPool,
    auction: &AuctionRow,
) -> Result<Option<SettledAuction>, SettleError> {
    let mut tx = pool.begin().await?;

    // Re-lock and re-read the live row, so the sold/unsold decision uses the
    // bid as it is now, not the snapshot's.
    let locked: Option<AuctionRow> = sqlx::query_as(concat!(
        "SELECT ",
        auction_columns!(),
        " FROM sgw_auction WHERE sequence_id = $1 FOR UPDATE"
    ))
    .bind(auction.sequence_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(live) = locked.filter(|row| row.status == auction_status::ACTIVE) else {
        return Ok(None);
    };

    match settle_locked(&mut tx, &live, SettleCause::Expired).await {
        Ok(settled) => {
            tx.commit().await?;
            Ok(Some(settled))
        }
        Err(SettleError::Gone) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Take a settlement that fails for good out of the sweep: `QUARANTINED`,
/// in a transaction of its own after the failed one rolled back. The item
/// stays in container 18 and any standing bid stays held until an operator
/// resolves it.
async fn quarantine(pool: &PgPool, auction: &AuctionRow, e: &SettleError) -> Outcome {
    let reason = e.reason();
    let marked =
        sqlx::query("UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2 AND status = $3")
            .bind(auction_status::QUARANTINED)
            .bind(auction.sequence_id)
            .bind(auction_status::ACTIVE)
            .execute(pool)
            .await;
    match marked {
        Ok(r) if r.rows_affected() == 1 => {
            let account_id = account_of(pool, auction.seller_id).await;
            tracing::error!(
                event = "bm.quarantined",
                account_id,
                player_id = auction.seller_id,
                auction_id = auction.sequence_id,
                seller_id = auction.seller_id,
                bidder_id = auction.current_bidder,
                held_cash = auction.escrowed_cash(),
                item_id = auction.item_id,
                item_def_id = auction.item_def_id,
                reason,
                error = %e,
                "Black Market auction could not be settled and is quarantined for an operator"
            );
            count_bm_outcome("settle", "quarantined");
            Outcome::Quarantined(reason)
        }
        Ok(_) => Outcome::Skipped,
        Err(db) => {
            tracing::warn!(
                event = "bm.settle_retry",
                auction_id = auction.sequence_id,
                seller_id = auction.seller_id,
                reason = "quarantine_failed",
                settle_reason = reason,
                error = %db,
                "Black Market settlement failed and could not be quarantined; retried next pass"
            );
            count_bm_outcome("settle", "retry");
            Outcome::Retry(reason)
        }
    }
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

/// Push `onBMAuctionRemove` to the seller and buyer (when online) for a
/// settled auction, since it left the active set, and tell each online
/// mail recipient their mail arrived.
pub(super) async fn notify_settled(net: BmNet<'_>, pool: &PgPool, s: &SettledAuction) {
    let mut targets = vec![s.seller_id];
    targets.extend(s.buyer_id);
    for player_id in targets {
        if let Some(eid) = net.entity_of(player_id) {
            send_bm_auction_remove(net, eid, s.sequence_id).await;
        }
    }
    let ctx = net.feedback();
    for payout in &s.payouts {
        payout.notify(pool, &ctx).await;
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
        Ok(report) => {
            tracing::Span::current().record("settled", report.settled.len());
            if !report.settled.is_empty()
                || !report.quarantined.is_empty()
                || !report.retried.is_empty()
            {
                tracing::info!(
                    settled = report.settled.len(),
                    quarantined = report.quarantined.len(),
                    retried = report.retried.len(),
                    "black_market: sweep pass done"
                );
            }
            for s in &report.settled {
                notify_settled(net, pool, s).await;
            }
        }
        Err(e) => tracing::warn!(
            event = "bm.sweep_failed",
            reason = "due_query_failed",
            error = %e,
            "black_market: sweep pass could not read the due auctions"
        ),
    }
}
