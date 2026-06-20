//! DB-row → `Action` conversions: destination parsing, move-waypoint,
//! set-active-slot, launch-ability, cross-world-teleport, and the
//! Phase 4/6/7 loader arms (set-npc-poi, set-follow-target,
//! set-npc-ai-state).

use super::super::action::{convert_action, parse_destination};
use super::super::*;

#[test]
fn parse_destination_valid() {
    assert_eq!(
        parse_destination("-123.625,1.311,-246.858"),
        [-123.625, 1.311, -246.858]
    );
}

#[test]
fn parse_destination_invalid() {
    assert_eq!(parse_destination("bad"), [0.0, 0.0, 0.0]);
}

#[test]
fn convert_move_waypoint_action() {
    let row = DbActionRow {
        chain_id: 1,
        action_type: "move_waypoint".to_string(),
        target_id: None,
        target_key: Some("NID_Guard_01".to_string()),
        params: serde_json::json!({"destination": "-296.715,68.511,-166.125", "speed": 1.5}),
        delay_ms: 0,
        sort_order: 0,
    };
    let action = convert_action(&row).unwrap();
    match action {
        Action::MoveWaypoint {
            entity_tag,
            destination,
            speed,
        } => {
            assert_eq!(entity_tag, "NID_Guard_01");
            assert_eq!(destination, [-296.715, 68.511, -166.125]);
            assert!((speed - 1.5).abs() < f32::EPSILON);
        }
        other => panic!("Expected MoveWaypoint, got {:?}", other),
    }
}

#[test]
fn convert_set_active_slot_action() {
    let row = DbActionRow {
        chain_id: 1,
        action_type: "set_active_slot".to_string(),
        target_id: None,
        target_key: None,
        params: serde_json::json!({"bag_id": 3, "slot": 0}),
        delay_ms: 0,
        sort_order: 0,
    };
    let action = convert_action(&row).unwrap();
    match action {
        Action::SetActiveSlot { bag_id, slot } => {
            assert_eq!(bag_id, 3);
            assert_eq!(slot, 0);
        }
        other => panic!("Expected SetActiveSlot, got {:?}", other),
    }
}

#[test]
fn convert_set_active_slot_defaults_bandolier() {
    let row = DbActionRow {
        chain_id: 1,
        action_type: "set_active_slot".to_string(),
        target_id: None,
        target_key: None,
        params: serde_json::json!({"slot": 2}),
        delay_ms: 0,
        sort_order: 0,
    };
    let action = convert_action(&row).unwrap();
    match action {
        Action::SetActiveSlot { bag_id, slot } => {
            assert_eq!(bag_id, 3); // defaults to Bandolier
            assert_eq!(slot, 2);
        }
        other => panic!("Expected SetActiveSlot, got {:?}", other),
    }
}

#[test]
fn convert_launch_ability_action() {
    let row = DbActionRow {
        chain_id: 1,
        action_type: "launch_ability".to_string(),
        target_id: Some(1372),
        target_key: Some("NID_Guard_01".to_string()),
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    };
    let action = convert_action(&row).unwrap();
    match action {
        Action::LaunchAbility {
            ability_id,
            entity_tag,
        } => {
            assert_eq!(ability_id, 1372);
            assert_eq!(entity_tag, Some("NID_Guard_01".to_string()));
        }
        other => panic!("Expected LaunchAbility, got {:?}", other),
    }
}

/// `cross_world_teleport` with valid x/y/z floats round-trips into
/// `Action::CrossWorldTeleport` carrying the world name and position
/// verbatim. Pins the canonical happy-path shape.
#[test]
fn convert_cross_world_teleport_action() {
    let row = DbActionRow {
        chain_id: 1109,
        action_type: "cross_world_teleport".to_string(),
        target_id: None,
        target_key: Some("Castle".to_string()),
        params: serde_json::json!({"x": 466.365, "y": 70.397, "z": 991.466}),
        delay_ms: 0,
        sort_order: 0,
    };
    match convert_action(&row).unwrap() {
        Action::CrossWorldTeleport {
            world_name,
            position,
        } => {
            assert_eq!(world_name, "Castle");
            assert!((position[0] - 466.365).abs() < 0.001);
            assert!((position[1] - 70.397).abs() < 0.001);
            assert!((position[2] - 991.466).abs() < 0.001);
        }
        other => panic!("Expected CrossWorldTeleport, got {:?}", other),
    }
}

