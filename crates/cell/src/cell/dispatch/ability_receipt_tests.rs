//! AB-C7: every client-to-server ability method writes a receipt row when
//! it goes through the full router (the "server recv row" column of the
//! telemetry coverage matrix).
//!
//! Removing an entry's row (the router's `log_receipt` call, or a
//! handler's own `use_ability_recv`) fails
//! `every_ability_method_writes_its_receipt_row`; moving the router's call
//! below the GM gate fails `a_refused_gm_debug_call_still_has_its_receipt`.

use tokio::sync::mpsc;
use tracing::Level;

use super::super::space_manager::SpaceManager;
use super::ability_receipt::{receipt, ABILITY_RECEIPTS, GENERIC_EVENT};
use super::*;
use crate::test_support::LogCapture;

const CALLER: u32 = 1;

fn world(access_level: u32) -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(CALLER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(CALLER).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    p.access_level = access_level;
    mgr
}

/// Well-formed arguments for each method, so a handler that checks the
/// length (and logs its own row) sees a valid call.
fn args_for(method: &str) -> Vec<u8> {
    let i = |v: i32| v.to_le_bytes().to_vec();
    match method {
        "useAbility" => [i(597), i(0)].concat(),
        "useAbilityOnGroundTarget" => [i(597), i(0), i(0), i(0)].concat(),
        "confirmationResponse" => [i(7), vec![1]].concat(),
        "trainAbility" | "gmDebugAbility" | "gmDebugAbilityOnMob" => i(597),
        "petInvokeAbility" => [i(9), i(597), i(0)].concat(),
        "petAbilityToggle" => [i(9), i(597), vec![1]].concat(),
        _ => Vec::new(),
    }
}

async fn route(mgr: &mut SpaceManager, index: u16, args: &[u8], seq: Option<u32>) {
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);
    dispatch_cell_method(CALLER, index, args, &tx, mgr, &engine, seq).await;
}

#[tokio::test]
async fn every_ability_method_writes_its_receipt_row() {
    for r in ABILITY_RECEIPTS {
        let capture = LogCapture::install();
        let mut mgr = world(2);
        route(&mut mgr, r.index, &args_for(r.method), Some(4242)).await;
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.level == Level::DEBUG && c.has_field("event", r.event))
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "{} ({}): expected exactly one `{}` row, got {:#?}",
            r.method,
            r.index,
            r.event,
            capture.all()
        );
        let row = &rows[0];
        assert_eq!(row.target, "abilities", "{}", r.method);
        assert!(row.has_field("stage", "recv"), "{}: {row:#?}", r.method);
        assert!(
            row.has_field("mercury_seq", "4242"),
            "{}: {row:#?}",
            r.method
        );
        assert!(row.has_field("player_id", "100"), "{}: {row:#?}", r.method);
        if r.event == GENERIC_EVENT {
            assert!(row.has_field("method", r.method), "{}: {row:#?}", r.method);
        }
    }
}

#[tokio::test]
async fn a_refused_gm_debug_call_still_has_its_receipt() {
    let capture = LogCapture::install();
    let mut mgr = world(0);
    route(&mut mgr, 170, &[], None).await;
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.has_field("event", GENERIC_EVENT) && c.has_field("method", "gmDebugCombat")),
        "{:#?}",
        capture.all()
    );
}

#[tokio::test]
async fn other_methods_write_no_receipt_row() {
    let capture = LogCapture::install();
    let mut mgr = world(0);
    // setTargetID (0) is not an ability method.
    route(&mut mgr, 0, &0i32.to_le_bytes(), None).await;
    assert!(receipt(0).is_none());
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", GENERIC_EVENT)),
        "{:#?}",
        capture.all()
    );
}

/// The table's names agree with the wire crate's names where it has them
/// (indices below 109; the GM tail has no name table).
#[test]
fn receipt_names_match_the_wire_names() {
    for r in ABILITY_RECEIPTS.iter().filter(|r| r.index < 109) {
        assert_eq!(cell_method_name(r.index), r.method, "index {}", r.index);
    }
    let mut seen = std::collections::HashSet::new();
    assert!(
        ABILITY_RECEIPTS.iter().all(|r| seen.insert(r.index)),
        "duplicate index"
    );
}
