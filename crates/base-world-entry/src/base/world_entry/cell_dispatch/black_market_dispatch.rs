//! Black Market auction dispatch arms (search / create / bid / cancel).
//!
//! Every `BlackMarketCellToBase` variant (carried by
//! `CellToBaseMsg::BlackMarket`) lands here and routes into the
//! [`crate::base::black_market`] state machine.

use crate::base::black_market::{bid, cancel, create, search};
use crate::cell::messages::BlackMarketCellToBase;

use super::DispatchCtx;

/// Route one `CellToBaseMsg::BlackMarket` message to its base handler.
pub(super) async fn route(msg: BlackMarketCellToBase, ctx: &DispatchCtx<'_>) {
    match msg {
        BlackMarketCellToBase::Search {
            entity_id,
            player_id,
            options,
        } => {
            search::handle_search(
                entity_id,
                player_id,
                options,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        BlackMarketCellToBase::CreateAuction {
            entity_id,
            player_id,
            item_id,
            starting_price,
            buyout_price,
            auction_length,
        } => {
            create::handle_create_auction(
                entity_id,
                player_id,
                item_id,
                starting_price,
                buyout_price,
                auction_length,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        BlackMarketCellToBase::PlaceBid {
            entity_id,
            player_id,
            sequence_id,
            bid_amount,
        } => {
            bid::handle_place_bid(
                entity_id,
                player_id,
                sequence_id,
                bid_amount,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        BlackMarketCellToBase::CancelAuction {
            entity_id,
            player_id,
            sequence_id,
        } => {
            cancel::handle_cancel_auction(
                entity_id,
                player_id,
                sequence_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
    }
}
