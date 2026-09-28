//! The Black Market's mail: every payout, return and refund an auction owes
//! is a system gate mail from "Black Market", written through the mail
//! module's one writer ([`send_system_mail_tx`]) inside the caller's
//! transaction (BM-02b, decision D-BM10).
//!
//! - **Sold** (sweep or buyout): the escrowed row goes to the buyer as an
//!   `ExistingInstance` from the seller's container 18, and the winning bid
//!   is minted to the seller as mail cash.
//! - **Expired unsold** and **cancelled**: the row goes back to the seller
//!   the same way.
//! - **Outbid**, and a **cancelled** auction's standing bid: the held cash
//!   goes back to the bidder as mail cash.
//!
//! Nothing here commits. The writer mints cash on every call, so each
//! caller gates its payouts on a conditional `sgw_auction` status write in
//! the same transaction (`settle.rs`, `cancel.rs`), and a bid refunds the
//! outbid player only inside the transaction that moved the bid on.
//! After the commit the caller runs [`Payout::log`] for each payout (the
//! `mail.system_sent` and `bm.payout` rows) and [`Payout::notify`] (the
//! new-mail notice to an online recipient).

use sqlx::{PgPool, Postgres, Transaction};

use super::super::mail::{
    send_system_mail_tx, SystemItem, SystemMail, SystemMailError, SystemMailSent,
};
use super::telemetry::count_bm_outcome;
use super::types::AuctionRow;
use crate::base::feedback::FeedbackCtx;

/// `sgw_gate_mail.sender_name` of every Black Market mail.
pub const BM_SENDER_NAME: &str = "Black Market";

/// Why a mail was sent: the `reason` of its `bm.payout` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayoutReason {
    /// The sweep settled an auction with a winning bid.
    Sold,
    /// A bid reached the buyout price and settled the auction at once.
    Buyout,
    /// The sweep settled an auction with no bid.
    Expired,
    /// The seller cancelled.
    Cancelled,
    /// A higher bid replaced this bidder's.
    Outbid,
}

impl PayoutReason {
    pub fn label(self) -> &'static str {
        match self {
            PayoutReason::Sold => "sold",
            PayoutReason::Buyout => "buyout",
            PayoutReason::Expired => "expired",
            PayoutReason::Cancelled => "cancelled",
            PayoutReason::Outbid => "outbid",
        }
    }
}

/// Who a mail went to, relative to the auction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayoutRole {
    Seller,
    Buyer,
    Bidder,
}

impl PayoutRole {
    pub fn label(self) -> &'static str {
        match self {
            PayoutRole::Seller => "seller",
            PayoutRole::Buyer => "buyer",
            PayoutRole::Bidder => "bidder",
        }
    }
}

/// The subject and body of one kind of mail.
fn texts(reason: PayoutReason, role: PayoutRole) -> (&'static str, &'static str) {
    match (reason, role) {
        (PayoutReason::Sold | PayoutReason::Buyout, PayoutRole::Seller) => (
            "Auction Sold",
            "Your Black Market auction sold. The winning bid is attached.",
        ),
        (PayoutReason::Sold | PayoutReason::Buyout, _) => (
            "Auction Won",
            "You won a Black Market auction. Your item is attached.",
        ),
        (PayoutReason::Expired, _) => (
            "Auction Expired",
            "Your Black Market auction ended with no bids. Your item is attached.",
        ),
        (PayoutReason::Cancelled, PayoutRole::Seller) => (
            "Auction Cancelled",
            "You cancelled your Black Market auction. Your item is attached.",
        ),
        (PayoutReason::Cancelled, _) => (
            "Auction Cancelled",
            "The seller cancelled a Black Market auction you were bidding on. \
             Your bid is attached.",
        ),
        (PayoutReason::Outbid, _) => (
            "Outbid",
            "You were outbid on a Black Market auction. Your bid is attached.",
        ),
    }
}

/// One mail a Black Market transaction wrote, not yet committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payout {
    pub auction_id: i32,
    pub reason: PayoutReason,
    pub role: PayoutRole,
    pub mail: SystemMailSent,
}

