//! Decoders are total: every malformed input is an error, never a panic,
//! and nothing is allocated or read on the input's say-so.

use super::{sample_item, StrictSource, SAMPLE_ITEM_BYTES};
use crate::black_market::*;
use crate::{Decode, DecodeError, Encode, EncodeError, SliceSource};

/// One well-formed payload per method, with a decoder for it.
struct Case {
    name: &'static str,
    bytes: Vec<u8>,
    decode_slice: fn(&[u8]) -> Result<(), DecodeError>,
    decode_strict: fn(&mut StrictSource<'_>) -> Result<(), DecodeError>,
}

fn case<T: Decode + Encode>(name: &'static str, value: T) -> Case {
    Case {
        name,
        bytes: value.to_bytes().unwrap(),
        decode_slice: |b| T::decode(b).map(|_| ()),
        decode_strict: |s| T::decode_from(s).map(|_| ()),
    }
}

fn cases() -> Vec<Case> {
    let search = BMSearchOptions {
        seller_name: "Ann".into(),
        bidder_name: "Bob".into(),
        item_name: "Zat".into(),
        quality: 2000,
        ..Default::default()
    };
    let mut second = sample_item();
    second.seller_name = String::new();
    vec![
        case("onBMOpen", OnBMOpen { entity_id: 5 }),
        case("onBMError", OnBMError { error_id: 1 }),
        case(
            "onBMAuctions",
            OnBMAuctions {
                auction_items: vec![sample_item(), second],
                total_results: 40,
                client_key: 0,
            },
        ),
        case("onBMAuctionRemove", OnBMAuctionRemove { sequence_id: 7 }),
        case(
            "onBMAuctionUpdate",
            OnBMAuctionUpdate {
                auction_item: sample_item(),
            },
        ),
        case(
            "onBMWatchedItemsUpdate",
            OnBMWatchedItemsUpdate {
                item_list: vec![1, 2, 3],
            },
        ),
        case(
            "BMSearch",
            BMSearch {
                search_options: search,
            },
        ),
        case(
            "BMCreateAuction",
            BMCreateAuction {
                item_instance_id: 1,
                buyout_price: 2,
                auction_length: 3,
                starting_price: 4,
            },
        ),
        case(
            "BMPlaceBid",
            BMPlaceBid {
                sequence_id: 1,
                bid_amount: 2,
            },
        ),
        case("BMCancelAuction", BMCancelAuction { sequence_id: 1 }),
        case(
            "BMStartWatchingItem",
            BMStartWatchingItem { item_def_id: 1 },
        ),
        case("BMStopWatchingItem", BMStopWatchingItem { item_def_id: 1 }),
    ]
}

/// Cutting any payload short at any byte is a `Truncated` error.
#[test]
fn every_truncation_is_an_error() {
    for c in cases() {
        for cut in 0..c.bytes.len() {
            match (c.decode_slice)(&c.bytes[..cut]) {
                Err(DecodeError::Truncated { .. }) => {}
                other => panic!(
                    "{}: {cut} of {} bytes decoded as {other:?}, expected Truncated",
                    c.name,
                    c.bytes.len()
                ),
            }
        }
        assert_eq!((c.decode_slice)(&c.bytes), Ok(()), "{} whole", c.name);
    }
}

/// Over the pull source the DLL uses, every decoder checks `remaining`
/// before each read and never asks for more than is there, whole or cut
/// short, and a whole payload is consumed exactly.
#[test]
fn pull_decoding_never_reads_unchecked_or_past_the_end() {
    for c in cases() {
        for cut in 0..=c.bytes.len() {
            let mut src = StrictSource::new(&c.bytes[..cut]);
            let result = (c.decode_strict)(&mut src);
            assert!(
                src.violations.is_empty(),
                "{} cut at {cut}: {:?}",
                c.name,
                src.violations
            );
            if cut == c.bytes.len() {
                assert_eq!(result, Ok(()), "{}", c.name);
                assert_eq!(src.pos, cut, "{} must consume exactly its bytes", c.name);
            } else {
                assert!(result.is_err(), "{} cut at {cut} decoded", c.name);
            }
        }
    }
}

/// `decode_from` leaves what follows the payload alone: the client's
/// stream belongs to the caller once the arguments are read.
#[test]
fn decode_from_stops_at_the_end_of_the_payload() {
    let mut bytes = SAMPLE_ITEM_BYTES.to_vec();
    bytes.extend_from_slice(&[0xAA, 0xBB]);
    let mut src = SliceSource::new(&bytes);
    assert_eq!(AuctionItem::decode_from(&mut src), Ok(sample_item()));
    assert_eq!(src.position(), SAMPLE_ITEM_BYTES.len());
}

/// A whole-payload decode rejects trailing bytes: they mean the two sides
/// disagree about the layout.
#[test]
fn whole_payload_decode_rejects_trailing_bytes() {
    for c in cases() {
        let mut bytes = c.bytes.clone();
        bytes.push(0);
        assert_eq!(
            (c.decode_slice)(&bytes),
            Err(DecodeError::TrailingBytes {
                consumed: c.bytes.len(),
                trailing: 1
            }),
            "{}",
            c.name
        );
    }
    assert!(matches!(
        CellCall::decode(CellMethod::BMCancelAuction, &[1, 0, 0, 0, 9]),
        Err(DecodeError::TrailingBytes { .. })
    ));
    assert!(matches!(
        ClientCall::decode(ClientMethod::OnBMOpen, &[1, 0, 0, 0, 9]),
        Err(DecodeError::TrailingBytes { .. })
    ));
}

#[test]
fn auction_count_over_the_cap_is_rejected() {
    for count in [MAX_AUCTION_ITEMS + 1, u32::MAX] {
        let mut bytes = count.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0; 64]);
        assert_eq!(
            OnBMAuctions::decode(&bytes),
            Err(DecodeError::CountTooLarge {
                field: "auctionItems",
                count,
                max: MAX_AUCTION_ITEMS
            })
        );
    }
}

