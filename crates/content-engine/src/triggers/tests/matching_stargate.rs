//! `Trigger::matches()` and `convert_trigger` coverage for the two
//! stargate triggers (CA10).
//!
//! The chain-replay guards in `cimmeria-services` exercise the same two
//! arms, but only against a live database — they self-skip without one,
//! so a `DATABASE_URL`-less run (a contributor's first `cargo test`, and
//! CI's non-live-DB job) has no signal at all on this vocabulary. These
//! are the no-DB half: pure `Trigger` / `DbTriggerRow` values, no pool.
//!
//! Three claims per trigger: the keyed form matches its own world, the
//! keyed form rejects another world, and the NULL `event_key` wildcard
//! matches anything. Plus the cross-trigger negative, which is what
//! stops mission 708's "step through the gate" objective completing the
//! moment the player touches the DHD.
//!
//! The `event_type` → `Trigger` half lives in
//! `loader::tests::trigger_conversion` — `convert_trigger` is private to
//! the `loader` module.

use super::super::*;
use super::make_event;

// ─── matching ───────────────────────────────────────────────────────

#[test]
fn stargate_dialed_matches_its_own_destination_world() {
    let trigger = Trigger::OnStargateDialed {
        destination_world: Some("Harset".to_string()),
    };
    let event = make_event(
        TriggerType::StargateDialed,
        vec![("destination_world", serde_json::json!("Harset"))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn stargate_dialed_rejects_another_destination_world() {
    let trigger = Trigger::OnStargateDialed {
        destination_world: Some("Harset".to_string()),
    };
    let event = make_event(
        TriggerType::StargateDialed,
        vec![("destination_world", serde_json::json!("Castle"))],
    );
    assert!(
        !trigger.matches(&event),
        "a world-keyed gate chain must not fire on every gate in the zone"
    );
}

#[test]
fn stargate_dialed_wildcard_matches_any_destination() {
    let trigger = Trigger::OnStargateDialed {
        destination_world: None,
    };
    for world in ["Harset", "Castle", "SGC"] {
        let event = make_event(
            TriggerType::StargateDialed,
            vec![("destination_world", serde_json::json!(world))],
        );
        assert!(trigger.matches(&event), "wildcard must match {world}");
    }
}

#[test]
fn stargate_crossed_matches_its_own_destination_world() {
    let trigger = Trigger::OnStargateCrossed {
        destination_world: Some("Harset".to_string()),
    };
    let event = make_event(
        TriggerType::StargateCrossed,
        vec![("destination_world", serde_json::json!("Harset"))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn stargate_crossed_rejects_another_destination_world() {
    let trigger = Trigger::OnStargateCrossed {
        destination_world: Some("Harset".to_string()),
    };
    let event = make_event(
        TriggerType::StargateCrossed,
        vec![("destination_world", serde_json::json!("Castle"))],
    );
    assert!(!trigger.matches(&event));
}

#[test]
fn stargate_crossed_wildcard_matches_any_destination() {
    let trigger = Trigger::OnStargateCrossed {
        destination_world: None,
    };
    let event = make_event(
        TriggerType::StargateCrossed,
        vec![("destination_world", serde_json::json!("Anywhere"))],
    );
    assert!(trigger.matches(&event));
}

/// Dialling is not crossing. The two share a `matches` arm, so only the
/// distinct `TriggerType` discriminants keep them apart — a refactor
/// that collapsed them would complete a "step through the gate"
/// objective as soon as the player touched the DHD.
#[test]
fn stargate_triggers_do_not_answer_each_others_events() {
    let dialed = Trigger::OnStargateDialed {
        destination_world: None,
    };
    let crossed = Trigger::OnStargateCrossed {
        destination_world: None,
    };
    assert_eq!(dialed.trigger_type(), TriggerType::StargateDialed);
    assert_eq!(crossed.trigger_type(), TriggerType::StargateCrossed);
    assert_ne!(dialed.trigger_type(), crossed.trigger_type());
}

/// An event with no `destination_world` param at all must not match a
/// keyed trigger. Guards the `is_some_and` in the match arm — a
/// `map_or(true, ...)` slip would turn a missing param into a match.
#[test]
fn keyed_stargate_trigger_rejects_an_event_missing_the_param() {
    let trigger = Trigger::OnStargateDialed {
        destination_world: Some("Harset".to_string()),
    };
    let event = make_event(TriggerType::StargateDialed, vec![]);
    assert!(!trigger.matches(&event));
}