impl Payout {
    /// After the commit: log `mail.system_sent` and `bm.payout`, with the
    /// actor's ids (for the sweep, the seller's), and count it.
    pub fn log(&self, account_id: Option<u32>, player_id: i32) {
        self.mail.log_sent();
        let item = self.mail.item;
        tracing::info!(
            event = "bm.payout",
            account_id,
            player_id,
            auction_id = self.auction_id,
            reason = self.reason.label(),
            role = self.role.label(),
            recipient_player_id = self.mail.recipient_player_id,
            mail_id = self.mail.mail_id,
            cash = self.mail.cash,
            item_source = self.mail.item_source,
            item_id = item.map(|i| i.item_id),
            type_id = item.map(|i| i.type_id),
            stack_size = item.map(|i| i.stack_size),
            "Black Market mail delivered"
        );
        count_bm_outcome("payout", self.reason.label());
    }

    /// After the commit: tell the recipient, if online, that the mail
    /// arrived (a feedback line and the header). Offline is skipped.
    pub async fn notify(&self, pool: &PgPool, ctx: &FeedbackCtx<'_>) {
        self.mail.notify(pool, ctx).await;
    }
}

/// Write one Black Market mail to `to` inside `tx`. Nothing is committed.
pub async fn mail_payout(
    tx: &mut Transaction<'_, Postgres>,
    auction: &AuctionRow,
    to: i32,
    reason: PayoutReason,
    role: PayoutRole,
    cash: i64,
    item: SystemItem,
) -> Result<Payout, SystemMailError> {
    let (subject, body) = texts(reason, role);
    let mail = SystemMail {
        sender_name: BM_SENDER_NAME.to_owned(),
        recipient_player_id: to,
        subject: subject.to_owned(),
        body: body.to_owned(),
        cash,
        item,
    };
    let sent = send_system_mail_tx(tx, &mail).await?;
    Ok(Payout {
        auction_id: auction.sequence_id,
        reason,
        role,
        mail: sent,
    })
}

/// Refund `auction`'s standing bid to its bidder by mail, if it has one.
/// A bidder whose character is gone is logged and skipped (their held cash
/// went with them; the `sgw_player` delete trigger normally clears the bid
/// first). The writer refuses a missing recipient before it writes
/// anything, so the transaction is still good.
pub async fn refund_standing_bid(
    tx: &mut Transaction<'_, Postgres>,
    auction: &AuctionRow,
    reason: PayoutReason,
) -> Result<Option<Payout>, SystemMailError> {
    let Some(bidder) = auction.current_bidder.filter(|_| auction.current_bid > 0) else {
        return Ok(None);
    };
    let cash = i64::from(auction.current_bid);
    match mail_payout(
        tx,
        auction,
        bidder,
        reason,
        PayoutRole::Bidder,
        cash,
        SystemItem::None,
    )
    .await
    {
        Ok(p) => Ok(Some(p)),
        Err(SystemMailError::RecipientNotFound) => {
            tracing::warn!(
                event = "bm.refund_skipped",
                auction_id = auction.sequence_id,
                bidder_id = bidder,
                amount = cash,
                cause = reason.label(),
                reason = "bidder_missing",
                "Black Market bidder row missing, cannot refund"
            );
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every (reason, role) pair a caller uses has one-line text the
    /// writer's D-SS12 rules accept (a refused text would roll back a
    /// settlement).
    #[test]
    fn every_mail_text_passes_the_writer_rules() {
        use cimmeria_entity::organization::org_text::{self, TextField};
        let reasons = [
            PayoutReason::Sold,
            PayoutReason::Buyout,
            PayoutReason::Expired,
            PayoutReason::Cancelled,
            PayoutReason::Outbid,
        ];
        for reason in reasons {
            for role in [PayoutRole::Seller, PayoutRole::Buyer, PayoutRole::Bidder] {
                let (subject, body) = texts(reason, role);
                org_text::validate(TextField::MailSubject, subject).unwrap();
                org_text::validate(TextField::MailBody, body).unwrap();
            }
        }
        org_text::validate(TextField::MailSubject, BM_SENDER_NAME).unwrap();
    }
}
