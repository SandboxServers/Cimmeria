//! Method indices, method names and the Black Market UI enums.
//!
//! Indices are the flattened `SGWPlayer` indices from
//! `docs/protocol/cell-method-dispatch-table.md` (cell, 61–66) and
//! `docs/protocol/client-method-dispatch-table.md` (client, 90–95). All
//! twelve are at or above `SGWPlayer`'s extended threshold of 61, so on the
//! wire each one is the message id [`EXTENDED_MESSAGE_ID`] followed by a
//! one-byte sub-index, `index - 61`.

use crate::UnknownEnumValue;

/// `SGWPlayer`'s first extended method index (`0x3D`), in both directions.
/// Methods at or above it are sent as [`EXTENDED_MESSAGE_ID`] plus a
/// sub-index byte of `index - EXTENDED_METHOD_BASE`.
pub const EXTENDED_METHOD_BASE: u16 = 0x3D;

/// The message id byte of an extended entity method: `0x3D | 0x80`.
pub const EXTENDED_MESSAGE_ID: u8 = 0xBD;

/// The exposed Black Market cell methods: client to server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum CellMethod {
    /// `BMSearch(BMSearchOptions searchOptions)`.
    BMSearch = 61,
    /// `BMCreateAuction(INT32 itemInstanceId, INT32 buyoutPrice, UINT8 auctionLength, INT32 startingPrice)`.
    BMCreateAuction = 62,
    /// `BMPlaceBid(INT32 sequenceId, INT32 bidAmount)`.
    BMPlaceBid = 63,
    /// `BMCancelAuction(INT32 sequenceId)`.
    BMCancelAuction = 64,
    /// `BMStartWatchingItem(INT32 itemDefId)`.
    BMStartWatchingItem = 65,
    /// `BMStopWatchingItem(INT32 itemDefId)`.
    BMStopWatchingItem = 66,
}

impl CellMethod {
    /// All six, in index order.
    pub const ALL: [Self; 6] = [
        Self::BMSearch,
        Self::BMCreateAuction,
        Self::BMPlaceBid,
        Self::BMCancelAuction,
        Self::BMStartWatchingItem,
        Self::BMStopWatchingItem,
    ];

    /// The flattened `SGWPlayer` cell method index.
    pub const fn index(self) -> u16 {
        self as u16
    }

    /// The sub-index byte that follows [`EXTENDED_MESSAGE_ID`]: 0–5.
    pub const fn sub_index(self) -> u8 {
        (self as u16 - EXTENDED_METHOD_BASE) as u8
    }

    /// The method name in the `.def`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::BMSearch => "BMSearch",
            Self::BMCreateAuction => "BMCreateAuction",
            Self::BMPlaceBid => "BMPlaceBid",
            Self::BMCancelAuction => "BMCancelAuction",
            Self::BMStartWatchingItem => "BMStartWatchingItem",
            Self::BMStopWatchingItem => "BMStopWatchingItem",
        }
    }

    /// The method for a sub-index byte, if it is one of these six.
    pub fn from_sub_index(sub_index: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.sub_index() == sub_index)
    }
}

impl TryFrom<u16> for CellMethod {
    type Error = UnknownEnumValue;

    fn try_from(index: u16) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|m| m.index() == index)
            .ok_or(UnknownEnumValue {
                enum_name: "CellMethod",
                value: i64::from(index),
            })
    }
}

/// The Black Market client methods: server to client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum ClientMethod {
    /// `onBMOpen(INT32 entityId)`.
    OnBMOpen = 90,
    /// `onBMError(INT32 errorId)`.
    OnBMError = 91,
    /// `onBMAuctions(ARRAY<AuctionItem> auctionItems, INT32 totalResults, INT32 clientKey)`.
    OnBMAuctions = 92,
    /// `onBMAuctionRemove(INT32 sequenceId)`.
    OnBMAuctionRemove = 93,
    /// `onBMAuctionUpdate(AuctionItem auctionItem)`.
    OnBMAuctionUpdate = 94,
    /// `onBMWatchedItemsUpdate(ARRAY<INT32> itemList)`.
    OnBMWatchedItemsUpdate = 95,
}

