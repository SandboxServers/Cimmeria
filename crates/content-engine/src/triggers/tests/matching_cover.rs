//! `Trigger::matches()` coverage for the cover-system triggers:
//! `OnPlayerEnteredCover`, `OnPlayerLeftCover`, `OnPlayerInCoverDuration`,
//! and `OnNpcFlanked`.

use super::super::*;
use super::make_event;

#[test]
fn player_entered_cover_filters_by_set_id() {
    let trigger = Trigger::OnPlayerEnteredCover {
        cover_set_id: Some(42),
    };
    let matching = make_event(
        TriggerType::PlayerEnteredCover,
        vec![("cover_set_id", serde_json::json!(42))],
    );
    assert!(trigger.matches(&matching));
    let other_set = make_event(
        TriggerType::PlayerEnteredCover,
        vec![("cover_set_id", serde_json::json!(43))],
    );
    assert!(!trigger.matches(&other_set));
}

#[test]
fn player_entered_cover_wildcard_matches_any_set() {
    let trigger = Trigger::OnPlayerEnteredCover { cover_set_id: None };
    let event = make_event(
        TriggerType::PlayerEnteredCover,
        vec![("cover_set_id", serde_json::json!(999))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn player_left_cover_filters_by_set_id() {
    let trigger = Trigger::OnPlayerLeftCover {
        cover_set_id: Some(7),
    };
    let matching = make_event(
        TriggerType::PlayerLeftCover,
        vec![("cover_set_id", serde_json::json!(7))],
    );
    assert!(trigger.matches(&matching));
    let other = make_event(
        TriggerType::PlayerLeftCover,
        vec![("cover_set_id", serde_json::json!(8))],
    );
    assert!(!trigger.matches(&other));
}

#[test]
fn player_left_cover_wildcard_matches_any_set() {
    let trigger = Trigger::OnPlayerLeftCover { cover_set_id: None };
    let event = make_event(
        TriggerType::PlayerLeftCover,
        vec![("cover_set_id", serde_json::json!(123))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn player_in_cover_duration_requires_seconds_match() {
    let trigger = Trigger::OnPlayerInCoverDuration {
        cover_set_id: None,
        seconds: 5,
    };
    let three_s = make_event(
        TriggerType::PlayerInCoverDuration,
        vec![("seconds", serde_json::json!(3))],
    );
    assert!(!trigger.matches(&three_s));
    let five_s = make_event(
        TriggerType::PlayerInCoverDuration,
        vec![("seconds", serde_json::json!(5))],
    );
    assert!(trigger.matches(&five_s));
}

#[test]
fn player_in_cover_duration_filters_by_set_id_too() {
    let trigger = Trigger::OnPlayerInCoverDuration {
        cover_set_id: Some(42),
        seconds: 5,
    };
    // Correct seconds, wrong set → no match.
    let wrong_set = make_event(
        TriggerType::PlayerInCoverDuration,
        vec![
            ("seconds", serde_json::json!(5)),
            ("cover_set_id", serde_json::json!(43)),
        ],
    );
    assert!(!trigger.matches(&wrong_set));
    // Correct set + correct seconds → match.
    let correct = make_event(
        TriggerType::PlayerInCoverDuration,
        vec![
            ("seconds", serde_json::json!(5)),
            ("cover_set_id", serde_json::json!(42)),
        ],
    );
    assert!(trigger.matches(&correct));
}

#[test]
fn npc_flanked_filters_by_template() {
    let trigger = Trigger::OnNpcFlanked {
        npc_template: Some("HumanGuard".to_string()),
    };
    let matching = make_event(
        TriggerType::NpcFlanked,
        vec![("npc_template", serde_json::json!("HumanGuard"))],
    );
    assert!(trigger.matches(&matching));
    let other = make_event(
        TriggerType::NpcFlanked,
        vec![("npc_template", serde_json::json!("GoauldGuard"))],
    );
    assert!(!trigger.matches(&other));
}

#[test]
fn npc_flanked_wildcard_matches_any() {
    let trigger = Trigger::OnNpcFlanked { npc_template: None };
    let event = make_event(
        TriggerType::NpcFlanked,
        vec![("npc_template", serde_json::json!("AnyGuard"))],
    );
    assert!(trigger.matches(&event));
}

// ─── OnPlayerInCoverDuration: wildcard seconds-only mismatch ──

#[test]
fn player_in_cover_duration_wildcard_set_matches_on_seconds() {
    let trigger = Trigger::OnPlayerInCoverDuration {
        cover_set_id: None,
        seconds: 5,
    };
    // Wildcard set + correct seconds → match even with a set id present.
    let event = make_event(
        TriggerType::PlayerInCoverDuration,
        vec![
            ("seconds", serde_json::json!(5)),
            ("cover_set_id", serde_json::json!(99)),
        ],
    );
    assert!(trigger.matches(&event));
}
