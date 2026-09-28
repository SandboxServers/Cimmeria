//! `BlackMarketCellToBase`: Black Market auction traffic from the cell to the
//! base, carried by `CellToBaseMsg::BlackMarket`.
//!
//! The cell only decodes cell methods 61-64 and forwards them; every auction
//! decision (escrow, cash, listing state) is the base's. Each variant carries
//! the acting player's `player_id` and `entity_id` **from the cell's own
//! session state** (`CellEntity`), never from the client payload.

use crate::black_market::BMSearchOptions;

/// The GM who ran a Black Market `.`-console command (BM-07), as the cell
/// knows them. Sent only after the console gate confirmed the caller's
/// server-side access level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmGmActor {
    /// The GM's entity id, for the feedback lines.
    pub entity_id: u32,
    /// The GM's `sgw_player.player_id`.
    pub player_id: i32,
    /// The GM's `account.account_id`, `None` if the cell has none.
    pub account_id: Option<u32>,
}

/// Black Market messages sent from CellApp to BaseApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlackMarketCellToBase {
    /// Search active listings (cell method 61 `BMSearch`).
    /// Base queries the open listings for the `clientKey` view, scoped to
    /// `player_id` for My Auctions / My Bids, and replies with one page of
    /// `onBMAuctions` (92).
    Search {
        entity_id: u32,
        player_id: i32,
        options: BMSearchOptions,
    },

    /// Create a listing (cell method 62 `BMCreateAuction`).
    /// Base moves the item into escrow (container 18), inserts the
    /// `sgw_auction` row, replies with `onBMAuctionUpdate`. `auction_length`
    /// is the raw UINT8 `UIAuctionTime` (1-based); the base clamps it to 1-5.
    /// Forwarded only from a player at an auctioneer (BM-02).
    CreateAuction {
        entity_id: u32,
        player_id: i32,
        item_id: i32,
        starting_price: i32,
        buyout_price: i32,
        auction_length: u8,
    },

    /// Bid on an active auction (cell method 63 `BMPlaceBid`).
    /// Base refunds the prior bidder, holds the new bid, pushes
    /// `onBMAuctionUpdate`; a bid at the buyout price settles at once.
    PlaceBid {
        entity_id: u32,
        player_id: i32,
        sequence_id: i32,
        bid_amount: i32,
    },

    /// Cancel an owned auction (cell method 64 `BMCancelAuction`).
    /// Base returns the escrowed item from container 18, refunds the current
    /// bidder, pushes `onBMAuctionRemove`.
    CancelAuction {
        entity_id: u32,
        player_id: i32,
        sequence_id: i32,
    },

    /// `.bm_seed [count]` (BM-07): list `count` system-seller auctions
    /// from the UAT set (1-60, the cell checked), after the same system
    /// seller check as the boot seed.
    GmSeed { actor: BmGmActor, count: u8 },

    /// `.bm_expire <auctionId>` (BM-07): make one active auction due now
    /// and settle it at once through the expiry sweep, so a tester sees a
    /// settlement without waiting. `sequence_id > 0` (the cell checked).
    GmExpire { actor: BmGmActor, sequence_id: i32 },

    /// `.bm_list` (BM-07): the newest active auctions, with their ids.
    GmList { actor: BmGmActor },
}
