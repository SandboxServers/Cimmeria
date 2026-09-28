//! Base-side types for the Black Market / Auction House (`SGWBlackMarket`):
//! the `sgw_auction` row model, its status values, and the listing rules.
//!
//! [`BMSearchOptions`], the `BMSearch` argument the cell decodes and forwards,
//! is part of the shared codec (`cimmeria-patch-wire`, re-exported through
//! `cimmeria_wire::black_market`) and is re-exported here.

use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};

pub use cimmeria_wire::black_market::BMSearchOptions;

/// The `sgw_auction` column list in [`AuctionRow`] field order, for every
/// `SELECT` / `RETURNING` that decodes a row. `sqlx::query` needs a
/// `&'static str`, so the SQL is built with `concat!`.
macro_rules! auction_columns {
    () => {
        "sequence_id, seller_id, item_id, item_def_id, stack_size, durability, \
         charges, starting_price, buyout_price, current_bid, current_bidder, \
         auction_length, created_at, expires_at, status"
    };
}
pub(crate) use auction_columns;

/// A row of `sgw_auction`, mirroring the table columns 1:1.
///
/// Status values: `0 = active, 1 = sold, 2 = cancelled, 3 = expired`. Time
/// columns are unix epoch seconds. `current_bidder` is the player_id of the
/// current high bidder (whose bid cash is held in escrow), or `None` if no bid
/// has landed yet. `item_id` is the listed `sgw_inventory` row, which sits in
/// the seller's container 18 while the auction is open (0 for a boot-seed
/// listing, which has no instance).
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AuctionRow {
    pub sequence_id: i32,
    pub seller_id: i32,
    pub item_id: i32,
    pub item_def_id: i32,
    pub stack_size: i32,
    pub durability: i32,
    pub charges: i32,
    pub starting_price: i32,
    pub buyout_price: i32,
    pub current_bid: i32,
    pub current_bidder: Option<i32>,
    pub auction_length: i16,
    pub created_at: i32,
    pub expires_at: i32,
    pub status: i16,
}

impl AuctionRow {
    /// The bidder's cash the auction holds: the standing bid, or 0 with no
    /// bidder.
    pub fn escrowed_cash(&self) -> i64 {
        if self.current_bidder.is_some() {
            i64::from(self.current_bid)
        } else {
            0
        }
    }
}

/// Auction lifecycle status, matching `sgw_auction.status`.
pub mod auction_status {
    pub const ACTIVE: i16 = 0;
    pub const SOLD: i16 = 1;
    pub const CANCELLED: i16 = 2;
    pub const EXPIRED: i16 = 3;
}

/// Most active listings one seller may have (decision D5; no listing fee).
pub const MAX_ACTIVE_LISTINGS: i64 = 20;

/// The bags an item may be listed from, and returned to: the carried bags.
/// Equipped slots, the bandolier, the mission bag and every vault are not
/// sale stock. A returned item goes to the first of these with a free slot
/// (grants place crafting items in either, so both are valid homes).
pub const LISTABLE_BAGS: [i32; 2] = [INV_MAIN, INV_CRAFTING];
