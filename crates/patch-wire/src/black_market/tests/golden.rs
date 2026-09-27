//! Byte-exact encodings for every Black Market method, both directions.
//! Each expected byte string is written out by hand from the `.def`; each
//! test encodes, compares, and decodes the bytes back.

use super::{sample_item, SAMPLE_ITEM_BYTES};
use crate::black_market::*;
use crate::{Decode, Encode};

/// Encode `value`, compare with `expected`, and decode `expected` back.
fn assert_golden<T: Encode + Decode + PartialEq + std::fmt::Debug>(value: &T, expected: &[u8]) {
    let bytes = value.to_bytes().expect("encodes");
    assert_eq!(bytes, expected, "encoding of {value:?}");
    assert_eq!(&T::decode(expected).expect("decodes"), value);
}

// ── server to client (90-95) ─────────────────────────────────────────────

#[test]
fn on_bm_open_is_one_int32() {
    assert_golden(
        &OnBMOpen {
            entity_id: 0x0102_0304,
        },
        &[0x04, 0x03, 0x02, 0x01],
    );
}

#[test]
fn on_bm_error_is_one_int32() {
    assert_golden(&OnBMError { error_id: 1 }, &[0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn auction_item_layout() {
    assert_golden(&sample_item(), SAMPLE_ITEM_BYTES);
    assert_eq!(SAMPLE_ITEM_BYTES.len(), AuctionItem::MIN_WIRE_LEN + 2);
}

/// `.def` order is `(auctionItems, totalResults, clientKey)`. The unmerged
/// server branch sent `(items, view, total)`; with these values that order
/// would put 1 where `totalResults` is expected.
#[test]
fn on_bm_auctions_is_items_then_total_then_client_key() {
    let mut expected = vec![0x01, 0x00, 0x00, 0x00]; // one item
    expected.extend_from_slice(SAMPLE_ITEM_BYTES);
    expected.extend_from_slice(&[0x09, 0x00, 0x00, 0x00]); // totalResults = 9
    expected.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // clientKey = MyAuctions
    assert_golden(
        &OnBMAuctions {
            auction_items: vec![sample_item()],
            total_results: 9,
            client_key: UIAuctionView::MyAuctions as i32,
        },
        &expected,
    );
}

#[test]
fn on_bm_auctions_empty_page() {
    assert_golden(
        &OnBMAuctions {
            auction_items: Vec::new(),
            total_results: 0,
            client_key: UIAuctionView::MyBids as i32,
        },
        &[
            0x00, 0x00, 0x00, 0x00, // no items
            0x00, 0x00, 0x00, 0x00, // totalResults = 0
            0x02, 0x00, 0x00, 0x00, // clientKey = MyBids
        ],
    );
}

#[test]
fn on_bm_auction_remove_is_one_int32() {
    assert_golden(
        &OnBMAuctionRemove { sequence_id: 7 },
        &[0x07, 0x00, 0x00, 0x00],
    );
}

/// A bare `AuctionItem`, with no count in front of it.
#[test]
fn on_bm_auction_update_is_a_bare_auction_item() {
    assert_golden(
        &OnBMAuctionUpdate {
            auction_item: sample_item(),
        },
        SAMPLE_ITEM_BYTES,
    );
}

#[test]
fn on_bm_watched_items_update_is_count_then_int32s() {
    assert_golden(
        &OnBMWatchedItemsUpdate {
            item_list: vec![5, 0x0100_0000],
        },
        &[
            0x02, 0x00, 0x00, 0x00, // two ids
            0x05, 0x00, 0x00, 0x00, // 5
            0x00, 0x00, 0x00, 0x01, // 0x01000000
        ],
    );
}

// ── client to server (61-66) ─────────────────────────────────────────────

#[test]
fn bm_search_layout() {
    let options = BMSearchOptions {
        sort_id: 2,
        client_key: UIAuctionView::MyAuctions as i32,
        sequence_id: 256,
        b_forward: 1,
        seller_name: "Ann".to_string(),
        bidder_name: String::new(),
        item_name: "Zat".to_string(),
        min_tc: 1,
        max_tc: 10,
        quality: 2000,
        filter_flags: 8,
    };
    assert_golden(
        &BMSearch {
            search_options: options,
        },
        &[
            0x02, // sortId
            0x01, 0x00, 0x00, 0x00, // clientKey = MyAuctions
            0x00, 0x01, 0x00, 0x00, // sequenceId = 256
            0x01, // bForward
            0x03, 0x00, 0x00, 0x00, b'A', b'n', b'n', // sellerName
            0x00, 0x00, 0x00, 0x00, // bidderName = ""
            0x03, 0x00, 0x00, 0x00, b'Z', b'a', b't', // itemName
            0x01, 0x00, 0x00, 0x00, // minTC
            0x0A, 0x00, 0x00, 0x00, // maxTC
            0xD0, 0x07, 0x00, 0x00, // quality = 2000
            0x08, 0x00, 0x00, 0x00, // monikerCRC / filterFlags
        ],
    );
}

/// `.def` order: `itemInstanceId, buyoutPrice, auctionLength:u8,
/// startingPrice`, 13 bytes. The unmerged server branch read `item,
/// starting, buyout, length`, which on these bytes gives a starting price of
/// 500 and a buyout of 25,605: this test fails if that order comes back.
#[test]
fn bm_create_auction_is_def_order_13_bytes() {
    let expected = [
        0x0D, 0x0C, 0x0B, 0x0A, // itemInstanceId
        0xF4, 0x01, 0x00, 0x00, // buyoutPrice = 500
        0x05, // auctionLength = VeryLong
        0x64, 0x00, 0x00, 0x00, // startingPrice = 100
    ];
    assert_eq!(expected.len(), BMCreateAuction::WIRE_LEN);
    let value = BMCreateAuction {
        item_instance_id: 0x0A0B_0C0D,
        buyout_price: 500,
        auction_length: UIAuctionTime::VeryLong as u8,
        starting_price: 100,
    };
    assert_golden(&value, &expected);
    let decoded = BMCreateAuction::decode(&expected).unwrap();
    assert_eq!(
        (decoded.buyout_price, decoded.starting_price),
        (500, 100),
        "buyout comes before auctionLength, starting price after it"
    );
}

#[test]
fn bm_place_bid_is_sequence_then_amount() {
    assert_golden(
        &BMPlaceBid {
            sequence_id: 3,
            bid_amount: 250,
        },
        &[0x03, 0x00, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00],
    );
}

#[test]
fn bm_cancel_auction_is_one_int32() {
    assert_golden(
        &BMCancelAuction { sequence_id: 9 },
        &[0x09, 0x00, 0x00, 0x00],
    );
}

#[test]
fn bm_watching_methods_are_one_int32() {
    assert_golden(
        &BMStartWatchingItem { item_def_id: 300 },
        &[0x2C, 0x01, 0x00, 0x00],
    );
    assert_golden(
        &BMStopWatchingItem { item_def_id: 300 },
        &[0x2C, 0x01, 0x00, 0x00],
    );
}

// ── the tagged enums ─────────────────────────────────────────────────────

/// `ClientCall` dispatches on the method and encodes exactly the inner
/// payload: no method id, no sub-index.
#[test]
fn client_call_round_trips_through_its_method() {
    let call = ClientCall::AuctionRemove(OnBMAuctionRemove { sequence_id: 7 });
    let bytes = call.to_bytes().unwrap();
    assert_eq!(bytes, [0x07, 0x00, 0x00, 0x00]);
    assert_eq!(call.method(), ClientMethod::OnBMAuctionRemove);
    assert_eq!(
        ClientCall::decode(ClientMethod::OnBMAuctionRemove, &bytes).unwrap(),
        call
    );
}

#[test]
fn cell_call_round_trips_through_its_method() {
    let call = CellCall::PlaceBid(BMPlaceBid {
        sequence_id: 3,
        bid_amount: 250,
    });
    let bytes = call.to_bytes().unwrap();
    assert_eq!(bytes, [0x03, 0x00, 0x00, 0x00, 0xFA, 0x00, 0x00, 0x00]);
    assert_eq!(call.method(), CellMethod::BMPlaceBid);
    assert_eq!(
        CellCall::decode(CellMethod::BMPlaceBid, &bytes).unwrap(),
        call
    );
}

// ── method ids and enums ─────────────────────────────────────────────────

/// Every Black Market method is extended: message id `0xBD`, then the
/// sub-index. Cell 61-66 are sub-indices 0-5, client 90-95 are 29-34.
#[test]
fn methods_use_the_extended_encoding() {
    assert_eq!(EXTENDED_MESSAGE_ID, (EXTENDED_METHOD_BASE as u8) | 0x80);
    let cell: Vec<u8> = CellMethod::ALL.iter().map(|m| m.sub_index()).collect();
    assert_eq!(cell, [0, 1, 2, 3, 4, 5]);
    let client: Vec<u8> = ClientMethod::ALL.iter().map(|m| m.sub_index()).collect();
    assert_eq!(client, [29, 30, 31, 32, 33, 34]);
    for m in CellMethod::ALL {
        assert_eq!(CellMethod::from_sub_index(m.sub_index()), Some(m));
        assert_eq!(CellMethod::try_from(m.index()), Ok(m));
    }
    assert_eq!(CellMethod::from_sub_index(6), None);
    assert!(CellMethod::try_from(60).is_err());
    assert!(ClientMethod::try_from(96).is_err());
}

#[test]
fn client_method_names_match_exactly() {
    for m in ClientMethod::ALL {
        assert_eq!(ClientMethod::from_name(m.name().as_bytes()), Some(m));
        assert_eq!(ClientMethod::try_from(m.index()), Ok(m));
    }
    assert_eq!(ClientMethod::from_name(b"onbmopen"), None, "case-sensitive");
    assert_eq!(
        ClientMethod::from_name(b"onBMOpenX"),
        None,
        "no prefix match"
    );
    assert_eq!(ClientMethod::from_name(b"onBMOpe"), None, "no truncation");
    assert_eq!(ClientMethod::from_name(b""), None);
    let longest = ClientMethod::ALL
        .iter()
        .map(|m| m.name().len())
        .max()
        .unwrap();
    assert_eq!(ClientMethod::MAX_NAME_LEN, longest);
}

#[test]
fn ui_enums_parse_their_wire_values() {
    assert_eq!(UIAuctionView::try_from(0), Ok(UIAuctionView::SearchResults));
    assert_eq!(UIAuctionView::try_from(1), Ok(UIAuctionView::MyAuctions));
    assert_eq!(UIAuctionView::try_from(2), Ok(UIAuctionView::MyBids));
    assert!(UIAuctionView::try_from(3).is_err());
    assert!(UIAuctionView::try_from(-1).is_err());
    let times: Vec<u8> = (0..=6)
        .filter(|v| UIAuctionTime::try_from(*v).is_ok())
        .collect();
    assert_eq!(times, [1, 2, 3, 4, 5], "UIAuctionTime is 1-based");
    assert_eq!(UIAuctionTime::try_from(5), Ok(UIAuctionTime::VeryLong));
}
