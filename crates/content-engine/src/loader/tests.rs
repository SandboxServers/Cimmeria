//! Tests for chain loading and DB-row → typed-enum conversion.

use super::action::{convert_action, parse_destination};
use super::condition::{convert_condition, parse_step_status};
use super::trigger::convert_trigger;
use super::*;
use crate::conditions::{ComparisonOp, Condition, StepStatusValue};

#[test]
fn load_empty_array() {
    let chains = load_chains_from_json("[]").unwrap();
    assert!(chains.is_empty());
}

#[test]
fn load_single_chain() {
    let json = r#"[{
        "id": 1,
        "name": "Grant XP on mob kill",
        "enabled": true,
        "trigger": {
            "OnEntityDeath": { "entity_type": "SGWMob" }
        },
        "conditions": [],
        "actions": [
            { "GrantXP": { "amount": 100 } }
        ],
        "priority": 10
    }]"#;

    let chains = load_chains_from_json(json).unwrap();
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].id, 1);
    assert_eq!(chains[0].name, "Grant XP on mob kill");
}

#[test]
fn load_invalid_json_returns_error() {
    let result = load_chains_from_json("not valid json");
    assert!(result.is_err());
}

#[test]
fn build_chains_from_db_rows() {
    let chain_rows = vec![DbChainRow {
        chain_id: 1001,
        description: Some("622 - Zone load: accept mission".to_string()),
        scope_type: "mission".to_string(),
        scope_id: Some(622),
        enabled: true,
        priority: 0,
    }];
    let trigger_rows = vec![DbTriggerRow {
        chain_id: 1001,
        event_type: "player_loaded".to_string(),
        event_key: None,
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    }];
    let condition_rows = vec![DbConditionRow {
        chain_id: 1001,
        condition_type: "mission_status".to_string(),
        target_id: Some(622),
        target_key: None,
        operator: "eq".to_string(),
        value: Some("not_active".to_string()),
        sort_order: 0,
    }];
    let action_rows = vec![
        DbActionRow {
            chain_id: 1001,
            action_type: "accept_mission".to_string(),
            target_id: Some(622),
            target_key: None,
            params: serde_json::json!({}),
            delay_ms: 0,
            sort_order: 0,
        },
        DbActionRow {
            chain_id: 1001,
            action_type: "display_dialog".to_string(),
            target_id: Some(2982),
            target_key: None,
            params: serde_json::json!({}),
            delay_ms: 0,
            sort_order: 1,
        },
    ];

    let chains = build_chains_from_rows(chain_rows, trigger_rows, condition_rows, action_rows);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].id, 1001);
    assert_eq!(chains[0].conditions.len(), 1);
    assert_eq!(chains[0].actions.len(), 2);

    match &chains[0].actions[0] {
        Action::AcceptMission { mission_id } => assert_eq!(*mission_id, 622),
        other => panic!("Expected AcceptMission, got {:?}", other),
    }
}