/// Missing x/y/z params on a `cross_world_teleport` row must drop the
/// action entirely (returns None). Silent fallback to (0,0,0) would
/// teleport the player inside-the-floor or out of map bounds — a worse
/// failure mode than dropping the action and logging at the loader.
#[test]
fn convert_cross_world_teleport_missing_coords_returns_none() {
    let row = DbActionRow {
        chain_id: 1109,
        action_type: "cross_world_teleport".to_string(),
        target_id: None,
        target_key: Some("Castle".to_string()),
        params: serde_json::json!({"x": 466.365, "z": 991.466}), // y missing
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(
        convert_action(&row).is_none(),
        "missing y coord must reject the row, not fall back to 0.0",
    );
}

/// Non-finite coords (NaN / Infinity) must drop the action. A NaN cast
/// via `as f32` produces NaN and the BigWorld client's position handling
/// against a NaN destination is undefined — better to fail closed at the
/// loader.
#[test]
fn convert_cross_world_teleport_non_finite_coords_returns_none() {
    let row = DbActionRow {
        chain_id: 1109,
        action_type: "cross_world_teleport".to_string(),
        target_id: None,
        target_key: Some("Castle".to_string()),
        params: serde_json::json!({"x": "NaN", "y": 70.397, "z": 991.466}),
        delay_ms: 0,
        sort_order: 0,
    };
    // serde_json's "NaN" as a string won't deserialize as f64 — covers
    // the wrong-type rejection path. The is_finite check inside
    // convert_action covers the explicit f64::NAN / f64::INFINITY case
    // when the value is numerically constructible.
    assert!(convert_action(&row).is_none());
}

/// Missing `target_key` (the world name) on a `cross_world_teleport`
/// row must drop the action. Without a target world there's nowhere
/// to teleport to.
#[test]
fn convert_cross_world_teleport_missing_world_returns_none() {
    let row = DbActionRow {
        chain_id: 1109,
        action_type: "cross_world_teleport".to_string(),
        target_id: None,
        target_key: None,
        params: serde_json::json!({"x": 1.0, "y": 2.0, "z": 3.0}),
        delay_ms: 0,
        sort_order: 0,
    };
    assert!(convert_action(&row).is_none());
}

// ── Phase 4 / 6 / 7 loader arms ──────────────────────────────────────

fn make_row(action_type: &str, target_key: Option<&str>, params: serde_json::Value) -> DbActionRow {
    DbActionRow {
        chain_id: 1,
        action_type: action_type.to_string(),
        target_id: None,
        target_key: target_key.map(|s| s.to_string()),
        params,
        delay_ms: 0,
        sort_order: 0,
    }
}

/// `set_npc_poi` parses target_key + x/y/z params into the action
/// variant. Pin the param-key names so a typo at the loader site
/// can't silently drop the action while the executor tests still pass.
#[test]
fn set_npc_poi_parses_target_key_and_coords() {
    use crate::actions::Action;
    let row = make_row(
        "set_npc_poi",
        Some("Guard"),
        serde_json::json!({ "x": 10.5, "y": 0.0, "z": -7.25 }),
    );
    match convert_action(&row).expect("parse must succeed") {
        Action::SetNpcPoi {
            entity_tag,
            x,
            y,
            z,
        } => {
            assert_eq!(entity_tag, "Guard");
            assert!((x - 10.5).abs() < 1e-4);
            assert!((y - 0.0).abs() < 1e-4);
            assert!((z + 7.25).abs() < 1e-4);
        }
        other => panic!("expected SetNpcPoi, got {other:?}"),
    }
}

/// `set_npc_poi` drops the action when any of x/y/z is missing rather
/// than silently routing the NPC to (0, 0, 0).
#[test]
fn set_npc_poi_drops_action_when_coord_missing() {
    let row = make_row(
        "set_npc_poi",
        Some("Guard"),
        serde_json::json!({ "x": 1.0, "y": 2.0 }), // missing z
    );
    assert!(
        convert_action(&row).is_none(),
        "missing coord must drop the action",
    );
}

/// `set_npc_poi` drops the action when a coord is non-finite (NaN /
/// inf). Same shape as `cross_world_teleport`.
#[test]
fn set_npc_poi_drops_action_on_nan_coord() {
    let row = make_row(
        "set_npc_poi",
        Some("Guard"),
        serde_json::json!({ "x": 1.0, "y": f64::NAN, "z": 0.0 }),
    );
    assert!(
        convert_action(&row).is_none(),
        "NaN coord must drop the action",
    );
}

/// `set_follow_target` with a `target_tag` param parses it into
/// `Some(tag)`.
#[test]
fn set_follow_target_with_string_target_tag_resolves_to_some() {
    use crate::actions::Action;
    let row = make_row(
        "set_follow_target",
        Some("Pet"),
        serde_json::json!({ "target_tag": "Owner" }),
    );
    match convert_action(&row).expect("parse must succeed") {
        Action::SetFollowTarget {
            entity_tag,
            target_tag,
        } => {
            assert_eq!(entity_tag, "Pet");
            assert_eq!(target_tag.as_deref(), Some("Owner"));
        }
        other => panic!("expected SetFollowTarget, got {other:?}"),
    }
}

/// `set_follow_target` with no `target_tag` param OR an empty-string
/// value resolves to `None` (the "clear follow" semantic).
#[test]
fn set_follow_target_with_no_or_empty_target_tag_resolves_to_none() {
    use crate::actions::Action;
    for params in [
        serde_json::json!({}),
        serde_json::json!({ "target_tag": "" }),
    ] {
        let row = make_row("set_follow_target", Some("Pet"), params);
        match convert_action(&row).expect("parse must succeed") {
            Action::SetFollowTarget {
                target_tag: None, ..
            } => {}
            other => panic!("expected target_tag = None, got {other:?}"),
        }
    }
}

/// `set_npc_ai_state` parses lowercase state strings.
#[test]
fn set_npc_ai_state_parses_lowercase_strings() {
    use crate::actions::{Action, NpcAiStateAction};
    let cases = [
        ("idle", NpcAiStateAction::Idle),
        ("despawning", NpcAiStateAction::Despawning),
        ("submit", NpcAiStateAction::Submit),
        ("error", NpcAiStateAction::Error),
    ];
    for (s, expected) in cases {
        let row = make_row(
            "set_npc_ai_state",
            Some("Boss"),
            serde_json::json!({ "state": s }),
        );
        match convert_action(&row).expect("parse must succeed") {
            Action::SetNpcAiState { state, .. } => {
                assert!(
                    matches!(state, _ if std::mem::discriminant(&state) == std::mem::discriminant(&expected)),
                    "state mismatch for {s:?}: expected {expected:?}, got {state:?}",
                );
            }
            other => panic!("expected SetNpcAiState, got {other:?}"),
        }
    }
}

/// `set_npc_ai_state` is case-insensitive — content authors can write
/// "Idle" / "DESPAWNING" / "Error" without dropping the action.
#[test]
fn set_npc_ai_state_is_case_insensitive() {
    use crate::actions::{Action, NpcAiStateAction};
    for s in ["Idle", "IDLE", "Despawning", "ERROR", "sUbMiT"] {
        let row = make_row(
            "set_npc_ai_state",
            Some("Boss"),
            serde_json::json!({ "state": s }),
        );
        let action = convert_action(&row)
            .unwrap_or_else(|| panic!("case-insensitive parse must succeed for {s:?}"));
        let Action::SetNpcAiState { state, .. } = action else {
            panic!("expected SetNpcAiState");
        };
        // Confirm the state variant is one of the admitted four.
        assert!(matches!(
            state,
            NpcAiStateAction::Idle
                | NpcAiStateAction::Despawning
                | NpcAiStateAction::Submit
                | NpcAiStateAction::Error,
        ));
    }
}

/// `set_npc_ai_state` drops the action when the state string is
/// unknown rather than silently picking a default. Pin: a typo
/// shouldn't flip an NPC to Idle.
#[test]
fn set_npc_ai_state_drops_action_on_unknown_state() {
    let row = make_row(
        "set_npc_ai_state",
        Some("Boss"),
        serde_json::json!({ "state": "totally_not_a_state" }),
    );
    assert!(
        convert_action(&row).is_none(),
        "unknown state must drop the action",
    );
}
