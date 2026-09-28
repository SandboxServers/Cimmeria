//! `.bm_expire <auctionId>` (BM-07): make one active auction due now and
//! settle it at once through the expiry sweep's own path.

use std::sync::Arc;

use sqlx::PgPool;

use super::super::escrow::is_seed_listing;
use super::super::helpers::now_unix_secs;
use super::super::player_name;
use super::super::sweep::{notify_settled, settle_expired_once, SettledAuction};
use super::super::types::auction_status;
use super::{item_name, pool_or_refuse, GmCtx};

/// The GM's line for an auction that is not active.
fn not_active_line(sequence_id: i32, status: Option<i16>) -> (&'static str, String) {
    match status {
        None => (
            "auction_not_found",
            format!(".bm_expire: no auction has id {sequence_id}. Type .bm_list for ids."),
        ),
        Some(auction_status::SOLD) => (
            "auction_sold",
            format!(".bm_expire: auction {sequence_id} is already sold."),
        ),
        Some(auction_status::CANCELLED) => (
            "auction_cancelled",
            format!(".bm_expire: auction {sequence_id} was cancelled by its seller."),
        ),
        Some(_) => (
            "auction_expired",
            format!(".bm_expire: auction {sequence_id} has already expired."),
        ),
    }
}

/// `.bm_expire <auctionId>`: set the auction's `expires_at` to now (never
/// later than it was), run one sweep pass, notify the parties the way the
/// sweep does, and tell the GM how this auction settled.
#[tracing::instrument(
    name = "black_market.gm_expire",
    level = "info",
    skip_all,
    fields(entity_id = ctx.actor.entity_id, player_id = ctx.actor.player_id, auction_id = sequence_id)
)]
pub async fn gm_expire(ctx: GmCtx<'_>, sequence_id: i32, db_pool: &Option<Arc<PgPool>>) {
    let Some(pool) = pool_or_refuse(&ctx, "bm_expire", db_pool).await else {
        return;
    };
    let now = now_unix_secs();
    // One conditional UPDATE: a bid, cancel or sweep racing it either
    // commits first (and the auction is refused here or settled by that
    // sweep) or waits on the row lock and sees it due.
    let made_due: Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
        "UPDATE sgw_auction SET expires_at = LEAST(expires_at, $2) \
         WHERE sequence_id = $1 AND status = $3 RETURNING seller_id",
    )
    .bind(sequence_id)
    .bind(now)
    .bind(auction_status::ACTIVE)
    .fetch_optional(pool)
    .await;
    match made_due {
        Ok(Some(_)) => {}
        Ok(None) => {
            let status: Result<Option<i16>, sqlx::Error> =
                sqlx::query_scalar("SELECT status FROM sgw_auction WHERE sequence_id = $1")
                    .bind(sequence_id)
                    .fetch_optional(pool)
                    .await;
            let (reason, line) = match status {
                Ok(status) => not_active_line(sequence_id, status),
                Err(e) => (
                    "db_error",
                    format!(".bm_expire: auction {sequence_id} was not changed ({e})."),
                ),
            };
            ctx.refuse("bm_expire", reason, &line).await;
            return;
        }
        Err(e) => {
            let line = format!(".bm_expire: auction {sequence_id} was not changed ({e}).");
            ctx.refuse("bm_expire", "db_error", &line).await;
            return;
        }
    }

    let report = match settle_expired_once(pool).await {
        Ok(report) => report,
        Err(e) => {
            let line = format!(
                ".bm_expire: auction {sequence_id} is due now, but settling failed ({e}); \
                 the sweep retries every 30 seconds."
            );
            ctx.refuse("bm_expire", "db_error", &line).await;
            return;
        }
    };
    for s in &report.settled {
        notify_settled(ctx.net, pool, s).await;
    }
    let ours = report.settled.iter().find(|s| s.sequence_id == sequence_id);
    let reason_of = |list: &[(i32, &'static str)]| {
        list.iter()
            .find(|(id, _)| *id == sequence_id)
            .map(|(_, reason)| *reason)
    };
    let quarantined = reason_of(&report.quarantined);
    let retried = reason_of(&report.retried);
    let outcome = match (ours, quarantined, retried) {
        (Some(s), _, _) if s.sold => "sold",
        (Some(_), _, _) => "expired",
        (None, Some(_), _) => "quarantined",
        (None, None, Some(_)) => "retry",
        (None, None, None) => "not_settled",
    };
    tracing::info!(
        event = "bm.gm_action",
        action = "bm_expire",
        entity_id = ctx.actor.entity_id,
        account_id = ctx.actor.account_id,
        player_id = ctx.actor.player_id,
        auction_id = sequence_id,
        expires_at = now,
        outcome,
        settled_in_pass = report.settled.len(),
        reason = quarantined.or(retried),
        "GM .bm_expire made an auction due and ran a sweep pass"
    );
    let line = match ours {
        Some(s) => settled_line(pool, s).await,
        // A settlement that fails for good is quarantined for an operator
        // (`bm.quarantined`); a database error stays active for the sweep.
        None => match (quarantined, retried) {
            (Some(reason), _) => format!(
                "Auction {sequence_id} could not be settled and is quarantined ({reason}): its \
                 item stays in escrow for an operator (look for bm.quarantined)."
            ),
            (None, Some(reason)) => format!(
                "Auction {sequence_id} is due now but settling failed ({reason}); the sweep \
                 retries every 30 seconds."
            ),
            // A racing sweep may have taken it first.
            (None, None) => format!(
                "Auction {sequence_id} is due now but did not settle in this pass: another \
                 sweep probably settled it first."
            ),
        },
    };
    ctx.tell(&line).await;
}

/// How one auction settled, in the GM's words. Since BM-02b every payout
/// is system mail from "Black Market".
async fn settled_line(pool: &PgPool, s: &SettledAuction) -> String {
    let item = item_name(pool, s.before.item_def_id).await;
    let id = s.sequence_id;
    match s.buyer_id {
        Some(buyer) => format!(
            "Auction {id} ({item}) expired and sold to {} for {} naquadah. The item is mailed \
             to the buyer and the cash to the seller ({}).",
            player_name(pool, buyer).await,
            s.before.current_bid,
            player_name(pool, s.seller_id).await,
        ),
        None if is_seed_listing(&s.before) => format!(
            "Auction {id} ({item}) expired unsold. It was a system listing, so there was no \
             item to return."
        ),
        None => format!(
            "Auction {id} ({item}) expired unsold. The item is mailed back to its seller ({}).",
            player_name(pool, s.seller_id).await,
        ),
    }
}
