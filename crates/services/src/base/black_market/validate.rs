//! Pure validation predicates for the Black Market state machine.
//!
//! These hold no DB or transport state — the create/bid/cancel handlers fetch
//! rows, then call these to decide accept/reject. Keeping the decisions pure
//! lets each rejection branch be unit-tested without a live database.
//!
//! Each predicate returns `Err(error_code)` carrying a [`wire::error_code`]
//! placeholder value to feed straight into `onBMError`.

use super::types::{auction_status, AuctionRow};
use super::wire::{error_code, next_min_bid};

/// Validate a `createAuction` request. `escrowed_ok` reports whether the
/// seller actually owned the item instance (the escrow DELETE matched a row);
/// the prices are sanity-bounded.
///
/// Ownership is the load-bearing check — the item must belong to the seller.
pub fn validate_create(
    escrowed_ok: bool,
    starting_price: i32,
    buyout_price: i32,
) -> Result<(), i32> {
    if !escrowed_ok {
        // Item missing / not owned / not auctionable.
        return Err(error_code::INVALID_ITEM);
    }
    if starting_price < 0 || buyout_price < 0 {
        return Err(error_code::INVALID_ITEM);
    }
    Ok(())
}

/// Validate a `placeBid` request against the fetched auction row and the
/// bidder's available balance.
///
/// Order of checks mirrors the failure precedence the client expects:
/// auction-gone → is-seller → bid-too-low → insufficient-funds.
pub fn validate_bid(
    auction: &AuctionRow,
    bidder_id: i32,
    bid_amount: i32,
    bidder_balance: i64,
) -> Result<(), i32> {
    if auction.status != auction_status::ACTIVE {
        return Err(error_code::AUCTION_GONE);
    }
    if auction.seller_id == bidder_id {
        return Err(error_code::IS_SELLER);
    }
    let floor = required_min_bid(auction);
    if (bid_amount as i64) < floor {
        return Err(error_code::BID_TOO_LOW);
    }
    if bidder_balance < bid_amount as i64 {
        return Err(error_code::NOT_ENOUGH_FUNDS);
    }
    Ok(())
}

/// The minimum acceptable bid for an auction: the next-min-bid above the
/// current bid once bidding has started, else the starting price (the first
/// bid must at least meet the starting price).
pub fn required_min_bid(auction: &AuctionRow) -> i64 {
    if auction.current_bid > 0 || auction.current_bidder.is_some() {
        next_min_bid(auction.current_bid as i64)
    } else {
        auction.starting_price.max(0) as i64
    }
}

/// Validate a `cancelAuction` request: caller must be the seller and the
/// auction must still be active.
pub fn validate_cancel(auction: &AuctionRow, caller_id: i32) -> Result<(), i32> {
    if auction.status != auction_status::ACTIVE {
        return Err(error_code::AUCTION_GONE);
    }
    if auction.seller_id != caller_id {
        return Err(error_code::NOT_SELLER);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            auction_length: 1,
            created_at: 0,
            expires_at: i32::MAX,
            status: auction_status::ACTIVE,
        }
    }

    // ── create ──────────────────────────────────────────────────────────

    #[test]
    fn create_rejects_when_not_owner() {
        // escrow DELETE matched no row => seller didn't own the instance.
        let err = validate_create(false, 100, 0).unwrap_err();
        assert_eq!(err, error_code::INVALID_ITEM);
    }

    #[test]
    fn create_accepts_owned_item() {
        assert!(validate_create(true, 100, 500).is_ok());
    }

    // ── bid ─────────────────────────────────────────────────────────────

    #[test]
    fn bid_rejected_too_low_below_starting_price() {
        let a = active_auction(); // starting_price 50, no bids yet
        let err = validate_bid(&a, 200, 49, 10_000).unwrap_err();
        assert_eq!(err, error_code::BID_TOO_LOW);
        // exactly meeting the starting price is accepted
        assert!(validate_bid(&a, 200, 50, 10_000).is_ok());
    }

    #[test]
    fn bid_rejected_below_next_min_once_bidding_started() {
        let mut a = active_auction();
        a.current_bid = 100;
        a.current_bidder = Some(201);
        // next_min_bid(100) = 105; 104 must fail, 105 must pass
        assert_eq!(
            validate_bid(&a, 202, 104, 10_000).unwrap_err(),
            error_code::BID_TOO_LOW
        );
        assert!(validate_bid(&a, 202, 105, 10_000).is_ok());
    }

    #[test]
    fn bid_rejected_on_own_auction() {
        let a = active_auction(); // seller 100
        let err = validate_bid(&a, 100, 1_000, 10_000).unwrap_err();
        assert_eq!(err, error_code::IS_SELLER);
    }

    #[test]
    fn bid_rejected_insufficient_funds() {
        let a = active_auction();
        // amount clears the floor (50) but balance can't cover it
        let err = validate_bid(&a, 200, 500, 100).unwrap_err();
        assert_eq!(err, error_code::NOT_ENOUGH_FUNDS);
    }

    #[test]
    fn bid_rejected_when_auction_not_active() {
        let mut a = active_auction();
        a.status = auction_status::SOLD;
        let err = validate_bid(&a, 200, 1_000, 10_000).unwrap_err();
        assert_eq!(err, error_code::AUCTION_GONE);
    }

    // ── cancel ──────────────────────────────────────────────────────────

    #[test]
    fn cancel_rejected_when_not_seller() {
        let a = active_auction(); // seller 100
        let err = validate_cancel(&a, 999).unwrap_err();
        assert_eq!(err, error_code::NOT_SELLER);
    }

    #[test]
    fn cancel_accepts_seller() {
        let a = active_auction();
        assert!(validate_cancel(&a, 100).is_ok());
    }

    #[test]
    fn cancel_rejected_when_already_gone() {
        let mut a = active_auction();
        a.status = auction_status::CANCELLED;
        assert_eq!(
            validate_cancel(&a, 100).unwrap_err(),
            error_code::AUCTION_GONE
        );
    }
}
