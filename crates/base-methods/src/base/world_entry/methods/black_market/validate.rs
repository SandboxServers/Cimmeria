//! Pure validation predicates for the Black Market state machine.
//!
//! These hold no DB or transport state: the create/bid/cancel handlers read
//! rows, then call these to decide. Each refusal is a [`BMError`], the id the
//! client gets in `onBMError` and whose [`BMError::reason`] is logged.

use super::types::{auction_status, AuctionRow, LISTABLE_BAGS, MAX_ACTIVE_LISTINGS};
use super::wire::{required_min_bid, BMError};

/// `createAuction` prices: a starting price of at least 1, and a buyout
/// that is either 0 (none) or at least the starting price. A starting price
/// of 0 would let a 0 bid "win", and the sweep settles only a positive bid.
pub fn validate_prices(starting_price: i32, buyout_price: i32) -> Result<(), BMError> {
    if starting_price < 1 {
        return Err(BMError::InvalidPrice);
    }
    if buyout_price != 0 && buyout_price < starting_price {
        return Err(BMError::InvalidPrice);
    }
    Ok(())
}

/// The listed row: it must be in a carried bag (`LISTABLE_BAGS`) and not
/// bound (a bound row may only ever go back to its owner, so it could not
/// be delivered to a buyer).
pub fn validate_listed_item(container_id: i32, bound: bool) -> Result<(), BMError> {
    if !LISTABLE_BAGS.contains(&container_id) {
        return Err(BMError::InvalidItem);
    }
    if bound {
        return Err(BMError::ItemBound);
    }
    Ok(())
}

/// The per-seller cap (D5): at most [`MAX_ACTIVE_LISTINGS`] active.
pub fn validate_listing_cap(active_listings: i64) -> Result<(), BMError> {
    if active_listings >= MAX_ACTIVE_LISTINGS {
        return Err(BMError::TooManyListings);
    }
    Ok(())
}

/// An auction is open to bids and cancellation only while it is `ACTIVE`
/// and its time is not up. Between `expires_at` and the next sweep pass the
/// row is still `ACTIVE`, but it is closed: a bid or cancel there would
/// change or void an auction the sweep is about to settle.
pub fn is_open(auction: &AuctionRow, now: i32) -> bool {
    auction.status == auction_status::ACTIVE && auction.expires_at > now
}

/// Is `bid_amount` a buyout? Any bid at or above a non-zero buyout price.
pub fn is_buyout(auction: &AuctionRow, bid_amount: i32) -> bool {
    auction.buyout_price > 0 && bid_amount >= auction.buyout_price
}

/// Validate a `placeBid` against the locked auction row and the bidder's
/// balance, `effective_balance` already counting a self-raise's refund.
///
/// Order: closed → is-seller → bid-too-low → funds. A buyout is charged the
/// buyout price, not the bid, and is exempt from the increment rule (the
/// next minimum can pass the buyout when the standing bid is close to it).
pub fn validate_bid(
    auction: &AuctionRow,
    bidder_id: i32,
    bid_amount: i32,
    effective_balance: i64,
    now: i32,
) -> Result<(), BMError> {
    if !is_open(auction, now) {
        return Err(BMError::AuctionGone);
    }
    if auction.seller_id == bidder_id {
        return Err(BMError::IsSeller);
    }
    let charge = if is_buyout(auction, bid_amount) {
        i64::from(auction.buyout_price)
    } else {
        if i64::from(bid_amount) < required_min_bid(auction) {
            return Err(BMError::BidTooLow);
        }
        i64::from(bid_amount)
    };
    if effective_balance < charge {
        return Err(BMError::NotEnoughFunds);
    }
    Ok(())
}

