//! The store methods (78-82) through the vendor dispatcher, fed the stock
//! client's bytes (`inventory-wire-formats.md` §Store Operations).
//!
//! Each happy-path payload here was refused by the old `(id, qty)`-pair
//! parser: a purchase/sell/buyback had its second array's count read as a
//! pair and its last quantity read as a "template id" (refused as
//! `template_mismatch`), and a repair/recharge was too short to be a pair
//! (refused as `malformed_args`).

use cimmeria_wire::cell::chat::CHAN_FEEDBACK;
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use tokio::sync::mpsc;
use tracing::Level;

use crate::cell::combat::BSF_DEAD;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};

use super::super::constants::{
    BUYBACK_ITEMS, PURCHASE_ITEMS, RECHARGE_ITEMS, REPAIR_ITEMS, SELL_ITEMS,
};
use super::dispatch;
use super::store_ops::{NO_VENDOR_TEXT, TOO_FAR_TEXT};

const ENTITY: u32 = 1;
const ACCOUNT: u32 = 7;
const PLAYER: i32 = 4242;
const TEMPLATE: i32 = 9001;

/// A player (entity 1). `vendor`: open a vendor, with `TEMPLATE` when
/// `with_template`. Returns the manager and the vendor's entity id.
fn player(vendor: bool, with_template: bool) -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(ENTITY, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let vendor_id = mgr.allocate_npc_id();
    mgr.spawn_npc(vendor_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if with_template {
        mgr.get_entity_mut(vendor_id).unwrap().template_id = Some(TEMPLATE);
    }
    let p = mgr.get_entity_mut(ENTITY).unwrap();
    p.is_player = true;
    p.account_id = Some(ACCOUNT);
    p.player_id = Some(PLAYER);
    if vendor {
        p.vendor_entity = Some(vendor_id);
    }
    (mgr, vendor_id)
}

fn array(values: &[i32]) -> Vec<u8> {
    let mut out = (values.len() as u32).to_le_bytes().to_vec();
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn two_arrays(ids: &[i32], quantities: &[i32]) -> Vec<u8> {
    let mut out = array(ids);
    out.extend(array(quantities));
    out
}

async fn send(mgr: &mut SpaceManager, method: u16, args: &[u8]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    assert!(dispatch(ENTITY, method, args, &tx, mgr).await, "claimed");
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// The refusal's only message: one `CHAN_FEEDBACK` line to the player.
fn only_feedback_text(msgs: &[CellToBaseMsg]) -> String {
    let [CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    }] = msgs
    else {
        panic!("expected exactly one feedback line, got {msgs:?}");
    };
    assert_eq!(*entity_id, ENTITY);
    assert_eq!(*method_index, ON_PLAYER_COMMUNICATION);
    let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
    let at = 4 + speaker_len * 2;
    assert_eq!(args[at + 1], CHAN_FEEDBACK);
    let text_len = u32::from_le_bytes(args[at + 2..at + 6].try_into().unwrap()) as usize;
    let units: Vec<u16> = args[at + 6..at + 6 + text_len * 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16(&units).unwrap()
}

#[tokio::test]
async fn stock_purchase_of_one_line_reaches_the_base_with_the_server_template() {
    let (mut mgr, vendor) = player(true, true);
    // [n=1][ItemIndices: 4][m=1][Quantities: 5]
    let bytes = [1, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 5, 0, 0, 0];
    match send(&mut mgr, PURCHASE_ITEMS, &bytes).await.as_slice() {
        [CellToBaseMsg::PurchaseVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        }] => {
            assert_eq!(
                (*entity_id, *player_id, *vendor_entity_id),
                (ENTITY, PLAYER, vendor as i32)
            );
            assert_eq!(*vendor_template_id, TEMPLATE);
            assert_eq!(items, &vec![(4, 5)]);
        }
        other => panic!("expected PurchaseVendorItems, got {other:?}"),
    }
}

#[tokio::test]
async fn stock_purchase_of_three_lines_keeps_every_line() {
    let (mut mgr, _) = player(true, true);
    let bytes = two_arrays(&[0, 7, 12], &[1, 20, 3]);
    match send(&mut mgr, PURCHASE_ITEMS, &bytes).await.as_slice() {
        [CellToBaseMsg::PurchaseVendorItems { items, .. }] => {
            assert_eq!(items, &vec![(0, 1), (7, 20), (12, 3)]);
        }
        other => panic!("expected PurchaseVendorItems, got {other:?}"),
    }
}

#[tokio::test]
async fn stock_sell_reaches_the_base() {
    let (mut mgr, _) = player(true, true);
    let bytes = two_arrays(&[555], &[10]);
    match send(&mut mgr, SELL_ITEMS, &bytes).await.as_slice() {
        [CellToBaseMsg::SellVendorItems {
            vendor_template_id,
            items,
            ..
        }] => {
            assert_eq!(*vendor_template_id, TEMPLATE);
            assert_eq!(items, &vec![(555, 10)]);
        }
        other => panic!("expected SellVendorItems, got {other:?}"),
    }
}

#[tokio::test]
async fn stock_buyback_reaches_the_base() {
    let (mut mgr, _) = player(true, true);
    let bytes = two_arrays(&[901, 902], &[1, 4]);
    match send(&mut mgr, BUYBACK_ITEMS, &bytes).await.as_slice() {
        [CellToBaseMsg::BuybackVendorItems {
            vendor_template_id,
            items,
            ..
        }] => {
            assert_eq!(*vendor_template_id, TEMPLATE);
            assert_eq!(items, &vec![(901, 1), (902, 4)]);
        }
        other => panic!("expected BuybackVendorItems, got {other:?}"),
    }
}

/// CAT-E-01 guard (with `stock_recharge_is_priced_by_the_open_vendor` and
/// the `i32` type of `vendor_template_id`). A one-item stock repair is 8
/// bytes and names no vendor; with a vendor session open, what reaches the
/// base is a priced repair carrying the session's template. The free path
/// was exactly this case: an open session plus a payload with no trailing
/// template, forwarded as `vendor_template_id: None`.
#[tokio::test]
async fn stock_repair_of_one_item_is_priced_by_the_open_vendor() {
    let (mut mgr, _) = player(true, true);
    let bytes = [1, 0, 0, 0, 42, 0, 0, 0];
    match send(&mut mgr, REPAIR_ITEMS, &bytes).await.as_slice() {
        [CellToBaseMsg::RepairInventoryItems {
            player_id,
            item_ids,
            vendor_template_id,
            ..
        }] => {
            assert_eq!(*player_id, PLAYER);
            assert_eq!(item_ids, &vec![42]);
            assert_eq!(*vendor_template_id, TEMPLATE);
        }
        other => panic!("expected RepairInventoryItems, got {other:?}"),
    }
}

#[tokio::test]
async fn stock_repair_of_three_items_reaches_the_base() {
    let (mut mgr, _) = player(true, true);
    match send(&mut mgr, REPAIR_ITEMS, &array(&[42, 43, 44]))
        .await
        .as_slice()
    {
        [CellToBaseMsg::RepairInventoryItems { item_ids, .. }] => {
            assert_eq!(item_ids, &vec![42, 43, 44]);
        }
        other => panic!("expected RepairInventoryItems, got {other:?}"),
    }
}

#[tokio::test]
async fn stock_recharge_is_priced_by_the_open_vendor() {
    let (mut mgr, _) = player(true, true);
    match send(&mut mgr, RECHARGE_ITEMS, &array(&[77]))
        .await
        .as_slice()
    {
        [CellToBaseMsg::RechargeInventoryItems {
            item_ids,
            vendor_template_id,
            ..
        }] => {
            assert_eq!(item_ids, &vec![77]);
            assert_eq!(*vendor_template_id, TEMPLATE);
        }
        other => panic!("expected RechargeInventoryItems, got {other:?}"),
    }
}

/// No vendor open: the request never reaches the base, the player gets a
/// feedback line, and the refusal is one WARN. The old cell refused a
/// sessionless request too, so this is not the CAT-E-01 guard.
#[tokio::test]
async fn repair_without_vendor_session_is_refused() {
    let (mut mgr, _) = player(false, true);
    let capture = LogCapture::install();
    let msgs = send(&mut mgr, REPAIR_ITEMS, &array(&[42])).await;
    assert_eq!(only_feedback_text(&msgs), NO_VENDOR_TEXT);
    let row = capture
        .find_event(Level::WARN, "vendor op refused", "no_vendor_session")
        .expect("refusal row");
    assert_eq!(row.target, "vendor");
    for (k, v) in [
        ("event", "refused"),
        ("action", "repair"),
        ("account_id", "7"),
        ("player_id", "4242"),
        ("item_count", "1"),
    ] {
        assert!(row.has_field(k, v), "field {k}={v}: {row:#?}");
    }
}

#[tokio::test]
async fn recharge_at_a_vendor_without_a_template_is_refused_with_feedback() {
    let (mut mgr, vendor) = player(true, false);
    let capture = LogCapture::install();
    let msgs = send(&mut mgr, RECHARGE_ITEMS, &array(&[77, 78])).await;
    assert_eq!(only_feedback_text(&msgs), NO_VENDOR_TEXT);
    let row = capture
        .find_event(Level::WARN, "vendor op refused", "vendor_has_no_template")
        .expect("refusal row");
    assert!(row.has_field("action", "recharge"));
    assert!(row.has_field("vendor_entity_id", &vendor.to_string()));
    assert!(row.has_field("item_count", "2"));
}

#[tokio::test]
async fn purchase_without_a_vendor_session_is_refused_with_feedback() {
    let (mut mgr, _) = player(false, true);
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &two_arrays(&[4], &[1])).await;
    assert_eq!(only_feedback_text(&msgs), NO_VENDOR_TEXT);
}

/// A trailing i32 (the old parser's "template id") is malformed now:
/// nothing is forwarded and nothing is said to a client that forged it.
#[tokio::test]
async fn trailing_template_id_is_malformed_and_sends_nothing() {
    let (mut mgr, _) = player(true, true);
    let mut bytes = two_arrays(&[4], &[1]);
    bytes.extend_from_slice(&TEMPLATE.to_le_bytes());
    let capture = LogCapture::install();
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &bytes).await;
    assert!(msgs.is_empty(), "{msgs:?}");
    let row = capture
        .find_event(Level::WARN, "vendor op refused", "malformed_args")
        .expect("refusal row");
    assert!(row.has_field("wire_reason", "trailing_bytes"), "{row:#?}");
    assert!(row.has_field("action", "buy"));
}