/// A count within the cap that the bytes cannot back is rejected before
/// any element is allocated or read.
#[test]
fn auction_count_larger_than_the_payload_is_rejected_up_front() {
    let mut bytes = MAX_AUCTION_ITEMS.to_le_bytes().to_vec();
    bytes.extend_from_slice(SAMPLE_ITEM_BYTES);
    assert_eq!(
        OnBMAuctions::decode(&bytes),
        Err(DecodeError::Truncated {
            field: "auctionItems",
            needed: MAX_AUCTION_ITEMS as usize * AuctionItem::MIN_WIRE_LEN,
            remaining: SAMPLE_ITEM_BYTES.len()
        })
    );
}

#[test]
fn watched_items_count_over_the_cap_is_rejected() {
    let bytes = (MAX_WATCHED_ITEMS + 1).to_le_bytes();
    assert_eq!(
        OnBMWatchedItemsUpdate::decode(&bytes),
        Err(DecodeError::CountTooLarge {
            field: "itemList",
            count: MAX_WATCHED_ITEMS + 1,
            max: MAX_WATCHED_ITEMS
        })
    );
}

/// Replace `sample_item`'s `sellerName` on the wire with `len` and `body`.
fn item_with_raw_seller(len: u32, body: &[u8]) -> Vec<u8> {
    let mut bytes = SAMPLE_ITEM_BYTES[..AuctionItem::MIN_WIRE_LEN - 4].to_vec();
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}

#[test]
fn string_at_the_cap_decodes_and_over_it_is_rejected() {
    let at_cap = vec![b'x'; MAX_STRING_LEN as usize];
    let bytes = item_with_raw_seller(MAX_STRING_LEN, &at_cap);
    assert_eq!(
        AuctionItem::decode(&bytes).unwrap().seller_name.len(),
        MAX_STRING_LEN as usize
    );

    let over = vec![b'x'; MAX_STRING_LEN as usize + 1];
    for len in [MAX_STRING_LEN + 1, u32::MAX] {
        assert_eq!(
            AuctionItem::decode(&item_with_raw_seller(len, &over)),
            Err(DecodeError::StringTooLong {
                field: "sellerName",
                len,
                max: MAX_STRING_LEN
            })
        );
    }
}

#[test]
fn invalid_utf8_is_rejected() {
    let bytes = item_with_raw_seller(2, &[0xC3, 0x28]);
    assert_eq!(
        AuctionItem::decode(&bytes),
        Err(DecodeError::InvalidUtf8 {
            field: "sellerName"
        })
    );
}

/// The encoders enforce the decoders' caps, so the server can never emit
/// a payload the DLL rejects, and a failed `encode` leaves the buffer as it
/// was.
#[test]
fn encoders_enforce_the_caps_atomically() {
    let mut item = sample_item();
    item.seller_name = "x".repeat(MAX_STRING_LEN as usize + 1);
    let mut out = vec![0xEE];
    assert_eq!(
        item.encode(&mut out),
        Err(EncodeError::StringTooLong {
            field: "sellerName",
            len: MAX_STRING_LEN as usize + 1,
            max: MAX_STRING_LEN
        })
    );
    assert_eq!(out, [0xEE], "a failed encode must not leave partial bytes");

    let page = OnBMAuctions {
        auction_items: vec![sample_item(); MAX_AUCTION_ITEMS as usize + 1],
        total_results: 0,
        client_key: 0,
    };
    assert!(matches!(
        page.to_bytes(),
        Err(EncodeError::CountTooLarge {
            field: "auctionItems",
            ..
        })
    ));

    let watched = OnBMWatchedItemsUpdate {
        item_list: vec![0; MAX_WATCHED_ITEMS as usize + 1],
    };
    assert!(watched.to_bytes().is_err());

    let search = BMSearchOptions {
        item_name: "x".repeat(MAX_STRING_LEN as usize + 1),
        ..Default::default()
    };
    assert!(matches!(
        search.to_bytes(),
        Err(EncodeError::StringTooLong {
            field: "itemName",
            ..
        })
    ));
}

/// The caps are sized so the largest `onBMAuctions` they allow still fits
/// one entity message: the server's framing caps a message body at 65,535
/// bytes, 5 of which are the entity id and the extended sub-index byte.
#[test]
fn largest_auction_page_fits_one_entity_message() {
    let mut item = sample_item();
    item.seller_name = "x".repeat(MAX_STRING_LEN as usize);
    let page = OnBMAuctions {
        auction_items: vec![item; MAX_AUCTION_ITEMS as usize],
        total_results: i32::MAX,
        client_key: 0,
    };
    let bytes = page.to_bytes().unwrap();
    assert_eq!(bytes.len(), 58_412);
    assert!(bytes.len() <= usize::from(u16::MAX) - 5);
    assert_eq!(OnBMAuctions::decode(&bytes).unwrap(), page);
}
