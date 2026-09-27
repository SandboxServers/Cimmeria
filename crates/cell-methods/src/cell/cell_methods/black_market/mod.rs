//! SGWBlackMarketManager interface exposed CellMethods (indices 61–66).
//!
//! Decodes the client's auction calls with the shared codec
//! (`cimmeria-patch-wire`, re-exported as `cimmeria_wire::black_market`) and
//! forwards them to the base as `CellToBaseMsg::BlackMarket`; nothing about
//! an auction is decided cell-side except who may ask. The base handlers are
//! `cimmeria_base_session::base::black_market`.
//!
//! **Authority (BM-02, CWE-862).** `BMCreateAuction`, `BMPlaceBid` and
//! `BMCancelAuction` move items and cash, so each is forwarded only while
//! [`black_market_access`] passes: the player was sent to an auctioneer by
//! the server's `open_black_market` action, is still interacting with it,
//! and is within interact distance. A refusal answers `onBMError`
//! (`NotAtAuctioneer`) so the press gets visible feedback. `BMSearch` is
//! read-only and needs no auctioneer; its My Auctions / My Bids views are
//! scoped to the caller's own `player_id` on the base (S3).
//!
//! The watch list (65/66) is deferred (decision D4) and answers
//! `onBMError(WatchUnavailable)`.

use crate::cell::black_market::{black_market_access, count_bm_outcome, BlackMarketReject};
use crate::cell::messages::{BlackMarketCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::black_market::{serialize_on_bm_error, BMError, CellCall, CellMethod};
use cimmeria_wire::cell::client_methods::black_market::ON_BM_ERROR;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::black_market::{
    CANCEL_AUCTION, CREATE_AUCTION, PLACE_BID, SEARCH, START_WATCHING, STOP_WATCHING,
};

/// The `op` label of a method, for logs and `bm_outcome_total`.
fn op_label(method: CellMethod) -> &'static str {
    match method {
        CellMethod::BMSearch => "search",
        CellMethod::BMCreateAuction => "create",
        CellMethod::BMPlaceBid => "bid",
        CellMethod::BMCancelAuction => "cancel",
        CellMethod::BMStartWatchingItem | CellMethod::BMStopWatchingItem => "watch",
    }
}

/// Resolve the player_id for a Black Market routing entity, refusing to fall
/// back to 0. Auction ops keyed on player_id=0 would target a sentinel row, so
/// returning `None` makes the caller bail + log rather than misroute.
fn resolve_player_id(id: PlayerIdentity, entity_id: u32, op: &str) -> Option<i32> {
    if id.player_id.is_none() {
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            op,
            reason = "no_player_id",
            "black market op dropped: entity has no player_id"
        );
    }
    id.player_id
}

/// Forward one decoded call to the base. `op` is the client method name, so
/// the warn keeps each arm's `"<op>: base channel closed"` text.
async fn forward(
    tx: &mpsc::Sender<CellToBaseMsg>,
    msg: BlackMarketCellToBase,
    id: PlayerIdentity,
    entity_id: u32,
    op: &str,
) {
    if tx.send(CellToBaseMsg::BlackMarket(msg)).await.is_err() {
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            reason = "base_channel_closed",
            "{op}: base channel closed, player action dropped"
        );
    }
}

/// Answer a refused call with `onBMError(error)` and log it. `access` is
/// the authority rule's label when that is why, so SigNoz shows which leg
/// failed beside `reason = not_at_auctioneer`.
async fn refuse(
    tx: &mpsc::Sender<CellToBaseMsg>,
    id: PlayerIdentity,
    entity_id: u32,
    method: CellMethod,
    error: BMError,
    access: Option<BlackMarketReject>,
) {
    let distance = match access {
        Some(BlackMarketReject::OutOfRange { dist }) => Some(dist),
        _ => None,
    };
    tracing::info!(
        event = "bm.refused",
        entity_id,
        account_id = id.account_id,
        player_id = id.player_id,
        op = op_label(method),
        method = method.name(),
        reason = error.reason(),
        error_id = error.id(),
        access = access.map(BlackMarketReject::label),
        distance,
        "Black Market request refused on the cell"
    );
    count_bm_outcome(op_label(method), error.reason());
    let msg = CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_BM_ERROR,
        args: serialize_on_bm_error(error),
    };
    if tx.send(msg).await.is_err() {
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            player_id = id.player_id,
            reason = "base_channel_closed",
            "onBMError: base channel closed, refusal not shown to the player"
        );
    }
}

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // The router offers every index from 61 up to this interface before the
    // SGWPlayer range, so filter first: only a real BM call opens a span.
    let Some(method) = CellMethod::ALL
        .into_iter()
        .find(|m| m.index() == method_index)
    else {
        return false;
    };
    handle(entity_id, method, args, tx, space_mgr).await;
    true
}

