//! The encoders against the entity definitions and the dispatch tables,
//! read from the repo rather than restated here.
//!
//! Each fixture gives the bytes of every field **by name**; the expected
//! encoding is those bytes concatenated in the order the `.def` or
//! `alias.xml` lists the names. So an encoder that writes two fields in the
//! wrong order fails, and so does a fixture that forgets a field, without
//! the test itself repeating the order it checks.

use std::collections::BTreeMap;
use std::path::Path;

use super::{sample_item, SAMPLE_ITEM_BYTES};
use crate::black_market::*;
use crate::Encode;

const DEF_PATH: &str = "entities/defs/interfaces/SGWBlackMarketManager.def";
const ALIAS_PATH: &str = "entities/defs/alias.xml";

fn repo_file(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The text between `open` and the next `close` after it.
fn between<'a>(text: &'a str, open: &str, close: &str) -> &'a str {
    let start = text
        .find(open)
        .unwrap_or_else(|| panic!("`{open}` not found"))
        + open.len();
    let len = text[start..]
        .find(close)
        .unwrap_or_else(|| panic!("`{close}` not found after `{open}`"));
    &text[start..start + len]
}

/// Collapse whitespace and spell `ARRAY <of> X </of>` as `ARRAY of X`.
fn normalise_type(raw: &str) -> String {
    raw.replace("<of>", " of ")
        .replace("</of>", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `(type, name)` for each `<Arg>` of `method` in the `.def`'s `section`
/// (`ClientMethods` or `CellMethods`).
fn def_args(section: &str, method: &str) -> Vec<(String, String)> {
    let def = repo_file(DEF_PATH);
    let block = between(&def, &format!("<{section}>"), &format!("</{section}>"));
    let body = between(block, &format!("<{method}>"), &format!("</{method}>"));
    body.split("<Arg>")
        .skip(1)
        .map(|arg| {
            let arg = &arg[..arg.find("</Arg>").expect("closed <Arg>")];
            let (ty, rest) = arg.split_once("<ArgName>").expect("named <Arg>");
            let name = &rest[..rest.find("</ArgName>").expect("closed <ArgName>")];
            (normalise_type(ty), name.trim().to_string())
        })
        .collect()
}

/// Method element names inside the `.def`'s `section`, in file order.
fn def_method_names(section: &str) -> Vec<String> {
    let def = repo_file(DEF_PATH);
    let block = between(&def, &format!("<{section}>"), &format!("</{section}>"));
    let mut names = Vec::new();
    let mut rest = block;
    while let Some(open) = rest.find('<') {
        let tail = &rest[open + 1..];
        let end = tail.find('>').expect("closed tag");
        let tag = &tail[..end];
        let is_method = !tag.starts_with('/')
            && !tag.starts_with('!')
            && tag != "Arg"
            && tag != "ArgName"
            && tag != "Exposed/"
            && tag != "of";
        if is_method {
            names.push(tag.to_string());
            let close = format!("</{tag}>");
            let skip = tail.find(&close).expect("closed method");
            rest = &tail[skip + close.len()..];
        } else {
            rest = &tail[end + 1..];
        }
    }
    names
}

/// `(type, name)` for each property of the `FIXED_DICT` alias `alias`.
fn alias_properties(alias: &str) -> Vec<(String, String)> {
    let xml = repo_file(ALIAS_PATH);
    let dict = between(
        &xml,
        &format!("<{alias}>FIXED_DICT"),
        &format!("</{alias}>"),
    );
    let props = between(dict, "<Properties>", "</Properties>");
    props
        .split("<Type>")
        .collect::<Vec<_>>()
        .windows(2)
        .map(|pair| {
            let before = pair[0];
            let open = before.rfind('<').expect("property tag");
            let name = &before[open + 1..before[open..].find('>').unwrap() + open];
            let ty = &pair[1][..pair[1].find("</Type>").expect("closed <Type>")];
            (normalise_type(ty), name.to_string())
        })
        .collect()
}

fn i32_bytes(v: i32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

fn string_bytes(s: &str) -> Vec<u8> {
    let mut out = (s.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

/// Check that `encoded` is the fixture's fields in `layout` order, that the
/// fixture names exactly the layout's fields, and that each field's bytes
/// have the shape its declared type needs.
fn assert_layout(
    what: &str,
    layout: &[(String, String)],
    fixture: &[(&str, Vec<u8>)],
    encoded: &[u8],
) {
    let by_name: BTreeMap<&str, &Vec<u8>> = fixture.iter().map(|(n, b)| (*n, b)).collect();
    let layout_names: Vec<&str> = layout.iter().map(|(_, n)| n.as_str()).collect();
    let fixture_names: Vec<&str> = by_name.keys().copied().collect();
    let mut sorted_layout = layout_names.clone();
    sorted_layout.sort_unstable();
    assert_eq!(
        fixture_names, sorted_layout,
        "{what}: the fixture must name exactly the definition's fields"
    );

    let mut expected = Vec::new();
    for (ty, name) in layout {
        let bytes = by_name[name.as_str()];
        match ty.as_str() {
            "INT32" => assert_eq!(bytes.len(), 4, "{what}.{name} is INT32"),
            "UINT8" => assert_eq!(bytes.len(), 1, "{what}.{name} is UINT8"),
            "STRING" => {
                let len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                assert_eq!(bytes.len(), 4 + len, "{what}.{name} is a STRING");
            }
            t if t.starts_with("ARRAY of ") => {
                assert!(bytes.len() >= 4, "{what}.{name} is an ARRAY")
            }
            "AuctionItem" | "BMSearchOptions" => {}
            other => panic!("{what}.{name}: unexpected type `{other}`"),
        }
        expected.extend_from_slice(bytes);
    }
    assert_eq!(encoded, expected.as_slice(), "{what}: field order");
}

#[test]
fn auction_item_follows_alias_xml() {
    let layout = alias_properties("AuctionItem");
    assert_eq!(layout.len(), 10);
    let item = AuctionItem {
        sequence_id: 1,
        item_def_id: 2,
        stack_size: 3,
        durability: 4,
        charges: 5,
        current_bid: 6,
        buyout_price: 7,
        end_time_value: 8,
        next_min_bid_price: 9,
        seller_name: "Sel".into(),
    };
    let fixture = [
        ("sequenceId", i32_bytes(1)),
        ("itemDefId", i32_bytes(2)),
        ("stackSize", i32_bytes(3)),
        ("durability", i32_bytes(4)),
        ("charges", i32_bytes(5)),
        ("currentBid", i32_bytes(6)),
        ("buyoutPrice", i32_bytes(7)),
        ("endTimeValue", vec![8]),
        ("nextMinBidPrice", i32_bytes(9)),
        ("sellerName", string_bytes("Sel")),
    ];
    assert_layout("AuctionItem", &layout, &fixture, &item.to_bytes().unwrap());
}

#[test]
fn search_options_follow_alias_xml() {
    let layout = alias_properties("BMSearchOptions");
    assert_eq!(layout.len(), 11);
    let options = BMSearchOptions {
        sort_id: 1,
        client_key: 2,
        sequence_id: 3,
        b_forward: 4,
        seller_name: "S".into(),
        bidder_name: "Bi".into(),
        item_name: "Itm".into(),
        min_tc: 5,
        max_tc: 6,
        quality: 7,
        filter_flags: 8,
    };
    let fixture = [
        ("sortId", vec![1]),
        ("clientKey", i32_bytes(2)),
        ("sequenceId", i32_bytes(3)),
        ("bForward", vec![4]),
        ("sellerName", string_bytes("S")),
        ("bidderName", string_bytes("Bi")),
        ("itemName", string_bytes("Itm")),
        ("minTC", i32_bytes(5)),
        ("maxTC", i32_bytes(6)),
        ("quality", i32_bytes(7)),
        // The client's emitter calls this field `filterFlags`.
        ("monikerCRC", i32_bytes(8)),
    ];
    assert_layout(
        "BMSearchOptions",
        &layout,
        &fixture,
        &options.to_bytes().unwrap(),
    );
}

#[test]
fn client_methods_follow_the_def() {
    let mut one_item = vec![0x01, 0x00, 0x00, 0x00];
    one_item.extend_from_slice(SAMPLE_ITEM_BYTES);
    let checks: [(ClientCall, Vec<(&str, Vec<u8>)>); 6] = [
        (
            ClientCall::Open(OnBMOpen { entity_id: 11 }),
            vec![("entityId", i32_bytes(11))],
        ),
        (
            ClientCall::Error(OnBMError { error_id: 12 }),
            vec![("errorId", i32_bytes(12))],
        ),
        (
            ClientCall::Auctions(OnBMAuctions {
                auction_items: vec![sample_item()],
                total_results: 13,
                client_key: 14,
            }),
            vec![
                ("auctionItems", one_item),
                ("totalResults", i32_bytes(13)),
                ("clientKey", i32_bytes(14)),
            ],
        ),
        (
            ClientCall::AuctionRemove(OnBMAuctionRemove { sequence_id: 15 }),
            vec![("sequenceId", i32_bytes(15))],
        ),
        (
            ClientCall::AuctionUpdate(OnBMAuctionUpdate {
                auction_item: sample_item(),
            }),
            vec![("auctionItem", SAMPLE_ITEM_BYTES.to_vec())],
        ),
        (
            ClientCall::WatchedItemsUpdate(OnBMWatchedItemsUpdate {
                item_list: vec![16],
            }),
            vec![("itemList", [i32_bytes(1), i32_bytes(16)].concat())],
        ),
    ];
    for (call, fixture) in checks {
        let name = call.method().name();
        let layout = def_args("ClientMethods", name);
        assert_layout(name, &layout, &fixture, &call.to_bytes().unwrap());
    }
}

#[test]
fn cell_methods_follow_the_def() {
    let checks: [(CellCall, Vec<(&str, Vec<u8>)>); 6] = [
        (
            CellCall::Search(BMSearch::default()),
            vec![(
                "searchOptions",
                BMSearchOptions::default().to_bytes().unwrap(),
            )],
        ),
        (
            CellCall::CreateAuction(BMCreateAuction {
                item_instance_id: 21,
                buyout_price: 22,
                auction_length: 3,
                starting_price: 24,
            }),
            vec![
                ("itemInstanceId", i32_bytes(21)),
                ("buyoutPrice", i32_bytes(22)),
                ("auctionLength", vec![3]),
                ("startingPrice", i32_bytes(24)),
            ],
        ),
        (
            CellCall::PlaceBid(BMPlaceBid {
                sequence_id: 25,
                bid_amount: 26,
            }),
            vec![("sequenceId", i32_bytes(25)), ("bidAmount", i32_bytes(26))],
        ),
        (
            CellCall::CancelAuction(BMCancelAuction { sequence_id: 27 }),
            vec![("sequenceId", i32_bytes(27))],
        ),
        (
            CellCall::StartWatchingItem(BMStartWatchingItem { item_def_id: 28 }),
            vec![("itemDefId", i32_bytes(28))],
        ),
        (
            CellCall::StopWatchingItem(BMStopWatchingItem { item_def_id: 29 }),
            vec![("itemDefId", i32_bytes(29))],
        ),
    ];
    for (call, fixture) in checks {
        let name = call.method().name();
        let layout = def_args("CellMethods", name);
        assert_layout(name, &layout, &fixture, &call.to_bytes().unwrap());
    }
}

/// The interface's methods are flattened consecutively, so the `.def`
/// order of each section must be the index order of the enums.
#[test]
fn def_method_order_matches_the_indices() {
    let client: Vec<&str> = ClientMethod::ALL.iter().map(|m| m.name()).collect();
    assert_eq!(def_method_names("ClientMethods"), client);
    let cell: Vec<&str> = CellMethod::ALL.iter().map(|m| m.name()).collect();
    assert_eq!(def_method_names("CellMethods"), cell);
}

/// The flattened indices against the canonical dispatch tables.
#[test]
fn indices_match_the_dispatch_tables() {
    let client_table = repo_file("docs/protocol/client-method-dispatch-table.md");
    for m in ClientMethod::ALL {
        let row = format!("| {} | `{}` |", m.index(), m.name());
        assert!(
            client_table.contains(&row),
            "client-method-dispatch-table.md has no row `{row}`"
        );
    }
    let cell_table = repo_file("docs/protocol/cell-method-dispatch-table.md");
    for m in CellMethod::ALL {
        let row = format!("| {} | {} | YES |", m.index(), m.name());
        assert!(
            cell_table.contains(&row),
            "cell-method-dispatch-table.md has no row `{row}`"
        );
    }
}
