//! Byte-exact wire tests for the `onBM*` arguments, and the listing-time
//! rules (durations, time-left bucket, next minimum bid, page budget).

use cimmeria_wire::black_market::{Decode, OnBMAuctionUpdate, OnBMAuctions};

use super::*;

const NOW: i32 = 1_000_000;

fn row(seq: i32) -> AuctionRow {
    AuctionRow {
        sequence_id: seq,
        seller_id: 2,
        item_id: 3,
        item_def_id: 0x22,
        stack_size: 5,
        durability: 100,
        charges: -1,
        starting_price: 50,
        buyout_price: 1000,
        current_bid: 400,
        current_bidder: Some(9),
        auction_length: 5,
        created_at: 0,
        // 60 h left: the Long bucket (48 h < 60 h <= 72 h).
        expires_at: NOW + 60 * 3_600,
        status: 0,
    }
}

/// `row(0x11)` with seller "Bo" as an `AuctionItem`, written out by hand:
/// 39 bytes.
const ROW_BYTES: &[u8] = &[
    0x11, 0x00, 0x00, 0x00, // sequenceId
    0x22, 0x00, 0x00, 0x00, // itemDefId
    0x05, 0x00, 0x00, 0x00, // stackSize
    0x64, 0x00, 0x00, 0x00, // durability = 100
    0xFF, 0xFF, 0xFF, 0xFF, // charges = -1
    0x90, 0x01, 0x00, 0x00, // currentBid = 400
    0xE8, 0x03, 0x00, 0x00, // buyoutPrice = 1000
    0x04, // endTimeValue = Long (time left, S6), not the listed tier 5
    0xA4, 0x01, 0x00, 0x00, // nextMinBidPrice = 400 + 5% = 420 (D6)
    0x02, 0x00, 0x00, 0x00, b'B', b'o', // sellerName
];

/// S1 regression guard: `onBMAuctions` is `(ARRAY<AuctionItem>,
/// totalResults, clientKey)`, the `.def` order. The branch sent `(items,
/// view, total)`, which puts 7 where the client reads the total.
#[test]
fn on_bm_auctions_is_items_then_total_then_client_key() {
    let (args, sent) = serialize_on_bm_auctions(&[(row(0x11), "Bo".into())], 99, 2, NOW);
    assert_eq!(sent, 1);
    let mut expected = vec![0x01, 0x00, 0x00, 0x00]; // count
    expected.extend_from_slice(ROW_BYTES);
    expected.extend_from_slice(&[0x63, 0x00, 0x00, 0x00]); // totalResults = 99
    expected.extend_from_slice(&[0x02, 0x00, 0x00, 0x00]); // clientKey = MyBids
    assert_eq!(args, expected);

    // The client patch decodes it with the same codec.
    let decoded = OnBMAuctions::decode(&args).expect("patch decodes it");
    assert_eq!(decoded.total_results, 99);
    assert_eq!(decoded.client_key, 2);
}

