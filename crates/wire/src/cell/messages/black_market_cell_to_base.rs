//! `BlackMarketCellToBase`: Black Market auction traffic from the cell to the
//! base, carried by `CellToBaseMsg::BlackMarket`.
//!
//! The cell only decodes cell methods 61-64 and forwards them; every auction
//! decision (escrow, cash, listing state) is the base's. Each variant carries
//! the acting player's `player_id` and `entity_id` **from the cell's own
//! session state** (`CellEntity`), never from the client payload.

use crate::black_market::BMSearchOptions;

/// Black Market messages sent from CellApp to BaseApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlackMarketCellToBase {
    /// Search active listings (cell method 61 `BMSearch`).
    /// Base queries `sgw_auction WHERE status = 0` and replies with `onBMAuctions` (92).
    Search {
        entity_id: u32,
        player_id: i32,
        options: BMSearchOptions,
    },

    /// Create a listing (cell method 62 `BMCreateAuction`).
    /// Base escrows the item, inserts the `sgw_auction` row, replies with
    /// `onBMAuctionUpdate`. `auction_length` is the UINT8 duration enum (D.5).
    CreateAuction {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        starting_price: i32,
        buyout_price: i32,
        auction_length: u8,
    },

    /// Bid on an active auction (cell method 63 `BMPlaceBid`).
    /// Base refunds the prior bidder, holds the new bid, pushes `onBMAuctionUpdate`.
    PlaceBid {
        entity_id: u32,
        player_id: i32,
        sequence_id: i32,
        bid_amount: i32,
    },

    /// Cancel an owned auction (cell method 64 `BMCancelAuction`).
    /// Base returns the escrowed item, refunds the current bidder, pushes
    /// `onBMAuctionRemove`.
    CancelAuction {
        entity_id: u32,
        player_id: i32,
        sequence_id: i32,
    },
}
