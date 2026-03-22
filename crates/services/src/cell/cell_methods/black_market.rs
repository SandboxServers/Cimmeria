//! SGWBlackMarketManager interface exposed CellMethods (indices 61–66).

use tokio::sync::mpsc;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub const SEARCH: u16 = 61;
pub const CREATE_AUCTION: u16 = 62;
pub const PLACE_BID: u16 = 63;
pub const CANCEL_AUCTION: u16 = 64;
pub const START_WATCHING: u16 = 65;
pub const STOP_WATCHING: u16 = 66;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    _tx: &mpsc::Sender<CellToBaseMsg>,
    _space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SEARCH => {
            // BMSearchOptions is a complex struct, just log arrival
            tracing::info!(entity_id, "UNIMPLEMENTED: BMSearch");
            true
        }
        CREATE_AUCTION => {
            if args.len() >= 16 {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let initial_bid = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let buyout_price = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let duration_days = i32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::info!(entity_id, item_id, initial_bid, buyout_price, duration_days, "UNIMPLEMENTED: BMCreateAuction");
            }
            true
        }
        PLACE_BID => {
            if args.len() >= 8 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let bid_amount = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, auction_id, bid_amount, "UNIMPLEMENTED: BMPlaceBid");
            }
            true
        }
        CANCEL_AUCTION => {
            if args.len() >= 4 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, auction_id, "UNIMPLEMENTED: BMCancelAuction");
            }
            true
        }
        START_WATCHING => {
            if args.len() >= 4 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, auction_id, "UNIMPLEMENTED: BMStartWatchingItem");
            }
            true
        }
        STOP_WATCHING => {
            if args.len() >= 4 {
                let auction_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, auction_id, "UNIMPLEMENTED: BMStopWatchingItem");
            }
            true
        }
        _ => false,
    }
}
