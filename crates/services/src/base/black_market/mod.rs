//! Black Market / Auction House (`SGWBlackMarket`) base-side support.
//!
//! Phase 1 scope: wire types only. Later phases add the singleton entity,
//! the create/bid/cancel state machine, the expiry sweep, search, and the
//! watch list — each as a sibling module here.

pub mod types;

pub use types::BMSearchOptions;
