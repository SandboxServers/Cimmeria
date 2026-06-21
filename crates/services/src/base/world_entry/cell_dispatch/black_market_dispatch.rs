//! Black Market auction dispatch arms (create / bid / cancel).
//!
//! Each function is the body of one `CellToBaseMsg::BM*` arm carved out of the
//! [`super::handle_cell_message`] match, routing into the
//! [`crate::base::black_market`] state machine.

use crate::base::black_market::{bid, cancel, create};
use crate::cell::messages::CellToBaseMsg;

use super::DispatchCtx;

/// Route the Black Market family of `CellToBaseMsg`.
///
/// Called by [`super::handle_cell_message`] once the outer match has narrowed
/// `msg` to this family; any other variant is `unreachable!` by construction.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::BMCreateAuction {
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
        CellToBaseMsg::BMPlaceBid {
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
        CellToBaseMsg::BMCancelAuction {
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
        other => unreachable!("black_market_dispatch::route got non-BM variant: {other:?}"),
    }
}