#[tokio::test]
async fn mismatched_arrays_and_short_counts_send_nothing() {
    let (mut mgr, _) = player(true, true);
    for (method, bytes) in [
        (SELL_ITEMS, two_arrays(&[1, 2], &[1])),
        (BUYBACK_ITEMS, array(&[1])),
        (REPAIR_ITEMS, vec![1, 0, 0]),
        (RECHARGE_ITEMS, {
            let mut b = 3u32.to_le_bytes().to_vec();
            b.extend_from_slice(&1i32.to_le_bytes());
            b
        }),
    ] {
        let msgs = send(&mut mgr, method, &bytes).await;
        assert!(msgs.is_empty(), "method {method}: {msgs:?}");
    }
}

/// Finding R1: the vendor stays open after the player walks away, so a
/// purchase from 40 u is refused with one feedback line and nothing else.
#[tokio::test]
async fn purchase_from_a_vendor_out_of_range_is_refused_with_feedback() {
    let (mut mgr, vendor) = player(true, true);
    mgr.update_entity_position(vendor, [40.0, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &two_arrays(&[4], &[1])).await;
    assert_eq!(only_feedback_text(&msgs), TOO_FAR_TEXT);
}

#[tokio::test]
async fn sell_buyback_repair_recharge_out_of_range_are_refused() {
    let (mut mgr, vendor) = player(true, true);
    mgr.update_entity_position(vendor, [40.0, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    for (method, bytes) in [
        (SELL_ITEMS, two_arrays(&[555], &[10])),
        (BUYBACK_ITEMS, two_arrays(&[901], &[1])),
        (REPAIR_ITEMS, array(&[42])),
        (RECHARGE_ITEMS, array(&[77])),
    ] {
        let msgs = send(&mut mgr, method, &bytes).await;
        assert_eq!(only_feedback_text(&msgs), TOO_FAR_TEXT, "method {method}");
    }
}

/// A dead player gets no feedback and nothing reaches the base.
#[tokio::test]
async fn purchase_while_dead_sends_nothing() {
    let (mut mgr, _) = player(true, true);
    mgr.get_entity_mut(ENTITY).unwrap().state_field |= BSF_DEAD;
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &two_arrays(&[4], &[1])).await;
    assert!(msgs.is_empty(), "{msgs:?}");
}

#[tokio::test]
async fn out_of_range_refusal_logs_the_reason() {
    let (mut mgr, vendor) = player(true, true);
    mgr.update_entity_position(vendor, [40.0, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    let capture = LogCapture::install();
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &two_arrays(&[4], &[1])).await;
    assert_eq!(only_feedback_text(&msgs), TOO_FAR_TEXT);
    let row = capture
        .find_event(Level::WARN, "vendor op refused", "out_of_range")
        .expect("refusal row");
    assert_eq!(row.target, "vendor");
    assert!(row.has_field("action", "buy"), "{row:#?}");
    assert!(row.has_field("item_count", "1"), "{row:#?}");
}

/// The gate is the interact range (5 u), not tighter: a purchase from 4.9 u
/// still reaches the base, and one from 5.2 u is refused.
#[tokio::test]
async fn purchase_at_the_edge_of_interact_range_is_forwarded_just_past_it_refused() {
    let (mut mgr, vendor) = player(true, true);
    mgr.update_entity_position(vendor, [4.9, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &two_arrays(&[4], &[1])).await;
    assert!(
        matches!(msgs.as_slice(), [CellToBaseMsg::PurchaseVendorItems { .. }]),
        "{msgs:?}"
    );
    mgr.update_entity_position(vendor, [5.2, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
    let msgs = send(&mut mgr, PURCHASE_ITEMS, &two_arrays(&[4], &[1])).await;
    assert_eq!(only_feedback_text(&msgs), TOO_FAR_TEXT);
}
