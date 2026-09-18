//! `Trigger::matches()` coverage for `OnEntityHealthBelow` (Harset H04).
//!
//! The whole trigger is a downward-crossing predicate, so these tests are
//! the primary specification of "once per crossing". Each one names the
//! gameplay shape it pins so a future refactor of `matches` can tell
//! which behaviour it broke.

use super::super::*;
use super::make_event;

/// Build the event the damage path emits: the damaged entity's tag plus
/// its health percentage before and after a single hit.
fn health_event(tag: &str, before: f64, after: f64) -> TriggerEvent {
    make_event(
        TriggerType::EntityHealthBelow,
        vec![
            ("entity_tag", serde_json::json!(tag)),
            ("pct_before", serde_json::json!(before)),
            ("pct_after", serde_json::json!(after)),
        ],
    )
}

fn trigger(tag: &str, pct: i32) -> Trigger {
    Trigger::OnEntityHealthBelow {
        entity_tag: tag.to_string(),
        pct,
    }
}

/// The headline case: a hit that takes the duel NPC from 60% to 40%
/// crosses the 50% threshold and must fire.
#[test]
fn fires_on_the_hit_that_crosses_downward() {
    assert!(trigger("Rinla_Malac", 50).matches(&health_event("Rinla_Malac", 60.0, 40.0)));
}

/// The second hit, already below the threshold, must NOT fire. This is
/// the "once per crossing" half of the contract — without the
/// `pct_before > pct` half of the predicate, every subsequent hit would
/// re-fire the chain and re-advance the mission step.
#[test]
fn does_not_fire_on_a_second_hit_already_below() {
    assert!(!trigger("Rinla_Malac", 50).matches(&health_event("Rinla_Malac", 40.0, 25.0)));
}

/// Healed back above the threshold and crossed again → fires again. The
/// crossing is a property of the hit, not a latched per-entity flag.
#[test]
fn fires_again_after_healing_back_above_and_recrossing() {
    let t = trigger("Rinla_Malac", 50);
    assert!(t.matches(&health_event("Rinla_Malac", 60.0, 40.0)));
    // (heal happens; the next damaging hit starts from 70%)
    assert!(t.matches(&health_event("Rinla_Malac", 70.0, 45.0)));
}

/// Landing exactly ON the threshold counts as crossed — "at or below".
#[test]
fn landing_exactly_on_the_threshold_fires() {
    assert!(trigger("Boss", 30).matches(&health_event("Boss", 55.0, 30.0)));
}

/// Starting exactly ON the threshold is already below, so the next hit
/// does not re-fire. Pairs with the test above: 55→30 fires, 30→10 does
/// not, and between them the threshold is crossed exactly once.
#[test]
fn starting_exactly_on_the_threshold_does_not_refire() {
    assert!(!trigger("Boss", 30).matches(&health_event("Boss", 30.0, 10.0)));
}

/// A hit that stays entirely above the threshold does nothing.
#[test]
fn does_not_fire_when_the_hit_stays_above() {
    assert!(!trigger("Boss", 30).matches(&health_event("Boss", 90.0, 55.0)));
}

/// Two thresholds on the same tag are independent: a 60→40 hit crosses
/// 50 but not 30. This is what lets an author stage a fight without the
/// firing site knowing which thresholds exist.
#[test]
fn distinct_thresholds_on_the_same_tag_fire_independently() {
    let ev = health_event("Rinla_Malac", 60.0, 40.0);
    assert!(trigger("Rinla_Malac", 50).matches(&ev));
    assert!(!trigger("Rinla_Malac", 30).matches(&ev));
}

/// One big hit that spans both thresholds fires both chains.
#[test]
fn a_single_hit_spanning_two_thresholds_fires_both() {
    let ev = health_event("Rinla_Malac", 80.0, 20.0);
    assert!(trigger("Rinla_Malac", 50).matches(&ev));
    assert!(trigger("Rinla_Malac", 30).matches(&ev));
}

/// Tag filter: a crossing on a different NPC must not fire this chain.
#[test]
fn rejects_a_crossing_on_a_different_tag() {
    assert!(!trigger("Rinla_Malac", 50).matches(&health_event("Storage_Petbe", 60.0, 40.0)));
}

/// Missing pct params must reject, not match. A `unwrap_or` default that
/// happened to satisfy the predicate would fire every seeded threshold
/// on every event that reached this arm.
#[test]
fn rejects_when_the_percentage_params_are_absent() {
    let ev = make_event(
        TriggerType::EntityHealthBelow,
        vec![("entity_tag", serde_json::json!("Rinla_Malac"))],
    );
    assert!(!trigger("Rinla_Malac", 50).matches(&ev));
}

/// Wrong discriminant short-circuits before the params are read.
#[test]
fn rejects_an_event_of_a_different_trigger_type() {
    let ev = make_event(
        TriggerType::EntityDeath,
        vec![
            ("entity_tag", serde_json::json!("Rinla_Malac")),
            ("pct_before", serde_json::json!(60.0)),
            ("pct_after", serde_json::json!(40.0)),
        ],
    );
    assert!(!trigger("Rinla_Malac", 50).matches(&ev));
}
