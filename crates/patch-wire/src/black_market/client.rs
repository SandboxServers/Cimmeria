//! Server-to-client Black Market methods (`SGWPlayer` client indices
//! 90–95). The stock client resolves each of these to a
//! `MethodDescription` and then drops it, because nothing was ever bound to
//! them; the client-patch DLL decodes them with these types instead.

use super::{int32_args, AuctionItem, ClientMethod, MAX_AUCTION_ITEMS, MAX_WATCHED_ITEMS};
use crate::codec::{decode_whole, read_count, read_i32, write_count, write_i32};
use crate::{ByteSource, Decode, DecodeError, Encode, EncodeError};

int32_args! {
    /// `onBMOpen(INT32 entityId)`: open the window.
    OnBMOpen {
        /// The auctioneer the player is talking to.
        entity_id = "entityId",
    }
}

int32_args! {
    /// `onBMError(INT32 errorId)`: a request failed. The client has no id
    /// to text table of its own; the UI overlay supplies one.
    OnBMError {
        /// `EBlackMarketError` for the two shipped values, then the
        /// server's own rejection ids.
        error_id = "errorId",
    }
}

int32_args! {
    /// `onBMAuctionRemove(INT32 sequenceId)`: drop a row the client may be
    /// showing.
    OnBMAuctionRemove {
        /// The auction to remove.
        sequence_id = "sequenceId",
    }
}

/// `onBMAuctions(ARRAY<AuctionItem> auctionItems, INT32 totalResults,
/// INT32 clientKey)`: one page of results for the list `client_key` names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnBMAuctions {
    /// `auctionItems`: at most [`MAX_AUCTION_ITEMS`].
    pub auction_items: Vec<AuctionItem>,
    /// `totalResults`: every match, not just this page.
    pub total_results: i32,
    /// `clientKey`: the `BMSearchOptions.clientKey` of the request, a
    /// [`UIAuctionView`](super::UIAuctionView) value.
    pub client_key: i32,
}

impl Encode for OnBMAuctions {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        write_count(
            out,
            self.auction_items.len(),
            "auctionItems",
            MAX_AUCTION_ITEMS,
        )?;
        for item in &self.auction_items {
            item.write_to(out)?;
        }
        write_i32(out, self.total_results);
        write_i32(out, self.client_key);
        Ok(())
    }
}

impl Decode for OnBMAuctions {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        let count = read_count(
            src,
            "auctionItems",
            MAX_AUCTION_ITEMS,
            AuctionItem::MIN_WIRE_LEN,
        )?;
        let mut auction_items = Vec::with_capacity(count);
        for _ in 0..count {
            auction_items.push(AuctionItem::decode_from(src)?);
        }
        Ok(Self {
            auction_items,
            total_results: read_i32(src, "totalResults")?,
            client_key: read_i32(src, "clientKey")?,
        })
    }
}

/// `onBMAuctionUpdate(AuctionItem auctionItem)`: a row changed, for
/// example after a bid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnBMAuctionUpdate {
    /// `auctionItem`.
    pub auction_item: AuctionItem,
}

impl Encode for OnBMAuctionUpdate {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        self.auction_item.write_to(out)
    }
}

impl Decode for OnBMAuctionUpdate {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        Ok(Self {
            auction_item: AuctionItem::decode_from(src)?,
        })
    }
}

/// `onBMWatchedItemsUpdate(ARRAY<INT32> itemList)`: the item definitions
/// the player is watching.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnBMWatchedItemsUpdate {
    /// `itemList`: at most [`MAX_WATCHED_ITEMS`] item definition ids.
    pub item_list: Vec<i32>,
}

impl Encode for OnBMWatchedItemsUpdate {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        write_count(out, self.item_list.len(), "itemList", MAX_WATCHED_ITEMS)?;
        for id in &self.item_list {
            write_i32(out, *id);
        }
        Ok(())
    }
}

impl Decode for OnBMWatchedItemsUpdate {
    fn decode_from<S: ByteSource + ?Sized>(src: &mut S) -> Result<Self, DecodeError> {
        let count = read_count(src, "itemList", MAX_WATCHED_ITEMS, 4)?;
        let mut item_list = Vec::with_capacity(count);
        for _ in 0..count {
            item_list.push(read_i32(src, "itemList")?);
        }
        Ok(Self { item_list })
    }
}

/// Any one Black Market client method call, tagged by method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientCall {
    /// `onBMOpen`.
    Open(OnBMOpen),
    /// `onBMError`.
    Error(OnBMError),
    /// `onBMAuctions`.
    Auctions(OnBMAuctions),
    /// `onBMAuctionRemove`.
    AuctionRemove(OnBMAuctionRemove),
    /// `onBMAuctionUpdate`.
    AuctionUpdate(OnBMAuctionUpdate),
    /// `onBMWatchedItemsUpdate`.
    WatchedItemsUpdate(OnBMWatchedItemsUpdate),
}

impl ClientCall {
    /// The method this call is.
    pub fn method(&self) -> ClientMethod {
        match self {
            Self::Open(_) => ClientMethod::OnBMOpen,
            Self::Error(_) => ClientMethod::OnBMError,
            Self::Auctions(_) => ClientMethod::OnBMAuctions,
            Self::AuctionRemove(_) => ClientMethod::OnBMAuctionRemove,
            Self::AuctionUpdate(_) => ClientMethod::OnBMAuctionUpdate,
            Self::WatchedItemsUpdate(_) => ClientMethod::OnBMWatchedItemsUpdate,
        }
    }

    /// Decode the arguments of `method` from `src`. Consumes exactly the
    /// arguments; see [`Decode::decode_from`].
    pub fn decode_from<S: ByteSource + ?Sized>(
        method: ClientMethod,
        src: &mut S,
    ) -> Result<Self, DecodeError> {
        Ok(match method {
            ClientMethod::OnBMOpen => Self::Open(OnBMOpen::decode_from(src)?),
            ClientMethod::OnBMError => Self::Error(OnBMError::decode_from(src)?),
            ClientMethod::OnBMAuctions => Self::Auctions(OnBMAuctions::decode_from(src)?),
            ClientMethod::OnBMAuctionRemove => {
                Self::AuctionRemove(OnBMAuctionRemove::decode_from(src)?)
            }
            ClientMethod::OnBMAuctionUpdate => {
                Self::AuctionUpdate(OnBMAuctionUpdate::decode_from(src)?)
            }
            ClientMethod::OnBMWatchedItemsUpdate => {
                Self::WatchedItemsUpdate(OnBMWatchedItemsUpdate::decode_from(src)?)
            }
        })
    }

    /// Decode a complete `method` payload, rejecting trailing bytes; see
    /// [`Decode::decode`].
    pub fn decode(method: ClientMethod, bytes: &[u8]) -> Result<Self, DecodeError> {
        decode_whole(bytes, |src| Self::decode_from(method, src))
    }
}

impl Encode for ClientCall {
    fn write_to(&self, out: &mut Vec<u8>) -> Result<(), EncodeError> {
        match self {
            Self::Open(args) => args.write_to(out),
            Self::Error(args) => args.write_to(out),
            Self::Auctions(args) => args.write_to(out),
            Self::AuctionRemove(args) => args.write_to(out),
            Self::AuctionUpdate(args) => args.write_to(out),
            Self::WatchedItemsUpdate(args) => args.write_to(out),
        }
    }
}
