//! Black Market telemetry (plan §5.1, BM-02 line): one DEBUG event per state
//! transition, one row per refusal, and the `bm_outcome_total` counter.
//!
//! - **Transitions**: `event = "bm.<transition>"` (`bm.listed`, `bm.bid`,
//!   `bm.outbid_refund`, `bm.cancelled`, `bm.sold`, `bm.expired`), each with
//!   `auction_id`, `seller_id`, `bidder_id`, `bid_before` / `bid_after`,
//!   `escrow_cash_before` / `escrow_cash_after` (the bidder cash the auction
//!   holds) and `item_def_id`, plus the actor's `account_id` and `player_id`.
//! - **Refusals**: `event = "bm.refused"` at INFO with `reason` equal to the
//!   [`BMError::reason`] of the id the client got, and `error_id`.
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

/// The player a request came from: the cell's entity and `player_id`, and
/// the session's `account_id` (`None` only when the session is gone).
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: i32,
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
        let account_id =
            session_identity::identity_for_entity(connected, entity_to_addr, entity_id).account_id;
        if let Some(account_id) = account_id {
            tracing::Span::current().record("account_id", account_id);
        }
        Self {
            entity_id,
            account_id,
            player_id,
        }
    }
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
            account_id = actor.account_id,
            player_id = actor.player_id,
            auction_id,
            reason = error.reason(),
            error_id = error.id(),
            bm_error = ?error,
            "Black Market request refused"
        ),
        Failure::Db { stage, error: e } => tracing::error!(
            event = "bm.refused",
            op,
            entity_id = actor.entity_id,
            account_id = actor.account_id,
            player_id = actor.player_id,
            auction_id,
            reason = error.reason(),
            error_id = error.id(),
            bm_error = ?error,
            stage,
            error = %e,
            "Black Market request failed in the database; nothing changed"
        ),
    }
    count_bm_outcome(op, error.reason());
}

/// One state transition. `account_id` / `player_id` are the actor's (for a
/// sweep settlement, the seller's).
pub fn log_transition(
    event: &'static str,
    account_id: Option<u32>,
    player_id: i32,
    before: Option<&AuctionRow>,
    after: &AuctionRow,
) {
    tracing::debug!(
        event,
        account_id,
        player_id,
        auction_id = after.sequence_id,
        seller_id = after.seller_id,
        bidder_id = after.current_bidder,
        bid_before = before.map(|b| b.current_bid),
        bid_after = after.current_bid,
        escrow_cash_before = before.map(AuctionRow::escrowed_cash),
        escrow_cash_after = after.escrowed_cash(),
        item_def_id = after.item_def_id,
        item_id = after.item_id,
        status = after.status,
        expires_at = after.expires_at,
        "Black Market transition"
    );
}

/// The outbid bidder got their held cash back.
pub fn log_outbid_refund(actor: &Actor, before: &AuctionRow, refunded_to: i32, amount: i32) {
    tracing::debug!(
        event = "bm.outbid_refund",
        account_id = actor.account_id,
        player_id = actor.player_id,
        auction_id = before.sequence_id,
        seller_id = before.seller_id,
        bidder_id = refunded_to,
        refunded_player_id = refunded_to,
        amount,
        bid_before = before.current_bid,
        escrow_cash_before = before.escrowed_cash(),
        escrow_cash_after = 0i64,
        item_def_id = before.item_def_id,
        "Black Market held bid refunded"
    );
}
