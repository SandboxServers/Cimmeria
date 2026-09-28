//! Settling one auction: the shared step behind the expiry sweep and an
//! immediate buyout (decision D8). Every item and coin it moves goes out
//! as system mail from "Black Market" (BM-02b, decision D-BM10):
//!
//! - **Sold** (a bidder holds a positive bid): the escrowed row is mailed
//!   to the buyer (`ExistingInstance` out of the seller's container 18), and
//!   the winning bid to the seller as cash; status → `SOLD`.
//! - **Unsold**: the escrowed row is mailed back to the seller; status →
//!   `EXPIRED`.
//! - **Boot-seed listings** have no instance and no seller to pay: a sold
//!   one mails the buyer a new instance of the listed type and nobody the
//!   cash (the house keeps it); an unsold one moves nothing.
//!
//! **Exactly once.** The mail writer mints cash on every call, so the first
//! thing a settlement does is the conditional status write (`WHERE status =
//! ACTIVE`, `RETURNING`). A second settlement of the same auction, even
//! from a stale snapshot, matches no row and writes nothing
//! ([`SettleError::Gone`]). Any later failure rolls the status back with
//! the mail.

use sqlx::{Postgres, Transaction};

use super::super::mail::{SystemItem, SystemMailError};
use super::escrow::{escrowed_item, is_seed_listing, lock_escrow};
use super::helpers::lock_players;
use super::payout_mail::{mail_payout, Payout, PayoutReason, PayoutRole};
use super::types::{auction_columns, auction_status, AuctionRow};

/// Why an auction was settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleCause {
    /// The sweep found it past `expires_at`.
    Expired,
    /// A bid reached the buyout price.
    Buyout,
}

impl SettleCause {
    pub fn label(self) -> &'static str {
        match self {
            SettleCause::Expired => "expired",
            SettleCause::Buyout => "buyout",
        }
    }
}

/// Why a settlement did not happen. The caller rolls back.
#[derive(Debug)]
pub enum SettleError {
    /// The auction was no longer `ACTIVE`: someone settled it first.
    Gone,
    /// A player's listing has no container-18 row (`bm.escrow_missing`).
    EscrowMissing,
    /// The mail writer refused a payout (`mail.system_refused`).
    Mail(SystemMailError),
    Db(sqlx::Error),
}

impl SettleError {
    /// Stable `reason` log value.
    pub fn reason(&self) -> &'static str {
        match self {
            SettleError::Gone => "not_active",
            SettleError::EscrowMissing => "escrow_missing",
            SettleError::Mail(e) => e.reason(),
            SettleError::Db(_) => "db_error",
        }
    }

    /// Will retrying fail the same way? A missing escrow row or a refused
    /// mail does; a database error may be transient.
    pub fn is_permanent(&self) -> bool {
        match self {
            SettleError::EscrowMissing => true,
            SettleError::Mail(e) => !matches!(e, SystemMailError::Db(_)),
            SettleError::Gone | SettleError::Db(_) => false,
        }
    }
}

impl std::fmt::Display for SettleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettleError::Mail(e) => write!(f, "settlement mail refused: {e}"),
            SettleError::Db(e) => write!(f, "database error: {e}"),
            other => f.write_str(other.reason()),
        }
    }
}

impl From<sqlx::Error> for SettleError {
    fn from(e: sqlx::Error) -> Self {
        SettleError::Db(e)
    }
}

impl From<SystemMailError> for SettleError {
    fn from(e: SystemMailError) -> Self {
        SettleError::Mail(e)
    }
}

/// One settled auction, for notification and logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledAuction {
    pub sequence_id: i32,
    pub seller_id: i32,
    pub buyer_id: Option<i32>,
    pub sold: bool,
    pub cause: SettleCause,
    /// The row before and after the status change.
    pub before: AuctionRow,
    pub after: AuctionRow,
    /// The mails written, not yet logged (see [`Payout::after_commit`]).
    pub payouts: Vec<Payout>,
}

/// Settle `live`, a row the caller has locked `FOR UPDATE` in `tx` and
/// found `ACTIVE`. Nothing is committed.
///
/// Lock order after the auction row: the seller's escrow advisory locks,
/// the escrowed item row, then the seller's and buyer's `sgw_player` rows
/// in ascending `player_id`, then the mail writer (re-locks only).
pub async fn settle_locked(
    tx: &mut Transaction<'_, Postgres>,
    live: &AuctionRow,
    cause: SettleCause,
) -> Result<SettledAuction, SettleError> {
    let buyer = live.current_bidder.filter(|_| live.current_bid > 0);
    let status = if buyer.is_some() {
        auction_status::SOLD
    } else {
        auction_status::EXPIRED
    };
    let after: Option<AuctionRow> = sqlx::query_as(concat!(
        "UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2 AND status = $3 RETURNING ",
        auction_columns!()
    ))
    .bind(status)
    .bind(live.sequence_id)
    .bind(auction_status::ACTIVE)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(after) = after else {
        return Err(SettleError::Gone);
    };

    let seed = is_seed_listing(live);
    let mut payouts = Vec::new();
    match buyer {
        Some(buyer_id) => {
            let item = locked_item(tx, live, seed).await?;
            let mut players = vec![buyer_id];
            if !seed {
                players.push(live.seller_id);
            }
            lock_players(tx, &players).await?;
            let reason = match cause {
                SettleCause::Expired => PayoutReason::Sold,
                SettleCause::Buyout => PayoutReason::Buyout,
            };
            payouts
                .push(mail_payout(tx, live, buyer_id, reason, PayoutRole::Buyer, 0, item).await?);
            if !seed {
                let cash = i64::from(live.current_bid);
                payouts.push(
                    mail_payout(
                        tx,
                        live,
                        live.seller_id,
                        reason,
                        PayoutRole::Seller,
                        cash,
                        SystemItem::None,
                    )
                    .await?,
                );
            }
        }
        None if seed => {}
        None => {
            let item = locked_item(tx, live, seed).await?;
            lock_players(tx, &[live.seller_id]).await?;
            payouts.push(
                mail_payout(
                    tx,
                    live,
                    live.seller_id,
                    PayoutReason::Expired,
                    PayoutRole::Seller,
                    0,
                    item,
                )
                .await?,
            );
        }
    }

    Ok(SettledAuction {
        sequence_id: live.sequence_id,
        seller_id: live.seller_id,
        buyer_id: buyer,
        sold: buyer.is_some(),
        cause,
        before: live.clone(),
        after,
        payouts,
    })
}

/// The seller's escrow locks, then the item the settlement mails.
async fn locked_item(
    tx: &mut Transaction<'_, Postgres>,
    live: &AuctionRow,
    seed: bool,
) -> Result<SystemItem, SettleError> {
    if !seed {
        lock_escrow(tx, live.seller_id).await?;
    }
    escrowed_item(tx, live)
        .await?
        .ok_or(SettleError::EscrowMissing)
}