/// Multiple `content_triggers` rows for the same chain id MUST
/// materialize one in-memory `Chain` per trigger (OR-semantics).
/// Pre-fix the loader silently dropped the 2nd+ rows, which
/// broke completion chains that needed to fire on either of two
/// guard tags (Mess Hall, Hallway05). Conditions and actions are
/// shared across the expanded chains.
#[test]
fn build_chains_from_rows_expands_multi_trigger_chain() {
    use crate::triggers::Trigger;

    let chain_rows = vec![DbChainRow {
        chain_id: 1087,
        description: Some("681 - Kill counter reached: complete 681, accept 682".to_string()),
        scope_type: "mission".to_string(),
        scope_id: Some(681),
        enabled: true,
        priority: 0,
    }];
    let trigger_rows = vec![
        DbTriggerRow {
            chain_id: 1087,
            event_type: "entity_dead_tag".to_string(),
            event_key: Some("MessHall_Guard1".to_string()),
            scope: "space".to_string(),
            once: false,
            sort_order: 0,
        },
        DbTriggerRow {
            chain_id: 1087,
            event_type: "entity_dead_tag".to_string(),
            event_key: Some("MessHall_Guard2".to_string()),
            scope: "space".to_string(),
            once: false,
            sort_order: 1,
        },
    ];
    let condition_rows = vec![DbConditionRow {
        chain_id: 1087,
        condition_type: "mission_status".to_string(),
        target_id: Some(681),
        target_key: None,
        operator: "eq".to_string(),
        value: Some("active".to_string()),
        sort_order: 0,
    }];
    let action_rows = vec![DbActionRow {
        chain_id: 1087,
        action_type: "complete_mission".to_string(),
        target_id: Some(681),
        target_key: None,
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    }];

    let chains = build_chains_from_rows(chain_rows, trigger_rows, condition_rows, action_rows);

    // Two trigger rows → two chains, both at the same chain_id.
    assert_eq!(
        chains.len(),
        2,
        "two content_triggers rows must produce two in-memory chains"
    );
    assert!(chains.iter().all(|c| c.id == 1087));

    // Conditions and actions are cloned across all expanded chains —
    // they share the same gating + side-effects regardless of which
    // trigger fired. A drift between the two would be a content
    // authoring footgun, so this assertion is part of the contract.
    for chain in &chains {
        assert_eq!(chain.conditions.len(), 1);
        assert_eq!(chain.actions.len(), 1);
    }

    // Distinct triggers on the two expansions, in declared sort_order.
    let triggers: Vec<&Trigger> = chains.iter().map(|c| &c.trigger).collect();
    match (triggers[0], triggers[1]) {
        (
            Trigger::OnEntityDeath {
                entity_tag: Some(a),
                ..
            },
            Trigger::OnEntityDeath {
                entity_tag: Some(b),
                ..
            },
        ) => {
            assert_eq!(a, "MessHall_Guard1");
            assert_eq!(b, "MessHall_Guard2");
        }
        other => panic!(
            "expected two OnEntityDeath triggers with distinct tags, got {:?}",
            other
        ),
    }
}

