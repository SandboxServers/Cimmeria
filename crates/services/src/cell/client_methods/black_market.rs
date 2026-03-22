//! SGWBlackMarketManager interface ClientMethods (indices 90–95).

/// Open the black market UI.
pub const ON_BM_OPEN: u16 = 90;
/// Black market error.
pub const ON_BM_ERROR: u16 = 91;
/// Auction listing results.
pub const ON_BM_AUCTIONS: u16 = 92;
/// Remove an auction from the list.
pub const ON_BM_AUCTION_REMOVE: u16 = 93;
/// Update an existing auction.
pub const ON_BM_AUCTION_UPDATE: u16 = 94;
/// Watched items list changed.
pub const ON_BM_WATCHED_ITEMS_UPDATE: u16 = 95;
