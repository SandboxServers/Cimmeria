//! The `onBMError.errorId` vocabulary.
//!
//! The shipped client enum `EBlackMarketError` has two values,
//! `InvalidSortType = 0` and `BMUnavailable = 1`, and no client code maps
//! an id to text (the C++ subscriber was never written). Decision D3 of
//! `docs/analysis/black-market/README.md`: keep those two ids and number
//! the server's rejections after them. The UI overlay maps each id to its
//! English line, and the server logs each refusal with [`BMError::reason`],
//! so a SigNoz `reason` and the id the client got always agree.

use crate::UnknownEnumValue;

/// Why a Black Market request was refused: the `errorId` of `onBMError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BMError {
    /// 0, shipped. The server sends it for a `clientKey` that names no
    /// `UIAuctionView`.
    InvalidSortType = 0,
    /// 1, shipped. The Black Market cannot serve requests (no database).
    BMUnavailable = 1,
    /// The bidder cannot cover the bid.
    NotEnoughFunds = 2,
    /// The auction does not exist, is settled, or its time is up.
    AuctionGone = 3,
    /// A seller bid on their own auction.
    IsSeller = 4,
    /// The bid is below the auction's next minimum bid.
    BidTooLow = 5,
    /// The item is not the seller's, or not in a bag it can be listed from.
    InvalidItem = 6,
    /// Only the seller may cancel an auction.
    NotSeller = 7,
    /// A server-side failure; nothing changed.
    Internal = 8,
    /// The request did not come from a player at an open auctioneer.
    NotAtAuctioneer = 9,
    /// The seller already has the most active listings allowed (D5).
    TooManyListings = 10,
    /// The starting price is below 1, or a buyout is below the start.
    InvalidPrice = 11,
    /// Bound items cannot be listed.
    ItemBound = 12,
    /// There is no free bag slot to return the item to.
    BagFull = 13,
    /// The watch list is not available yet (D4).
    WatchUnavailable = 14,
}

impl BMError {
    /// Every id, in value order.
    pub const ALL: [Self; 15] = [
        Self::InvalidSortType,
        Self::BMUnavailable,
        Self::NotEnoughFunds,
        Self::AuctionGone,
        Self::IsSeller,
        Self::BidTooLow,
        Self::InvalidItem,
        Self::NotSeller,
        Self::Internal,
        Self::NotAtAuctioneer,
        Self::TooManyListings,
        Self::InvalidPrice,
        Self::ItemBound,
        Self::BagFull,
        Self::WatchUnavailable,
    ];

    /// The wire value.
    pub const fn id(self) -> i32 {
        self as i32
    }

    /// The stable `reason` label the server logs with this refusal.
    pub const fn reason(self) -> &'static str {
        match self {
            Self::InvalidSortType => "invalid_client_key",
            Self::BMUnavailable => "bm_unavailable",
            Self::NotEnoughFunds => "not_enough_funds",
            Self::AuctionGone => "auction_gone",
            Self::IsSeller => "is_seller",
            Self::BidTooLow => "bid_too_low",
            Self::InvalidItem => "invalid_item",
            Self::NotSeller => "not_seller",
            Self::Internal => "internal",
            Self::NotAtAuctioneer => "not_at_auctioneer",
            Self::TooManyListings => "too_many_listings",
            Self::InvalidPrice => "invalid_price",
            Self::ItemBound => "item_bound",
            Self::BagFull => "bag_full",
            Self::WatchUnavailable => "watch_unavailable",
        }
    }
}

impl TryFrom<i32> for BMError {
    type Error = UnknownEnumValue;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::ALL
            .into_iter()
            .find(|e| e.id() == value)
            .ok_or(UnknownEnumValue {
                enum_name: "BMError",
                value: i64::from(value),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D3: the two shipped ids keep their values, and every id is its
    /// position in `ALL`, so the overlay's table has no holes.
    #[test]
    fn shipped_ids_are_kept_and_ids_are_dense() {
        assert_eq!(BMError::InvalidSortType.id(), 0);
        assert_eq!(BMError::BMUnavailable.id(), 1);
        for (i, e) in BMError::ALL.into_iter().enumerate() {
            assert_eq!(e.id(), i as i32, "{e:?}");
            assert_eq!(BMError::try_from(e.id()), Ok(e));
        }
        assert!(BMError::try_from(BMError::ALL.len() as i32).is_err());
        assert!(BMError::try_from(-1).is_err());
    }

    /// Reason labels are unique, so a log `reason` names one id.
    #[test]
    fn reasons_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for e in BMError::ALL {
            assert!(seen.insert(e.reason()), "duplicate reason {}", e.reason());
        }
    }
}