impl ClientMethod {
    /// All six, in index order.
    pub const ALL: [Self; 6] = [
        Self::OnBMOpen,
        Self::OnBMError,
        Self::OnBMAuctions,
        Self::OnBMAuctionRemove,
        Self::OnBMAuctionUpdate,
        Self::OnBMWatchedItemsUpdate,
    ];

    /// Length of the longest name, `onBMWatchedItemsUpdate`. A name longer
    /// than this cannot be one of these methods, so a caller reading names
    /// out of client memory need not read further.
    pub const MAX_NAME_LEN: usize = 22;

    /// The flattened `SGWPlayer` client method index.
    pub const fn index(self) -> u16 {
        self as u16
    }

    /// The sub-index byte that follows [`EXTENDED_MESSAGE_ID`]: 29–34.
    pub const fn sub_index(self) -> u8 {
        (self as u16 - EXTENDED_METHOD_BASE) as u8
    }

    /// The method name in the `.def`, which is also the name the client
    /// keeps in its `MethodDescription`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::OnBMOpen => "onBMOpen",
            Self::OnBMError => "onBMError",
            Self::OnBMAuctions => "onBMAuctions",
            Self::OnBMAuctionRemove => "onBMAuctionRemove",
            Self::OnBMAuctionUpdate => "onBMAuctionUpdate",
            Self::OnBMWatchedItemsUpdate => "onBMWatchedItemsUpdate",
        }
    }

    /// The method with exactly this name. Case-sensitive, and no prefix or
    /// suffix matching: `onBMOpenX` and `onbmopen` are not `onBMOpen`.
    pub fn from_name(name: &[u8]) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.name().as_bytes() == name)
    }
}

impl TryFrom<u16> for ClientMethod {
    type Error = UnknownEnumValue;

    fn try_from(index: u16) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|m| m.index() == index)
            .ok_or(UnknownEnumValue {
                enum_name: "ClientMethod",
                value: i64::from(index),
            })
    }
}

/// Which auction list a search fills. The client sends it as
/// `BMSearchOptions.clientKey` and the server echoes it as
/// `onBMAuctions.clientKey`. Values are the client's tolua constants, which
/// agree with the seeded `EBlackMarketSearchType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum UIAuctionView {
    /// A plain search.
    SearchResults = 0,
    /// The player's own listings (`refreshMyAuctions`).
    MyAuctions = 1,
    /// Listings the player has bid on (`refreshMyBids`).
    MyBids = 2,
}

impl TryFrom<i32> for UIAuctionView {
    type Error = UnknownEnumValue;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::SearchResults),
            1 => Ok(Self::MyAuctions),
            2 => Ok(Self::MyBids),
            _ => Err(UnknownEnumValue {
                enum_name: "UIAuctionView",
                value: i64::from(value),
            }),
        }
    }
}

/// Listing duration buckets: `BMCreateAuction.auctionLength` and the
/// time-left bucket in `AuctionItem.endTimeValue`. 1-based, from the
/// client's tolua constants; the create form offers `Medium`, `Long` and
/// `VeryLong`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum UIAuctionTime {
    /// 1.
    VeryShort = 1,
    /// 2.
    Short = 2,
    /// 3.
    Medium = 3,
    /// 4.
    Long = 4,
    /// 5, the create form's default.
    VeryLong = 5,
}

impl TryFrom<u8> for UIAuctionTime {
    type Error = UnknownEnumValue;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::VeryShort),
            2 => Ok(Self::Short),
            3 => Ok(Self::Medium),
            4 => Ok(Self::Long),
            5 => Ok(Self::VeryLong),
            _ => Err(UnknownEnumValue {
                enum_name: "UIAuctionTime",
                value: i64::from(value),
            }),
        }
    }
}
