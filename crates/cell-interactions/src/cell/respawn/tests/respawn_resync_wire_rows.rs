//! AB-C7 full accounting: the respawn resync's ability methods each write
//! one `abilities.wire` row with `origin = respawn_resync`, and the hotbar
//! list sent from any trigger names that trigger.
//!
//! Reverting the resync loop to the plain `send_entity_method`, or the
//! known-abilities send to a raw `EntityMethodCall`, drops the rows and
//! fails these tests.

use super::super::resync::{
    resync_after_pawn_recreate, send_known_abilities_update, ORIGIN_RESPAWN_RESYNC,
};
use super::make_mgr_with_player;
use crate::test_support::{LogCapture, LogCaptureGuard};
use tokio::sync::mpsc;

fn wire_rows(capture: &LogCaptureGuard, method: &str) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| {
            c.target == "abilities.wire"
                && c.has_field("event", "wire_sent")
                && c.has_field("method", method)
        })
        .collect()
}

#[tokio::test]
async fn the_resync_writes_one_row_per_ability_method_with_its_trigger() {
    let capture = LogCapture::install();
    let mut mgr = make_mgr_with_player("Agnos");
    mgr.get_entity_mut(1).unwrap().abilities.add_ability(597);
    let (tx, mut rx) = mpsc::channel(64);
    resync_after_pawn_recreate(1, &tx, &mgr).await;
    while rx.try_recv().is_ok() {}

    for method in [
        "onStateFieldUpdate",
        "onStatUpdate",
        "onStatBaseUpdate",
        "onAbilityTreeInfo",
        "onKnownAbilitiesUpdate",
    ] {
        let rows = wire_rows(&capture, method);
        assert_eq!(rows.len(), 1, "{method}: {:#?}", capture.all());
        assert!(
            rows[0].has_field("origin", ORIGIN_RESPAWN_RESYNC),
            "{method}: {:#?}",
            rows[0]
        );
        assert!(rows[0].has_field("player_id", "100"), "{:#?}", rows[0]);
    }
    let known = &wire_rows(&capture, "onKnownAbilitiesUpdate")[0];
    assert!(known.has_field("ability_ids", "597"), "{known:#?}");
    assert!(known.has_field("route", "self"), "{known:#?}");
    // onLevelUpdate is not an ability method: no ledger row.
    assert!(wire_rows(&capture, "other").is_empty());
}

#[tokio::test]
async fn the_hotbar_list_names_its_trigger() {
    let capture = LogCapture::install();
    let mgr = make_mgr_with_player("Agnos");
    let (tx, mut rx) = mpsc::channel(8);
    send_known_abilities_update(1, "ability_granted", &tx, &mgr).await;
    assert!(rx.try_recv().is_ok(), "the method still goes to the client");
    let rows = wire_rows(&capture, "onKnownAbilitiesUpdate");
    assert_eq!(rows.len(), 1, "{:#?}", capture.all());
    assert!(rows[0].has_field("origin", "ability_granted"));
}
