//! SGWBlackMarketManager interface exposed CellMethods (indices 61–66).

use crate::base::black_market::BMSearchOptions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use tokio::sync::mpsc;

pub const SEARCH: u16 = 61;
pub const CREATE_AUCTION: u16 = 62;
pub const PLACE_BID: u16 = 63;
pub const CANCEL_AUCTION: u16 = 64;
pub const START_WATCHING: u16 = 65;
pub const STOP_WATCHING: u16 = 66;

/// Wire size of the `BMCreateAuction` payload: `INT32 itemInstanceId,
/// INT32 startingPrice, INT32 buyoutPrice, UINT8 auctionLength`.
///
/// 13 bytes (4+4+4+1) — NOT 16. The client emitter packs `auctionLength` as a
/// single byte; reading it as an INT32 (the old behaviour) both over-reads by
/// 3 bytes and corrupts the duration value.
const CREATE_AUCTION_LEN: usize = 13;

/// Resolve the player_id for a Black Market routing entity, refusing to fall
/// back to 0. Auction ops keyed on player_id=0 would target a sentinel row, so
/// returning `None` makes the caller bail + log rather than misroute.
fn resolve_player_id(entity_id: u32, space_mgr: &SpaceManager, op: &str) -> Option<i32> {
    match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => Some(id),
        None => {
            tracing::warn!(
                entity_id,
                op,
                "black market op dropped: entity has no player_id"
            );
            None
        }
    }
}

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    match method_index {
        SEARCH => {
            match BMSearchOptions::from_wire(args) {
                Some(opts) => {
                    tracing::info!(
                        entity_id,
                        sort_id = opts.sort_id,
                        client_key = opts.client_key,
                        sequence_id = opts.sequence_id,
                        b_forward = opts.b_forward,
                        seller_name = %opts.seller_name,
                        bidder_name = %opts.bidder_name,
                        item_name = %opts.item_name,
                        min_tc = opts.min_tc,
                        max_tc = opts.max_tc,
                        quality = opts.quality,
                        filter_flags = opts.filter_flags,
                        "UNIMPLEMENTED: BMSearch (parsed)"
                    );
                }
                None => {
                    tracing::warn!(
                        entity_id,
                        arg_len = args.len(),
                        "BMSearch: failed to deserialize BMSearchOptions"
                    );
                }
            }
            true
        }
        CREATE_AUCTION => {
            if args.len() >= CREATE_AUCTION_LEN {
                let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let starting_price = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let buyout_price = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                // auctionLength is UINT8 — a single byte at offset 12.
                let auction_length = args[12];
                if let Some(player_id) = resolve_player_id(entity_id, space_mgr, "createAuction") {
                    let _ = tx
                        .send(CellToBaseMsg::BMCreateAuction {
                            entity_id,
                            player_id,
                            item_id,
                            starting_price,
                            buyout_price,
                            auction_length,
                        })
                        .await;
                }
            } else {
                tracing::warn!(
                    entity_id,
                    arg_len = args.len(),
                    "BMCreateAuction: payload too short (need 13 bytes)"
                );
            }
            true
        }
        PLACE_BID => {
            if args.len() >= 8 {
                let sequence_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let bid_amount = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if let Some(player_id) = resolve_player_id(entity_id, space_mgr, "placeBid") {
                    let _ = tx
                        .send(CellToBaseMsg::BMPlaceBid {
                            entity_id,
                            player_id,
                            sequence_id,
                            bid_amount,
                        })
                        .await;
                }
            } else {
                tracing::warn!(
                    entity_id,
                    arg_len = args.len(),
                    "BMPlaceBid: payload too short (need 8 bytes)"
                );
            }
            true
        }
        CANCEL_AUCTION => {
            if args.len() >= 4 {
                let sequence_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                if let Some(player_id) = resolve_player_id(entity_id, space_mgr, "cancelAuction") {
                    let _ = tx
                        .send(CellToBaseMsg::BMCancelAuction {
                            entity_id,
                            player_id,
                            sequence_id,
                        })
                        .await;
                }
            } else {
                tracing::warn!(
                    entity_id,
                    arg_len = args.len(),
                    "BMCancelAuction: payload too short (need 4 bytes)"
                );
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

#[cfg(test)]
mod tests;
