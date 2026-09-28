//! Settling one auction: the shared step behind the expiry sweep and an
//! immediate buyout (decision D8).
//!
//! - **Sold** (a bidder holds a positive bid): the escrowed row moves from
//!   the seller's container 18 into the buyer's bags, the seller is mailed
//!   the winning cash, the buyer a notice; status → `SOLD`.
//! - **Unsold**: the escrowed row moves back into the seller's bags and the
//!   seller is mailed a notice; status → `EXPIRED`.
//!
//! The status write is conditional on the row still being `ACTIVE`
//! (`rows_affected == 1`), so a settlement can never pay twice: the cash
//! mail mints money on every call. Moving settlement onto the social-systems
//! mail API (the item mailed as an `ExistingInstance` from container 18) is
//! packet BM-02b; until then the item goes straight to the bags.

use sqlx::PgConnection;

use super::escrow::{deliver_from_escrow, is_seed_listing, Placed};
use super::payout_mail::{
    send_mail_to_player, BM_SENDER_NAME, SOLD_BUYER_BODY, SOLD_BUYER_SUBJECT, SOLD_SELLER_BODY,
    SOLD_SELLER_SUBJECT, UNSOLD_BODY, UNSOLD_SUBJECT,
};
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

/// One settled auction, for notification and logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettledAuction {
    pub sequence_id: i32,
    pub seller_id: i32,
    pub buyer_id: Option<i32>,
    pub sold: bool,
    /// The row before and after the status change.
    pub before: AuctionRow,
    pub after: AuctionRow,
    /// Where the item went; `None` when a boot-seed listing expired
    /// unsold (it has no instance to return).
    pub placed: Option<Placed>,
}

/// Settle `live`, a row the caller has locked `FOR UPDATE` in this
/// transaction and found `ACTIVE`. Returns `Ok(None)` if it cannot settle:
/// the conditional status write matched nothing (someone settled it first),
/// or a player's escrowed row is missing (`bm.escrow_missing`; the auction
/// is left for an operator rather than paid out or duplicated). The caller
/// then rolls back.
pub async fn settle_locked(
    conn: &mut PgConnection,
    live: &AuctionRow,
    cause: SettleCause,
) -> Result<Option<SettledAuction>, sqlx::Error> {
    let buyer = live.current_bidder.filter(|_| live.current_bid > 0);
    let (status, placed) = match buyer {
        Some(buyer_id) => {
            send_mail_to_player(
                &mut *conn,
                live.seller_id,
                i64::from(live.current_bid),
                None,
                0,
                SOLD_SELLER_SUBJECT,
                SOLD_SELLER_BODY,
                BM_SENDER_NAME,
            )
            .await?;
            let Ok(placed) = deliver_from_escrow(conn, live, buyer_id, true).await? else {
                return Ok(None);
            };
            send_mail_to_player(
                &mut *conn,
                buyer_id,
                0,
                None,
                0,
                SOLD_BUYER_SUBJECT,
                SOLD_BUYER_BODY,
                BM_SENDER_NAME,
            )
            .await?;
            (auction_status::SOLD, Some(placed))
        }
        // A boot-seed listing has no instance and a reserved seller:
        // nothing to hand back.
        None if is_seed_listing(live) => (auction_status::EXPIRED, None),
        None => {
            let Ok(placed) = deliver_from_escrow(conn, live, live.seller_id, true).await? else {
                return Ok(None);
            };
            send_mail_to_player(
                &mut *conn,
                live.seller_id,
                0,
                None,
                0,
                UNSOLD_SUBJECT,
                UNSOLD_BODY,
                BM_SENDER_NAME,
            )
            .await?;
            (auction_status::EXPIRED, Some(placed))
        }
    };

    let after: Option<AuctionRow> = sqlx::query_as(concat!(
        "UPDATE sgw_auction SET status = $1 WHERE sequence_id = $2 AND status = $3 RETURNING ",
        auction_columns!()
    ))
    .bind(status)
    .bind(live.sequence_id)
    .bind(auction_status::ACTIVE)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(after) = after else {
        return Ok(None);
    };
    if placed.is_some_and(|p| p.overflow) {
        tracing::warn!(
            event = "bm.delivery_overflow",
            auction_id = live.sequence_id,
            player_id = buyer.unwrap_or(live.seller_id),
            cause = cause.label(),
            reason = "bag_full",
            "Black Market item placed past the main bag's last slot: every carried bag was full"
        );
    }
    Ok(Some(SettledAuction {
        sequence_id: live.sequence_id,
        seller_id: live.seller_id,
        buyer_id: buyer,
        sold: buyer.is_some(),
        before: live.clone(),
        after,
        placed,
    }))
}
