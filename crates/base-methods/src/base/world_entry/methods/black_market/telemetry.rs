//! Black Market telemetry (plan §5.1, BM-02 line): one DEBUG event per state
//! transition, one row per refusal, and the `bm_outcome_total` counter.
//!
//! - **Transitions**: `event = "bm.<transition>"` (`bm.listed`, `bm.bid`,
//!   `bm.outbid_refund`, `bm.cancelled`, `bm.sold`, `bm.expired`), each with
//!   `auction_id`, `seller_id`, `bidder_id`, `bid_before` / `bid_after`,
//!   `escrow_cash_before` / `escrow_cash_after` (the bidder cash the auction
//!   holds) and `item_type_id` / `item_name`, plus the actor's `account_id`,
//!   `player_id` and their names. `seller_name` / `bidder_name` are present
//!   only when the line already knows the player (the actor, or the seller a
//!   sweep looked up): no log line pays a query for a name.
//! - **Refusals**: `event = "bm.refused"` at INFO with `reason` equal to the
//!   [`BMError::reason`] of the id the client got, and `error_id` (named by
//!   `bm_error`).
//!   A server failure is `reason = internal` at ERROR, with the `stage`.
//! - **Counter**: `bm_outcome_total{op, outcome}`; `outcome` is `ok` or the
//!   refusal reason, never an id. The cell counts its own refusals on the
//!   same series.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use super::types::AuctionRow;
use super::wire::BMError;
use crate::base::{session_identity, ConnectedClientState};

/// Count one request on `bm_outcome_total{op, outcome}`.
pub fn count_bm_outcome(op: &'static str, outcome: &'static str) {
    cimmeria_observability::counter!(
        "bm_outcome_total",
        "op" => op,
        "outcome" => outcome,
    );
}

/// The acting player's ids with the names that pair with them (Rule 6). The
/// names are `None` when the session has none yet; they are then left off the
/// line, never written as `""`.
#[derive(Debug, Clone, Copy, Default)]
pub struct Who {
    pub account_id: Option<u32>,
    pub account_name: Option<&'static str>,
    pub player_id: i32,
    pub player_name: Option<&'static str>,
}

impl Who {
    /// The name of `player_id` if it is this player; any other player's name
    /// would cost a lookup, so it is left `None`.
    pub fn name_of(&self, player_id: i32) -> Option<&'static str> {
        (player_id == self.player_id)
            .then_some(self.player_name)
            .flatten()
    }
}

/// The player a request came from: the cell's entity and `player_id`, and
/// the session's `account_id` (`None` only when the session is gone), with
/// the names the session carries.
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub account_name: Option<&'static str>,
    pub player_id: i32,
    pub player_name: Option<&'static str>,
}

impl Actor {
    /// Resolve the account from the session map and record it on the
    /// current span.
    pub fn resolve(
        entity_id: u32,
        player_id: i32,
        connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) -> Self {
        let identity = session_identity::identity_for_entity(connected, entity_to_addr, entity_id);
        if let Some(account_id) = identity.account_id {
            tracing::Span::current().record("account_id", account_id);
        }
        Self {
            entity_id,
            account_id: identity.account_id,
            account_name: identity.account_name,
            player_id,
            player_name: identity.player_name,
        }
    }

    /// The ids and names a transition or payout line carries.
    pub fn who(&self) -> Who {
        Who {
            account_id: self.account_id,
            account_name: self.account_name,
            player_id: self.player_id,
            player_name: self.player_name,
        }
    }
}

/// `items.name` of the listed item type, `None` for a seed placeholder.
pub fn item_name(item_def_id: i32) -> Option<&'static str> {
    cimmeria_names::book()
        .item(item_def_id)
        .and_then(cimmeria_entity::name_intern::intern)
}

