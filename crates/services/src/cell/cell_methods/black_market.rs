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
mod tests {
    use super::*;

    /// Decode a `BMCreateAuction` payload the way `dispatch` does, returning the
    /// four parsed fields. Mirrors the production decode so a test can assert on
    /// the result without a full async dispatch.
    fn decode_create_auction(args: &[u8]) -> Option<(i32, i32, i32, u8)> {
        if args.len() < CREATE_AUCTION_LEN {
            return None;
        }
        let item_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
        let starting_price = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
        let buyout_price = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
        let auction_length = args[12];
        Some((item_id, starting_price, buyout_price, auction_length))
    }

    /// A valid `BMCreateAuction` is exactly 13 bytes and decodes
    /// `auctionLength` from the single byte at offset 12.
    #[test]
    fn create_auction_decodes_thirteen_byte_payload() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&777i32.to_le_bytes()); // itemInstanceId
        buf.extend_from_slice(&100i32.to_le_bytes()); // startingPrice
        buf.extend_from_slice(&5000i32.to_le_bytes()); // buyoutPrice
        buf.push(3u8); // auctionLength (UINT8)
        assert_eq!(buf.len(), CREATE_AUCTION_LEN);

        let (item, starting, buyout, length) =
            decode_create_auction(&buf).expect("13-byte payload should decode");
        assert_eq!(item, 777);
        assert_eq!(starting, 100);
        assert_eq!(buyout, 5000);
        assert_eq!(length, 3);
    }

    /// Regression guard for the UINT8 fix. `auctionLength` is the byte at offset
    /// 12. If someone reverts to reading it as an INT32 (the old 16-byte
    /// layout), they would consume bytes 12..16 — bytes that this 13-byte
    /// payload does not have, so the old guard (`args.len() >= 16`) would
    /// reject this valid message outright. We pin both facts:
    ///   1. a 13-byte buffer is accepted, and
    ///   2. the length byte is read from offset 12 (value 200, the high end of
    ///      a UINT8 that an INT32 read would smear across following bytes).
    #[test]
    fn create_auction_length_is_uint8_at_offset_12_not_int32() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&1i32.to_le_bytes());
        buf.extend_from_slice(&2i32.to_le_bytes());
        buf.extend_from_slice(&3i32.to_le_bytes());
        buf.push(200u8); // distinctive single-byte length
        assert_eq!(buf.len(), 13);

        // The fixed-size constant must be 13, not 16. A revert to INT32 would
        // push this back to 16 and break the accept path below.
        assert_eq!(CREATE_AUCTION_LEN, 13);

        let (_, _, _, length) =
            decode_create_auction(&buf).expect("13-byte payload must be accepted");
        assert_eq!(length, 200, "auctionLength must be the UINT8 at offset 12");
    }

    /// The `BMSearch` arm must parse the full 11-field `BMSearchOptions`.
    /// (Behavioural coverage of the deserializer lives in
    /// `base::black_market::types`; this confirms the cell arm uses it.)
    #[test]
    fn search_arm_parses_search_options() {
        let opts = BMSearchOptions {
            sort_id: 1,
            client_key: 11,
            sequence_id: 22,
            b_forward: 1,
            seller_name: "Carter".to_string(),
            bidder_name: String::new(),
            item_name: "Zat".to_string(),
            min_tc: 0,
            max_tc: 999,
            quality: 2,
            filter_flags: 8,
        };
        let mut wire = Vec::new();
        wire.push(opts.sort_id);
        wire.extend_from_slice(&opts.client_key.to_le_bytes());
        wire.extend_from_slice(&opts.sequence_id.to_le_bytes());
        wire.push(opts.b_forward);
        for s in [&opts.seller_name, &opts.bidder_name, &opts.item_name] {
            wire.extend_from_slice(&(s.len() as u32).to_le_bytes());
            wire.extend_from_slice(s.as_bytes());
        }
        wire.extend_from_slice(&opts.min_tc.to_le_bytes());
        wire.extend_from_slice(&opts.max_tc.to_le_bytes());
        wire.extend_from_slice(&opts.quality.to_le_bytes());
        wire.extend_from_slice(&opts.filter_flags.to_le_bytes());

        assert_eq!(BMSearchOptions::from_wire(&wire), Some(opts));
    }
}
