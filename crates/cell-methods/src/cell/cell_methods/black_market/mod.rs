//! SGWBlackMarketManager interface exposed CellMethods (indices 61–66).
//!
//! Decodes the client's auction calls and forwards them to the base as
//! `CellToBaseMsg::BlackMarket`; nothing about an auction is decided cell-side. The
//! base handlers are `cimmeria_base_session::base::black_market`.

use crate::cell::messages::{BlackMarketCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::black_market::BMSearchOptions;
use tokio::sync::mpsc;

pub use cimmeria_wire::cell::cell_methods::black_market::{
    CANCEL_AUCTION, CREATE_AUCTION, PLACE_BID, SEARCH, START_WATCHING, STOP_WATCHING,
};

/// Wire size of the `BMCreateAuction` payload: `INT32 itemInstanceId,
/// INT32 startingPrice, INT32 buyoutPrice, UINT8 auctionLength`.
///
/// 13 bytes (4+4+4+1) — NOT 16. The client emitter packs `auctionLength` as a
/// single byte; reading it as an INT32 (the old behaviour) both over-reads by
/// 3 bytes and corrupts the duration value.
const CREATE_AUCTION_LEN: usize = 13;

/// Resolve the player_id for a Black Market routing entity, refusing to fall
/// back to 0. Auction ops keyed on player_id=0 would target a sentinel row, so
/// returning `None` makes the caller bail + log rather than misroute.
fn resolve_player_id(id: PlayerIdentity, entity_id: u32, op: &str) -> Option<i32> {
    if id.player_id.is_none() {
        tracing::warn!(
            entity_id,
            account_id = id.account_id,
            op,
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

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    // The router offers every index from 61 up to this interface before the
    // SGWPlayer range, so filter first: only a real BM call opens a span.
    if !matches!(
        method_index,
        SEARCH | CREATE_AUCTION | PLACE_BID | CANCEL_AUCTION | START_WATCHING | STOP_WATCHING
    ) {
        return false;
    }
    handle(entity_id, method_index, args, tx, space_mgr).await;
    true
}

/// Decode one SGWBlackMarketManager call (61-66) and forward it to the base.
#[tracing::instrument(
    name = "black_market.cell_call",
    level = "info",
    skip_all,
    fields(
        entity_id,
        method_index,
        account_id = tracing::field::Empty,
        player_id = tracing::field::Empty
    )
)]
async fn handle(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let id = space_mgr.player_identity(entity_id);
    let span = tracing::Span::current();
    if let Some(account_id) = id.account_id {
        span.record("account_id", account_id);
    }
    if let Some(player_id) = id.player_id {
        span.record("player_id", player_id);
    }

    match method_index {
        SEARCH => match BMSearchOptions::from_wire(args) {
            Some(options) => {
                if let Some(player_id) = resolve_player_id(id, entity_id, "search") {
                    let msg = BlackMarketCellToBase::Search {
                        entity_id,
                        player_id,
                        options,
                    };
                    forward(tx, msg, id, entity_id, "BMSearch").await;
                }
            }
            None => {
                tracing::warn!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    arg_len = args.len(),
                    "BMSearch: failed to deserialize BMSearchOptions"
                );
            }
        },
        CREATE_AUCTION => {
            if args.len() >= CREATE_AUCTION_LEN {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let starting_price = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let buyout_price = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                // auctionLength is UINT8 — a single byte at offset 12.
                let auction_length = args[12];
                if let Some(player_id) = resolve_player_id(id, entity_id, "createAuction") {
                    let msg = BlackMarketCellToBase::CreateAuction {
                        entity_id,
                        player_id,
                        item_id,
                        starting_price,
                        buyout_price,
                        auction_length,
                    };
                    forward(tx, msg, id, entity_id, "BMCreateAuction").await;
                }
            } else {
                tracing::warn!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    arg_len = args.len(),
                    "BMCreateAuction: payload too short (need 13 bytes)"
                );
            }
        }
        PLACE_BID => {
            if args.len() >= 8 {
                let sequence_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let bid_amount = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if let Some(player_id) = resolve_player_id(id, entity_id, "placeBid") {
                    let msg = BlackMarketCellToBase::PlaceBid {
                        entity_id,
                        player_id,
                        sequence_id,
                        bid_amount,
                    };
                    forward(tx, msg, id, entity_id, "BMPlaceBid").await;
                }
            } else {
                tracing::warn!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    arg_len = args.len(),
                    "BMPlaceBid: payload too short (need 8 bytes)"
                );
            }
        }
        CANCEL_AUCTION => {
            if args.len() >= 4 {
                let sequence_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                if let Some(player_id) = resolve_player_id(id, entity_id, "cancelAuction") {
                    let msg = BlackMarketCellToBase::CancelAuction {
                        entity_id,
                        player_id,
                        sequence_id,
                    };
                    forward(tx, msg, id, entity_id, "BMCancelAuction").await;
                }
            } else {
                tracing::warn!(
                    entity_id,
                    account_id = id.account_id,
                    player_id = id.player_id,
                    arg_len = args.len(),
                    "BMCancelAuction: payload too short (need 4 bytes)"
                );
            }
        }
        START_WATCHING if args.len() >= 4 => {
            let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            tracing::info!(
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                auction_id,
                "UNIMPLEMENTED: BMStartWatchingItem"
            );
        }
        STOP_WATCHING if args.len() >= 4 => {
            let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            tracing::info!(
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                auction_id,
                "UNIMPLEMENTED: BMStopWatchingItem"
            );
        }
        // A short watch payload, or an index `dispatch` already filtered out.
        _ => {}
    }
}

#[cfg(test)]
mod tests;