/// Why a request did not go through.
#[derive(Debug)]
pub enum Failure {
    /// A rule refused it; nothing changed.
    Refused(BMError),
    /// A database call failed at `stage`; the transaction rolled back.
    Db { stage: &'static str, error: String },
}

impl Failure {
    /// The id the client is sent.
    pub fn error(&self) -> BMError {
        match self {
            Failure::Refused(e) => *e,
            Failure::Db { .. } => BMError::Internal,
        }
    }
}

impl From<BMError> for Failure {
    fn from(e: BMError) -> Self {
        Failure::Refused(e)
    }
}

/// `map_err` adapter: a failed query at `stage`.
pub fn db(stage: &'static str) -> impl FnOnce(sqlx::Error) -> Failure {
    move |error| Failure::Db {
        stage,
        error: error.to_string(),
    }
}

/// Log a failed request (refusal at INFO, server failure at ERROR) and
/// count it. `detail` is any extra context the handler has (`auction_id`,
/// `item_id`).
pub fn log_failure(op: &'static str, actor: &Actor, auction_id: Option<i32>, f: &Failure) {
    let error = f.error();
    match f {
        Failure::Refused(_) => tracing::info!(
            event = "bm.refused",
            op,
            entity_id = actor.entity_id,
            entity_name = actor.player_name,
            account_id = actor.account_id,
            account_name = actor.account_name,
            player_id = actor.player_id,
            player_name = actor.player_name,
            auction_id, // nt:id-only auctions have no name column; the item is not loaded on a refusal
            reason = error.reason(),
            error_id = error.id(), // nt:id-only wire id of the BMError, named by bm_error
            bm_error = ?error,
            "Black Market request refused"
        ),
        Failure::Db { stage, error: e } => tracing::error!(
            event = "bm.refused",
            op,
            entity_id = actor.entity_id,
            entity_name = actor.player_name,
            account_id = actor.account_id,
            account_name = actor.account_name,
            player_id = actor.player_id,
            player_name = actor.player_name,
            auction_id, // nt:id-only auctions have no name column; the item is not loaded on a refusal
            reason = error.reason(),
            error_id = error.id(), // nt:id-only wire id of the BMError, named by bm_error
            bm_error = ?error,
            stage,
            error = %e,
            "Black Market request failed in the database; nothing changed"
        ),
    }
    count_bm_outcome(op, error.reason());
}

/// One state transition. `who` is the actor (for a sweep settlement, the
/// seller). The auction's item is named from its type; the seller and bidder
/// are named only when `who` is that player.
pub fn log_transition(
    event: &'static str,
    who: &Who,
    before: Option<&AuctionRow>,
    after: &AuctionRow,
) {
    tracing::debug!(
        event,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        auction_id = after.sequence_id, // nt:id-only auctions have no name column; item_name names the listing
        seller_id = after.seller_id,
        seller_name = who.name_of(after.seller_id),
        bidder_id = after.current_bidder,
        bidder_name = after.current_bidder.and_then(|b| who.name_of(b)),
        bid_before = before.map(|b| b.current_bid),
        bid_after = after.current_bid,
        escrow_cash_before = before.map(AuctionRow::escrowed_cash),
        escrow_cash_after = after.escrowed_cash(),
        item_type_id = after.item_def_id,
        item_id = after.item_id,
        item_name = item_name(after.item_def_id),
        status = after.status,
        expires_at = after.expires_at,
        "Black Market transition"
    );
}

/// The outbid bidder got their held cash back.
pub fn log_outbid_refund(actor: &Actor, before: &AuctionRow, refunded_to: i32, amount: i32) {
    let who = actor.who();
    tracing::debug!(
        event = "bm.outbid_refund",
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        auction_id = before.sequence_id, // nt:id-only auctions have no name column; item_name names the listing
        seller_id = before.seller_id,
        seller_name = who.name_of(before.seller_id),
        bidder_id = refunded_to,
        bidder_name = who.name_of(refunded_to),
        refunded_player_id = refunded_to,
        refunded_player_name = who.name_of(refunded_to),
        amount,
        bid_before = before.current_bid,
        escrow_cash_before = before.escrowed_cash(),
        escrow_cash_after = 0i64,
        item_type_id = before.item_def_id,
        item_name = item_name(before.item_def_id),
        "Black Market held bid refunded"
    );
}
