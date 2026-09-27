//! Client-to-server Black Market methods (`SGWPlayer` exposed cell indices
//! 61–66). The stock client's emitters for these were never wired up; the
//! client-patch DLL sends them itself, and the server decodes them with the
//! same types.

use super::{int32_args, BMSearchOptions, CellMethod};
use crate::codec::{decode_whole, read_i32, read_u8, write_i32, write_u8};
use crate::{ByteSource, Decode, DecodeError, Encode, EncodeError};

/// `BMSearch(BMSearchOptions searchOptions)`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BMSearch {
    /// `searchOptions`.
    pub search_options: BMSearchOptions,
}

impl Encode for BMSearch {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        self.search_options.write_to(out)
    }
}

impl Decode for BMSearch {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        Ok(Self {
            search_options: BMSearchOptions::decode_from(src)?,
        })
    }
}

/// `BMCreateAuction(INT32 itemInstanceId, INT32 buyoutPrice, UINT8
/// auctionLength, INT32 startingPrice)`: 13 bytes.
///
/// This is the `.def` order. The client's emitter stores the four values
/// under their `.def` names in a different order (item, starting, buyout,
/// length); that is insertion order, not wire order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BMCreateAuction {
    /// `itemInstanceId`: the inventory item to list.
    pub item_instance_id: i32,
    /// `buyoutPrice`: 0 for no buyout.
    pub buyout_price: i32,
    /// `auctionLength`: a [`UIAuctionTime`](super::UIAuctionTime) value,
    /// 1-based. The create form sends 3, 4 or 5.
    pub auction_length: u8,
    /// `startingPrice`.
    pub starting_price: i32,
}

impl BMCreateAuction {
    /// The payload size: 4 + 4 + 1 + 4.
    pub const WIRE_LEN: usize = 13;
}

impl Encode for BMCreateAuction {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        write_i32(out, self.item_instance_id);
        write_i32(out, self.buyout_price);
        write_u8(out, self.auction_length);
        write_i32(out, self.starting_price);
        Ok(())
    }
}

impl Decode for BMCreateAuction {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        Ok(Self {
            item_instance_id: read_i32(src, "itemInstanceId")?,
            buyout_price: read_i32(src, "buyoutPrice")?,
            auction_length: read_u8(src, "auctionLength")?,
            starting_price: read_i32(src, "startingPrice")?,
        })
    }
}

int32_args! {
    /// `BMPlaceBid(INT32 sequenceId, INT32 bidAmount)`.
    BMPlaceBid {
        /// The auction.
        sequence_id = "sequenceId",
        /// The bid.
        bid_amount = "bidAmount",
    }
}

int32_args! {
    /// `BMCancelAuction(INT32 sequenceId)`.
    BMCancelAuction {
        /// The auction to cancel.
        sequence_id = "sequenceId",
    }
}

int32_args! {
    /// `BMStartWatchingItem(INT32 itemDefId)`.
    BMStartWatchingItem {
        /// The item definition to watch.
        item_def_id = "itemDefId",
    }
}

int32_args! {
    /// `BMStopWatchingItem(INT32 itemDefId)`.
    BMStopWatchingItem {
        /// The item definition to stop watching.
        item_def_id = "itemDefId",
    }
}

/// Any one Black Market cell method call, tagged by method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellCall {
    /// `BMSearch`.
    Search(BMSearch),
    /// `BMCreateAuction`.
    CreateAuction(BMCreateAuction),
    /// `BMPlaceBid`.
    PlaceBid(BMPlaceBid),
    /// `BMCancelAuction`.
    CancelAuction(BMCancelAuction),
    /// `BMStartWatchingItem`.
    StartWatchingItem(BMStartWatchingItem),
    /// `BMStopWatchingItem`.
    StopWatchingItem(BMStopWatchingItem),
}

impl CellCall {
    /// The method this call is.
    pub fn method(&self) -> CellMethod {
        match self {
            Self::Search(_) => CellMethod::BMSearch,
            Self::CreateAuction(_) => CellMethod::BMCreateAuction,
            Self::PlaceBid(_) => CellMethod::BMPlaceBid,
            Self::CancelAuction(_) => CellMethod::BMCancelAuction,
            Self::StartWatchingItem(_) => CellMethod::BMStartWatchingItem,
            Self::StopWatchingItem(_) => CellMethod::BMStopWatchingItem,
        }
    }

    /// Decode the arguments of `method` from `src`. Consumes exactly the
    /// arguments; see [`Decode::decode_from`].
    pub fn decode_from<S: ByteSource + ?Sized>(
        method: CellMethod,
        src: &mut S,
    ) -> Result<Self, DecodeError> {
        Ok(match method {
            CellMethod::BMSearch => Self::Search(BMSearch::decode_from(src)?),
            CellMethod::BMCreateAuction => Self::CreateAuction(BMCreateAuction::decode_from(src)?),
            CellMethod::BMPlaceBid => Self::PlaceBid(BMPlaceBid::decode_from(src)?),
            CellMethod::BMCancelAuction => Self::CancelAuction(BMCancelAuction::decode_from(src)?),
            CellMethod::BMStartWatchingItem => {
                Self::StartWatchingItem(BMStartWatchingItem::decode_from(src)?)
            }
            CellMethod::BMStopWatchingItem => {
                Self::StopWatchingItem(BMStopWatchingItem::decode_from(src)?)
            }
        })
    }

    /// Decode a complete `method` payload, rejecting trailing bytes; see
    /// [`Decode::decode`].
    pub fn decode(method: CellMethod, bytes: &[u8]) -> Result<Self, DecodeError> {
        decode_whole(bytes, |src| Self::decode_from(method, src))
    }
}

impl Encode for CellCall {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Self::Search(args) => args.write_to(out),
            Self::CreateAuction(args) => args.write_to(out),
            Self::PlaceBid(args) => args.write_to(out),
            Self::CancelAuction(args) => args.write_to(out),
            Self::StartWatchingItem(args) => args.write_to(out),
            Self::StopWatchingItem(args) => args.write_to(out),
        }
    }
}
