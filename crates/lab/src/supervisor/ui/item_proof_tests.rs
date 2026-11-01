//! Tests for the item, drag, vendor and world-map checks (AX-41). Pure:
//! no supervisor and no bridge.

use serde_json::{json, Value};

use super::{drag_checks, item_checks, item_evidence, map_checks, vendor_checks, LootFacts};
use crate::proof::{EvVerdict, Evidence, Why};

#[test]
fn item_use_that_changed_nothing_denies() {
    let ev = Evidence::judge(item_checks(
        true,
        false,
        &json!({ "changed": false }),
        None,
        false,
    ));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NoEffect);
}

#[test]
fn negative_item_check_confirms_when_unchanged() {
    let unchanged = Evidence::judge(item_checks(false, false, &Value::Null, None, false));
    assert_eq!(unchanged.verdict, EvVerdict::Confirmed);
    let changed = Evidence::judge(item_checks(false, true, &Value::Null, None, false));
    assert_eq!(changed.verdict, EvVerdict::Denied);
}

#[test]
fn loot_counts_down_confirms() {
    let down = LootFacts {
        count_before: 3,
        count_after: 2,
        window_open_after: true,
    };
    let ev = Evidence::judge(item_checks(true, false, &Value::Null, Some(down), false));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let same = LootFacts {
        count_before: 3,
        count_after: 3,
        window_open_after: true,
    };
    let ev = Evidence::judge(item_checks(true, false, &Value::Null, Some(same), false));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NoEffect);
}

#[test]
fn double_use_risk_never_decides() {
    let checks = item_checks(true, true, &Value::Null, None, true);
    let ev = Evidence::judge(checks.clone());
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let risk = checks
        .iter()
        .find(|c| c.name == "double_use_risk")
        .expect("note present");
    assert!(!risk.required);
    assert_eq!(risk.ok, Some(false));
}

#[test]
fn drag_that_never_started_is_ignored() {
    let ev = Evidence::judge(drag_checks(false, false, "slots", false));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::Ignored);
}

#[test]
fn drag_snap_back_denies() {
    let ev = Evidence::judge(drag_checks(true, false, "slots", true));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NoEffect);
    assert_eq!(ev.checks[1].after.as_ref().unwrap()["snap_back"], true);
}

#[test]
fn vendor_short_quantity_denies() {
    let ev = Evidence::judge(vendor_checks(true, true, 3, 5));
    assert_eq!(ev.verdict, EvVerdict::Denied);
    let denying = ev
        .checks
        .iter()
        .find(|c| c.ok == Some(false))
        .expect("a denying check");
    assert_eq!(denying.name, "quantity_matched");
}

#[test]
fn vendor_negative_check() {
    let ev = Evidence::judge(vendor_checks(false, false, 0, 5));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
}

#[test]
fn map_closed_confirms_with_a_pending_server_note() {
    let ev = Evidence::judge(map_checks(true));
    assert_eq!(ev.verdict, EvVerdict::Confirmed);
    let server = ev
        .checks
        .iter()
        .find(|c| c.name == "destination_sent")
        .expect("note present");
    assert_eq!(server.ok, None);
    assert_eq!(server.why, Some(Why::ServerUnreachable));
}

#[test]
fn loot_facts_are_read_only_for_loot_actions() {
    let out = json!({ "loot_count_before": 3, "loot_count_after": 2, "loot_window_open": true });
    let as_loot = item_evidence(&out, true, true);
    assert_eq!(as_loot.verdict, EvVerdict::Confirmed);
    // Not a loot row: the counters are ignored, and no diff says a change.
    let as_use = item_evidence(&out, true, false);
    assert_eq!(as_use.verdict, EvVerdict::Denied);
}
