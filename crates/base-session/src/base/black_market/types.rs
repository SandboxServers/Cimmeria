//! Base-side types for the Black Market / Auction House (`SGWBlackMarket`):
//! the `sgw_auction` row model and its status values.
//!
//! [`BMSearchOptions`], the `BMSearch` argument the cell decodes and forwards,
//! is part of the Base<->Cell contract, so it lives in `cimmeria-wire`
//! (`cimmeria_wire::black_market`) and is re-exported here.

pub use cimmeria_wire::black_market::BMSearchOptions;

/// A row of `sgw_auction`, mirroring the table columns 1:1.
///
/// Status values: `0 = active, 1 = sold, 2 = cancelled, 3 = expired`. Time
/// columns are unix epoch seconds. `current_bidder` is the player_id of the
/// current high bidder (whose bid cash is held in escrow), or `None` if no bid
/// has landed yet.
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

/// Auction lifecycle status, matching `sgw_auction.status`.
pub mod auction_status {
    pub const ACTIVE: i16 = 0;
    pub const SOLD: i16 = 1;
    pub const CANCELLED: i16 = 2;
    pub const EXPIRED: i16 = 3;
}
