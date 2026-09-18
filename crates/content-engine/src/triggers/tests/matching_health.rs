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

/// The readable spec case: a hit that takes the duel NPC from 60% to
/// 40% crosses the 50% threshold and must fire.
///
/// Note this is **not** a regression guard for either half of the band
/// predicate — 60→40 against `:50` still matches if you delete either
/// `before > threshold` or `after <= threshold`. The two halves are
/// guarded by [`does_not_fire_on_a_second_hit_already_below`] and
/// [`does_not_fire_when_the_hit_stays_above`] respectively; this test is
/// here so a reader meets the happy path first.
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

// A "healed back above, crossed again, fires again" case does NOT belong
// here. `matches` is a pure function of (before, after, threshold), so
// calling it twice with two crossing tuples asserts nothing the single
// crossing case above doesn't already. The "not a latch" claim only has
// content where state could accumulate — the dispatcher — and it is
// pinned there against a real `SpaceManager` by
// `a_second_genuine_crossing_after_a_heal_fires_again` in
// `cimmeria-services`.

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
///
/// **This is the guard for the `after <= threshold` half of the band.**
/// Delete that half and `90 > 30` alone matches, firing a duel's submit
/// chain on the opening shot.
#[test]
fn does_not_fire_when_the_hit_stays_above() {
    assert!(!trigger("Boss", 30).matches(&health_event("Boss", 90.0, 55.0)));
}

/// Two thresholds on the same tag are independent: a 60→40 hit crosses
/// 50 but not 30. This is what lets an author stage a fight without the
/// firing site knowing which thresholds exist.
///
/// The second assertion is the other guard for the `after <= threshold`
/// half — without it, the `:30` chain fires on a hit that never reached
/// 30%.
#[test]
fn distinct_thresholds_on_the_same_tag_fire_independently() {
    let ev = health_event("Rinla_Malac", 60.0, 40.0);
    assert!(trigger("Rinla_Malac", 50).matches(&ev));
    assert!(!trigger("Rinla_Malac", 30).matches(&ev));
}

// The "one big hit spans both thresholds" case is covered at the
// dispatcher level (`one_hit_spanning_two_thresholds_fires_both_chains`)
// where two chains are actually registered and resolved. Restating it
// here as two positive `matches` calls would add no signal over the
// single-crossing case plus the independence test above.

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

// No "wrong discriminant rejects" case here: that short-circuit
// (`if self.trigger_type() != event.trigger_type { return false }`) sits
// at the top of `matches` and is variant-independent, already pinned by
// `wrong_trigger_type_never_matches` in `matching_entity.rs`. Restating
// a shared invariant once per variant is how a matcher test file grows
// thirty tests that all fail together.
