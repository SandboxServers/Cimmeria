//! SGWBlackMarketManager interface exposed CellMethods (indices 61–66).
//!
//! The handlers are in `cimmeria_services::cell::cell_methods::black_market`,
//! which re-exports these constants.

pub const SEARCH: u16 = 61;
pub const CREATE_AUCTION: u16 = 62;
pub const PLACE_BID: u16 = 63;
pub const CANCEL_AUCTION: u16 = 64;
pub const START_WATCHING: u16 = 65;
pub const STOP_WATCHING: u16 = 66;