/// Validate a `cancelAuction`: the auction must be open and the caller its
/// seller.
pub fn validate_cancel(auction: &AuctionRow, caller_id: i32, now: i32) -> Result<(), BMError> {
    if !is_open(auction, now) {
        return Err(BMError::AuctionGone);
    }
    if auction.seller_id != caller_id {
        return Err(BMError::NotSeller);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i32 = 1_000;

    fn active_auction() -> AuctionRow {
        AuctionRow {
            sequence_id: 1,
            seller_id: 100,
            item_id: 10001,
            item_def_id: 5,
            stack_size: 1,
            durability: 100,
            charges: 0,
            starting_price: 50,
            buyout_price: 0,
            current_bid: 0,
            current_bidder: None,
            auction_length: 5,
            created_at: 0,
            expires_at: NOW + 60,
            status: auction_status::ACTIVE,
        }
    }

    #[test]
    fn prices_need_a_start_of_one_and_a_buyout_above_it() {
        assert_eq!(validate_prices(0, 0), Err(BMError::InvalidPrice));
        assert_eq!(validate_prices(-5, 0), Err(BMError::InvalidPrice));
        assert_eq!(validate_prices(100, 99), Err(BMError::InvalidPrice));
        assert_eq!(validate_prices(100, 0), Ok(()));
        assert_eq!(validate_prices(100, 100), Ok(()));
    }

    #[test]
    fn listed_item_must_be_in_a_carried_bag_and_unbound() {
        assert_eq!(validate_listed_item(1, false), Ok(()));
        assert_eq!(validate_listed_item(15, false), Ok(()));
        for bag in [2, 3, 4, 16, 17, 18, 19, 20] {
            assert_eq!(
                validate_listed_item(bag, false),
                Err(BMError::InvalidItem),
                "{bag}"
            );
        }
        assert_eq!(validate_listed_item(1, true), Err(BMError::ItemBound));
    }

    /// D5: the 21st active listing is refused, the 20th is not.
    #[test]
    fn listing_cap_is_twenty() {
        assert_eq!(validate_listing_cap(19), Ok(()));
        assert_eq!(validate_listing_cap(20), Err(BMError::TooManyListings));
    }

    #[test]
    fn bid_below_start_or_increment_is_too_low() {
        let mut a = active_auction();
        assert_eq!(
            validate_bid(&a, 200, 49, 10_000, NOW),
            Err(BMError::BidTooLow)
        );
        assert_eq!(validate_bid(&a, 200, 50, 10_000, NOW), Ok(()));
        a.current_bid = 100;
        a.current_bidder = Some(201);
        // D6: 5% of 100 is 5, so 105 is the floor.
        assert_eq!(
            validate_bid(&a, 202, 104, 10_000, NOW),
            Err(BMError::BidTooLow)
        );
        assert_eq!(validate_bid(&a, 202, 105, 10_000, NOW), Ok(()));
    }

    #[test]
    fn bid_refusals_in_order() {
        let a = active_auction();
        assert_eq!(
            validate_bid(&a, 100, 1_000, 10_000, NOW),
            Err(BMError::IsSeller)
        );
        assert_eq!(
            validate_bid(&a, 200, 500, 100, NOW),
            Err(BMError::NotEnoughFunds)
        );
        let mut sold = active_auction();
        sold.status = auction_status::SOLD;
        assert_eq!(
            validate_bid(&sold, 200, 1_000, 10_000, NOW),
            Err(BMError::AuctionGone)
        );
    }

    /// The expired-window guard: once `expires_at` has passed, the row is
    /// still ACTIVE until the sweep runs, but bids and cancels are refused.
    #[test]
    fn bid_and_cancel_after_expiry_are_auction_gone() {
        let mut a = active_auction();
        a.expires_at = NOW;
        assert_eq!(
            validate_bid(&a, 200, 1_000, 10_000, NOW),
            Err(BMError::AuctionGone)
        );
        assert_eq!(validate_cancel(&a, 100, NOW), Err(BMError::AuctionGone));
        a.expires_at = NOW + 1;
        assert_eq!(validate_cancel(&a, 100, NOW), Ok(()));
    }

    /// A buyout is charged the buyout price and skips the increment rule.
    #[test]
    fn buyout_is_charged_the_buyout_price() {
        let mut a = active_auction();
        a.buyout_price = 1_000;
        a.current_bid = 990;
        a.current_bidder = Some(201);
        assert!(is_buyout(&a, 1_000));
        assert!(!is_buyout(&a, 999));
        // next_min_bid(990) = 1039 > 1000, but the buyout still goes through.
        assert_eq!(validate_bid(&a, 202, 1_000, 1_000, NOW), Ok(()));
        // An over-bid is charged only the buyout price.
        assert_eq!(validate_bid(&a, 202, 5_000, 1_000, NOW), Ok(()));
        assert_eq!(
            validate_bid(&a, 202, 5_000, 999, NOW),
            Err(BMError::NotEnoughFunds)
        );
        a.buyout_price = 0;
        assert!(!is_buyout(&a, 5_000), "no buyout price, no buyout");
    }

    #[test]
    fn cancel_needs_the_seller() {
        let a = active_auction();
        assert_eq!(validate_cancel(&a, 999, NOW), Err(BMError::NotSeller));
        assert_eq!(validate_cancel(&a, 100, NOW), Ok(()));
        let mut gone = active_auction();
        gone.status = auction_status::CANCELLED;
        assert_eq!(validate_cancel(&gone, 100, NOW), Err(BMError::AuctionGone));
    }
}
