//! AB-T5: the `abilities.snapshot` row and the snapshot's JSON form.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::{AbilityStateSnapshot, TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::ACCURACY;
use tracing::Level;

use super::{log_ability_snapshot, SnapshotTrigger};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const PLAYER: u32 = 1;

fn world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(PLAYER, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(PLAYER).unwrap();
    e.is_player = true;
    e.account_id = Some(601);
    e.player_id = Some(71);
    e.abilities
        .start_ability_cooldown(592, Duration::from_secs(30));
    e.apply_timed_effect(
        TimedEffectSpec {
            cast_id: Some(41),
            effect_id: 700,
            ability_id: 637,
            invoker_id: PLAYER,
            effect_flags: 21,
            moniker_ids: vec![],
            stats: vec![(ACCURACY, 200)],
            absorb: Vec::new(),
            state_flags: 0,
            duration_secs: Some(15.0),
            stacking: TimedStacking::PerSource,
            invoker_identity: Default::default(),
        },
        Instant::now(),
    )
    .expect("buff applies");
    mgr
}

/// The snapshot crosses a JSON boundary (this row, the lab tool): it must
/// come back equal.
#[test]
fn ab_t5_snapshot_round_trips_through_json() {
    let s = world().ability_state(PLAYER).expect("player exists");
    let json = serde_json::to_string(&s).expect("serialise");
    let back: AbilityStateSnapshot = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, s);
}

#[test]
fn ab_t5_snapshot_row_carries_identity_counts_and_the_json() {
    let mgr = world();
    let logs = LogCapture::install();
    assert!(log_ability_snapshot(
        &mgr,
        PLAYER,
        SnapshotTrigger::Bookmark,
        Some(1234)
    ));

    let row = logs
        .find_message(Level::INFO, "ability state snapshot")
        .expect("one INFO row");
    assert_eq!(row.target, "abilities.snapshot");
    assert!(row.has_field("event", "ability_snapshot"));
    assert!(row.has_field("trigger", "bookmark"));
    assert!(row.has_field("bookmark_id", "1234"));
    assert!(row.has_field("account_id", "601"));
    assert!(row.has_field("player_id", "71"));
    assert!(row.has_field("cooldowns", "1"));
    assert!(row.has_field("ledger", "1"));
    let json = row.fields.get("snapshot").expect("snapshot field");
    let s: AbilityStateSnapshot = serde_json::from_str(json).expect("the field is the JSON");
    assert_eq!(s.entity_id, PLAYER);
    assert_eq!(s.ledger[0].effect_id, 700);
    assert_eq!(s.ledger[0].cast_id, Some(41));
    assert_eq!(s.cooldowns[0].ability_id, 592);
}

#[test]
fn ab_t5_snapshot_of_a_missing_entity_writes_no_info_row() {
    let mgr = world();
    let logs = LogCapture::install();
    assert!(!log_ability_snapshot(
        &mgr,
        999,
        SnapshotTrigger::Death,
        None
    ));
    assert!(logs
        .find_message(Level::INFO, "ability state snapshot")
        .is_none());
    let skipped = logs
        .find_message(Level::DEBUG, "ability snapshot not written")
        .expect("the skip is logged");
    assert!(skipped.has_field("reason", "entity_missing"));
}