#[test]
fn on_bm_auctions_empty_is_twelve_bytes() {
    let (args, sent) = serialize_on_bm_auctions(&[], 0, 1, NOW);
    assert_eq!(sent, 0);
    assert_eq!(args, vec![0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);
}

#[test]
fn on_bm_auction_update_is_one_auction_item() {
    let args = serialize_on_bm_auction_update(&row(0x11), "Bo", NOW);
    assert_eq!(args, ROW_BYTES);
    assert!(OnBMAuctionUpdate::decode(&args).is_ok());
}

#[test]
fn on_bm_auction_remove_and_error_are_one_int32() {
    assert_eq!(serialize_on_bm_auction_remove(777), vec![0x09, 0x03, 0, 0]);
    assert_eq!(serialize_on_bm_error(BMError::BidTooLow), vec![5, 0, 0, 0]);
}

/// S7 regression guard: the page is cut by serialized size, never past the
/// one-message budget, and `totalResults` stays the full match count.
#[test]
fn on_bm_auctions_pages_by_serialized_size() {
    let long_name = "N".repeat(200);
    let rows: Vec<_> = (0..40).map(|i| (row(i), long_name.clone())).collect();
    let (args, sent) = serialize_on_bm_auctions(&rows, 40, 0, NOW);
    assert!(args.len() <= AUCTIONS_ARG_BUDGET, "{} bytes", args.len());
    // 12 + n * (37 + 200) <= 1200 -> n = 5.
    assert_eq!(sent, 5);
    let decoded = OnBMAuctions::decode(&args).unwrap();
    assert_eq!(decoded.auction_items.len(), 5);
    assert_eq!(decoded.total_results, 40, "total is every match");

    // Short names fit more rows, still within budget.
    let rows: Vec<_> = (0..40).map(|i| (row(i), "Bo".to_string())).collect();
    let (args, sent) = serialize_on_bm_auctions(&rows, 40, 0, NOW);
    assert!(args.len() <= AUCTIONS_ARG_BUDGET);
    assert_eq!(sent, (AUCTIONS_ARG_BUDGET - 12) / 39);
}

/// A seller name longer than the codec cap is cut, not a failed reply.
#[test]
fn over_long_seller_name_is_capped() {
    let name = "é".repeat(200); // 400 bytes
    let args = serialize_on_bm_auction_update(&row(1), &name, NOW);
    let decoded = OnBMAuctionUpdate::decode(&args).unwrap();
    assert!(decoded.auction_item.seller_name.len() <= MAX_STRING_LEN as usize);
}

// ── durations and buckets ─────────────────────────────────────────────────

/// S5 regression guard: `auctionLength` is 1-based. The UI's three buttons
/// send 3, 4 and 5; the branch mapped 4 and 5 both to its longest tier.
#[test]
fn auction_length_is_one_based() {
    let hours = |raw| auction_length_seconds(clamp_auction_length(raw).0) / 3_600;
    assert_eq!(
        [1, 2, 3, 4, 5].map(hours),
        [12, 24, 48, 72, 96],
        "each tier its own duration"
    );
    assert_ne!(hours(4), hours(5));
}

/// Out-of-range lengths clamp to the nearest tier and say so.
#[test]
fn auction_length_clamps_to_valid_tiers() {
    assert_eq!(clamp_auction_length(0), (UIAuctionTime::VeryShort, true));
    assert_eq!(clamp_auction_length(6), (UIAuctionTime::VeryLong, true));
    assert_eq!(clamp_auction_length(255), (UIAuctionTime::VeryLong, true));
    assert_eq!(clamp_auction_length(3), (UIAuctionTime::Medium, false));
}

/// S6 regression guard: `endTimeValue` is the time left, which falls
/// through the tiers as the listing ages.
#[test]
fn end_time_value_is_the_time_left_bucket() {
    let h = 3_600;
    assert_eq!(time_left_bucket(NOW + 96 * h, NOW), UIAuctionTime::VeryLong);
    assert_eq!(time_left_bucket(NOW + 73 * h, NOW), UIAuctionTime::VeryLong);
    assert_eq!(time_left_bucket(NOW + 72 * h, NOW), UIAuctionTime::Long);
    assert_eq!(time_left_bucket(NOW + 30 * h, NOW), UIAuctionTime::Medium);
    assert_eq!(time_left_bucket(NOW + 13 * h, NOW), UIAuctionTime::Short);
    assert_eq!(time_left_bucket(NOW + 60, NOW), UIAuctionTime::VeryShort);
    assert_eq!(time_left_bucket(NOW - 60, NOW), UIAuctionTime::VeryShort);

    // A VeryLong listing, listed with tier 5, shows Medium a day and a half
    // before it ends: the stored tier is not echoed.
    let mut r = row(1);
    r.auction_length = 5;
    r.expires_at = NOW + 36 * h;
    assert_eq!(auction_item(&r, "", NOW).end_time_value, 3);
}

// ── next minimum bid (D6) ─────────────────────────────────────────────────

#[test]
fn next_min_bid_is_five_percent_at_least_one() {
    assert_eq!(next_min_bid(0), 1);
    assert_eq!(next_min_bid(10), 11);
    assert_eq!(next_min_bid(100), 105);
    assert_eq!(next_min_bid(1_000), 1_050);
}

#[test]
fn required_min_bid_is_start_then_increment() {
    let mut r = row(1);
    r.current_bid = 0;
    r.current_bidder = None;
    r.starting_price = 100;
    assert_eq!(required_min_bid(&r), 100);
    r.starting_price = 0;
    assert_eq!(required_min_bid(&r), 1, "never below 1");
    r.current_bid = 200;
    r.current_bidder = Some(5);
    assert_eq!(required_min_bid(&r), 210);
}

/// The next minimum clamps at `i32::MAX` on the wire, never wraps negative.
#[test]
fn next_min_bid_clamps_at_i32_max_on_the_wire() {
    let mut r = row(1);
    r.current_bid = i32::MAX;
    assert_eq!(auction_item(&r, "", NOW).next_min_bid_price, i32::MAX);
}
