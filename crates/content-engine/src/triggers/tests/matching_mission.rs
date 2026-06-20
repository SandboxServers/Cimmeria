//! `Trigger::matches()` coverage for the mission triggers:
//! `OnMissionStep`, `OnMissionAccepted`, and `OnMissionCompleted`.

use super::super::*;
use super::make_event;

#[test]
fn mission_step_requires_both_fields() {
    let trigger = Trigger::OnMissionStep {
        mission_id: 10,
        step: 3,
    };

    let event = make_event(
        TriggerType::MissionStep,
        vec![
            ("mission_id", serde_json::json!(10)),
            ("step", serde_json::json!(3)),
        ],
    );
    assert!(trigger.matches(&event));

    let event = make_event(
        TriggerType::MissionStep,
        vec![
            ("mission_id", serde_json::json!(10)),
            ("step", serde_json::json!(1)),
        ],
    );
    assert!(!trigger.matches(&event));
}

#[test]
fn mission_completed_matches() {
    let trigger = Trigger::OnMissionCompleted { mission_id: 1559 };
    let event = make_event(
        TriggerType::MissionCompleted,
        vec![("mission_id", serde_json::json!(1559))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn mission_accepted_matches_correct_mission_id() {
    let trigger = Trigger::OnMissionAccepted { mission_id: 687 };
    let event = make_event(
        TriggerType::MissionAccepted,
        vec![("mission_id", serde_json::json!(687))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn mission_accepted_rejects_wrong_mission_id() {
    let trigger = Trigger::OnMissionAccepted { mission_id: 687 };
    let event = make_event(
        TriggerType::MissionAccepted,
        vec![("mission_id", serde_json::json!(641))],
    );
    assert!(!trigger.matches(&event));
}

// ─── OnMissionStep: step matches but mission id mismatch ──────

#[test]
fn mission_step_rejects_wrong_mission() {
    let trigger = Trigger::OnMissionStep {
        mission_id: 10,
        step: 3,
    };
    let event = make_event(
        TriggerType::MissionStep,
        vec![
            ("mission_id", serde_json::json!(11)),
            ("step", serde_json::json!(3)),
        ],
    );
    assert!(!trigger.matches(&event));
}

// ─── OnMissionCompleted: reject path ──────────────────────────

#[test]
fn mission_completed_rejects_wrong_id() {
    let trigger = Trigger::OnMissionCompleted { mission_id: 1559 };
    let event = make_event(
        TriggerType::MissionCompleted,
        vec![("mission_id", serde_json::json!(1560))],
    );
    assert!(!trigger.matches(&event));
}