/// Decode one SGWBlackMarketManager call (61-66), check who may make it, and
/// forward it to the base.
#[tracing::instrument(
    name = "black_market.cell_call",
    level = "info",
    skip_all,
    fields(
        entity_id,
        method_index = method.index(),
        account_id = tracing::field::Empty,
        player_id = tracing::field::Empty
    )
)]
async fn handle(
    entity_id: u32,
    method: CellMethod,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let span = tracing::Span::current();
    if let Some(account_id) = id.account_id {
        span.record("account_id", account_id);
    }
    if let Some(player_id) = id.player_id {
        span.record("player_id", player_id);
        // Any 61-66 call, decodable or not, proves the client patch is
        // talking: it clears the `bm.open_without_client_call` signal.
        space_mgr.black_market.note_client_call(player_id);
    }

    let call = match CellCall::decode(method, args) {
        Ok(call) => call,
        Err(e) => {
            tracing::warn!(
                event = "bm.decode_failed",
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                method = method.name(),
                arg_len = args.len(),
                reason = e.kind(),
                error = %e,
                "{}: payload did not decode",
                method.name()
            );
            count_bm_outcome(op_label(method), "decode_failed");
            return;
        }
    };
    let Some(player_id) = resolve_player_id(id, entity_id, method.name()) else {
        return;
    };

    // 62-64 change who owns what: only at an open auctioneer.
    if matches!(
        call,
        CellCall::CreateAuction(_) | CellCall::PlaceBid(_) | CellCall::CancelAuction(_)
    ) {
        if let Err(reject) = black_market_access(entity_id, space_mgr) {
            refuse(
                tx,
                id,
                entity_id,
                method,
                BMError::NotAtAuctioneer,
                Some(reject),
            )
            .await;
            return;
        }
    }

    let msg = match call {
        CellCall::Search(search) => BlackMarketCellToBase::Search {
            entity_id,
            player_id,
            options: search.search_options,
        },
        CellCall::CreateAuction(c) => BlackMarketCellToBase::CreateAuction {
            entity_id,
            player_id,
            item_id: c.item_instance_id,
            starting_price: c.starting_price,
            buyout_price: c.buyout_price,
            auction_length: c.auction_length,
        },
        CellCall::PlaceBid(b) => BlackMarketCellToBase::PlaceBid {
            entity_id,
            player_id,
            sequence_id: b.sequence_id,
            bid_amount: b.bid_amount,
        },
        CellCall::CancelAuction(c) => BlackMarketCellToBase::CancelAuction {
            entity_id,
            player_id,
            sequence_id: c.sequence_id,
        },
        CellCall::StartWatchingItem(_) | CellCall::StopWatchingItem(_) => {
            refuse(tx, id, entity_id, method, BMError::WatchUnavailable, None).await;
            return;
        }
    };
    forward(tx, msg, id, entity_id, method.name()).await;
}

/// `DisconnectEntity`: end the player's Black Market session, and log once
/// if the server opened the window but the client never called 61-66: the
/// server-side sign that the client patch is missing.
pub fn on_disconnect(entity_id: u32, space_mgr: &mut SpaceManager) {
    let id = space_mgr.player_identity(entity_id);
    let Some(player_id) = id.player_id else {
        return;
    };
    let Some(session) = space_mgr.black_market.end(player_id) else {
        return;
    };
    if session.open_without_client_call() {
        tracing::info!(
            event = "bm.open_without_client_call",
            entity_id,
            account_id = id.account_id,
            player_id,
            opens = session.opens,
            auctioneer_entity_id = session.auctioneer_id,
            "Black Market opened this session but the client never called it: \
             the client patch is probably missing"
        );
    }
}

#[cfg(test)]
mod tests;
