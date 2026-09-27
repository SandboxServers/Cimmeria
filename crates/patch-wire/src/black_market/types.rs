//! The two Black Market `FIXED_DICT`s from `entities/defs/alias.xml`.

use super::MAX_STRING_LEN;
use crate::codec::{read_i32, read_string, read_u8, write_i32, write_string, write_u8};
use crate::{ByteSource, Decode, DecodeError, Encode, EncodeError};

/// One auction row: the `AuctionItem` `FIXED_DICT`.
///
/// Wire order: seven `INT32`s (`sequenceId` … `buyoutPrice`), `UINT8
/// endTimeValue`, `INT32 nextMinBidPrice`, `STRING sellerName`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuctionItem {
    /// `sequenceId`: the auction's id.
    pub sequence_id: i32,
    /// `itemDefId`: the item definition on sale.
    pub item_def_id: i32,
    /// `stackSize`.
    pub stack_size: i32,
    /// `durability`.
    pub durability: i32,
    /// `charges`.
    pub charges: i32,
    /// `currentBid`: the standing bid, 0 when there is none.
    pub current_bid: i32,
    /// `buyoutPrice`: 0 when there is no buyout.
    pub buyout_price: i32,
    /// `endTimeValue`: the time-left bucket, a
    /// [`UIAuctionTime`](super::UIAuctionTime) value (1–5) that drives the
    /// row's timer icon. Not the listing's original duration.
    pub end_time_value: u8,
    /// `nextMinBidPrice`: the lowest bid the server will accept next. The
    /// client only displays it.
    pub next_min_bid_price: i32,
    /// `sellerName`: a narrow `STRING`.
    pub seller_name: String,
}

impl AuctionItem {
    /// Wire size of an `AuctionItem` with an empty `sellerName`: 7 × 4 + 1 +
    /// 4 + 4. Used to bound an array count against the bytes left.
    pub const MIN_WIRE_LEN: usize = 37;
}

impl Encode for AuctionItem {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        write_i32(out, self.sequence_id);
        write_i32(out, self.item_def_id);
        write_i32(out, self.stack_size);
        write_i32(out, self.durability);
        write_i32(out, self.charges);
        write_i32(out, self.current_bid);
        write_i32(out, self.buyout_price);
        write_u8(out, self.end_time_value);
        write_i32(out, self.next_min_bid_price);
        write_string(out, &self.seller_name, "sellerName", MAX_STRING_LEN)
    }
}

impl Decode for AuctionItem {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        Ok(Self {
            sequence_id: read_i32(src, "sequenceId")?,
            item_def_id: read_i32(src, "itemDefId")?,
            stack_size: read_i32(src, "stackSize")?,
            durability: read_i32(src, "durability")?,
            charges: read_i32(src, "charges")?,
            current_bid: read_i32(src, "currentBid")?,
            buyout_price: read_i32(src, "buyoutPrice")?,
            end_time_value: read_u8(src, "endTimeValue")?,
            next_min_bid_price: read_i32(src, "nextMinBidPrice")?,
            seller_name: read_string(src, "sellerName", MAX_STRING_LEN)?,
        })
    }
}

/// The search filter: the `BMSearchOptions` `FIXED_DICT`, the single
/// argument of `BMSearch`.
///
/// Wire order: `UINT8 sortId`, `INT32 clientKey`, `INT32 sequenceId`,
/// `UINT8 bForward`, `STRING sellerName`, `STRING bidderName`, `STRING
/// itemName`, `INT32 minTC`, `INT32 maxTC`, `INT32 quality`, `INT32
/// monikerCRC`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BMSearchOptions {
    /// `sortId`: the sort column (`EBlackMarketSortType`).
    pub sort_id: u8,
    /// `clientKey`: which list the reply fills, a
    /// [`UIAuctionView`](super::UIAuctionView) value. The server echoes it
    /// in `onBMAuctions`.
    pub client_key: i32,
    /// `sequenceId`: paging cursor, the last auction id the client saw.
    pub sequence_id: i32,
    /// `bForward`: non-zero pages forward from `sequence_id`.
    pub b_forward: u8,
    /// `sellerName`: set to the player's own name by `refreshMyAuctions`.
    pub seller_name: String,
    /// `bidderName`: set to the player's own name by `refreshMyBids`.
    pub bidder_name: String,
    /// `itemName`: name filter, empty for none.
    pub item_name: String,
    /// `minTC`: minimum tech competency.
    pub min_tc: i32,
    /// `maxTC`: maximum tech competency.
    pub max_tc: i32,
    /// `quality`: the client's default is 2000.
    pub quality: i32,
    /// The eleventh field. The `.def` names it `monikerCRC`; the client's
    /// emitter stores it under the name `filterFlags` (an
    /// `EBlackMarketFilter` bitfield). The shipped UI always sends 0.
    pub filter_flags: i32,
}

impl Encode for BMSearchOptions {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        write_u8(out, self.sort_id);
        write_i32(out, self.client_key);
        write_i32(out, self.sequence_id);
        write_u8(out, self.b_forward);
        write_string(out, &self.seller_name, "sellerName", MAX_STRING_LEN)?;
        write_string(out, &self.bidder_name, "bidderName", MAX_STRING_LEN)?;
        write_string(out, &self.item_name, "itemName", MAX_STRING_LEN)?;
        write_i32(out, self.min_tc);
        write_i32(out, self.max_tc);
        write_i32(out, self.quality);
        write_i32(out, self.filter_flags);
        Ok(())
    }
}

impl Decode for BMSearchOptions {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        Ok(Self {
            sort_id: read_u8(src, "sortId")?,
            client_key: read_i32(src, "clientKey")?,
            sequence_id: read_i32(src, "sequenceId")?,
            b_forward: read_u8(src, "bForward")?,
            seller_name: read_string(src, "sellerName", MAX_STRING_LEN)?,
            bidder_name: read_string(src, "bidderName", MAX_STRING_LEN)?,
            item_name: read_string(src, "itemName", MAX_STRING_LEN)?,
            min_tc: read_i32(src, "minTC")?,
            max_tc: read_i32(src, "maxTC")?,
            quality: read_i32(src, "quality")?,
            filter_flags: read_i32(src, "monikerCRC")?,
        })
    }
}