/// `event_type = "mission_accepted"` with the mission id in `event_key`
/// must round-trip into a `Trigger::OnMissionAccepted`. This is the
/// load-bearing path for chains like Aftermath chain 1097 that need
/// to react to mission start without piggybacking on whichever chain
/// did the accepting.
#[test]
fn mission_accepted_event_type_loads_as_on_mission_accepted_trigger() {
    use crate::triggers::Trigger;

    let row = DbTriggerRow {
        chain_id: 1097,
        event_type: "mission_accepted".to_string(),
        event_key: Some("687".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };

    match convert_trigger(&row) {
        Some(Trigger::OnMissionAccepted { mission_id }) => assert_eq!(mission_id, 687),
        other => panic!("expected OnMissionAccepted(687), got {:?}", other),
    }
}

/// `mission_accepted` without an `event_key` cannot resolve a target
/// mission — must drop to None so the loader skips the row rather
/// than firing on every accept event.
#[test]
fn mission_accepted_without_key_returns_none() {
    let row = DbTriggerRow {
        chain_id: 1097,
        event_type: "mission_accepted".to_string(),
        event_key: None,
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    assert!(convert_trigger(&row).is_none());
}

/// `item_equipped` accepts NULL `event_key` as a wildcard — the chain
/// should match any equipped item.
#[test]
fn item_equipped_null_key_loads_as_wildcard() {
    use crate::triggers::Trigger;
    let row = DbTriggerRow {
        chain_id: 9000,
        event_type: "item_equipped".to_string(),
        event_key: None,
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    match convert_trigger(&row) {
        Some(Trigger::OnItemEquipped { item_id: None }) => {}
        other => panic!("expected OnItemEquipped(wildcard), got {:?}", other),
    }
}

/// A specific integer key must round-trip as a typed filter.
#[test]
fn item_equipped_numeric_key_loads_as_typed_filter() {
    use crate::triggers::Trigger;
    let row = DbTriggerRow {
        chain_id: 1004,
        event_type: "item_equipped".to_string(),
        event_key: Some("55".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    match convert_trigger(&row) {
        Some(Trigger::OnItemEquipped { item_id: Some(55) }) => {}
        other => panic!("expected OnItemEquipped(55), got {:?}", other),
    }
}

/// A non-empty `event_key` that fails to parse as i32 must drop the
/// chain entirely. Silently collapsing `Some("bad")` into `None` would
/// turn a typo'd integer into a wildcard that fires for every equip
/// event — visible bug shape: an unrelated equip would advance an
/// unrelated mission.
#[test]
fn item_equipped_malformed_key_returns_none_not_wildcard() {
    let row = DbTriggerRow {
        chain_id: 9001,
        event_type: "item_equipped".to_string(),
        event_key: Some("not_a_number".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    assert!(
        convert_trigger(&row).is_none(),
        "malformed item_equipped event_key must reject the chain, not silently \
         load as a wildcard match",
    );
}

#[test]
fn triggerless_chain_gets_custom_event() {
    let chain_rows = vec![DbChainRow {
        chain_id: 1017,
        description: Some("Victory callback".to_string()),
        scope_type: "mission".to_string(),
        scope_id: Some(638),
        enabled: true,
        priority: 0,
    }];
    let action_rows = vec![DbActionRow {
        chain_id: 1017,
        action_type: "advance_step".to_string(),
        target_id: Some(638),
        target_key: Some("2116".to_string()),
        params: serde_json::json!({}),
        delay_ms: 0,
        sort_order: 0,
    }];

    let chains = build_chains_from_rows(chain_rows, vec![], vec![], action_rows);
    assert_eq!(chains.len(), 1);
    match &chains[0].trigger {
        Trigger::OnCustomEvent { event_name } => {
            assert!(event_name.contains("1017"));
        }
        other => panic!("Expected OnCustomEvent, got {:?}", other),
    }
}

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
fn convert_interact_tag_trigger() {
    let row = DbTriggerRow {
        chain_id: 1,
        event_type: "interact_tag".to_string(),
        event_key: Some("ArmYourself_FrostBody".to_string()),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    };
    let trigger = convert_trigger(&row).unwrap();
    match trigger {
        Trigger::OnInteractTag { entity_tag } => {
            assert_eq!(entity_tag, "ArmYourself_FrostBody")
        }
        other => panic!("Expected OnInteractTag, got {:?}", other),
    }
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

#[test]
fn convert_counter_condition() {
    let row = DbConditionRow {
        chain_id: 1,
        condition_type: "counter".to_string(),
        target_id: None,
        target_key: Some("hallway01_kills".to_string()),
        operator: "gte".to_string(),
        value: Some("3".to_string()),
        sort_order: 0,
    };
    let condition = convert_condition(&row).unwrap();
    match condition {
        Condition::Counter {
            counter_name,
            operator,
            value,
        } => {
            assert_eq!(counter_name, "hallway01_kills");
            assert_eq!(operator, ComparisonOp::Gte);
            assert_eq!(value, 3);
        }
        other => panic!("Expected Counter, got {:?}", other),
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

/// `parse_step_status` accepts each of the three valid step states.
/// `completed` is the recently-added third leaf (matches the `completed_steps`
/// list populated in `services::cell::content::mission_context`); without
/// this case the chain seed would silently drop any condition that uses
/// `eq completed` for a step.
#[test]
fn parse_step_status_accepts_all_three_states() {
    assert_eq!(
        parse_step_status("not_active"),
        Some(StepStatusValue::NotActive)
    );
    assert_eq!(parse_step_status("active"), Some(StepStatusValue::Active));
    assert_eq!(
        parse_step_status("completed"),
        Some(StepStatusValue::Completed)
    );
    // Unknown values fall through to None — the loader logs a warning
    // and skips the condition rather than misinterpreting it.
    assert_eq!(parse_step_status("nonsense"), None);
    assert_eq!(parse_step_status(""), None);
}
