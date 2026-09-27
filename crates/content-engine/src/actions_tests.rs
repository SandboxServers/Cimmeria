//! Unit tests for [`super::Action::execute`] and the action enum,
//! kept beside `actions.rs` so that file stays under the size cap.

use super::*;

#[test]
fn trigger_chain_returns_chain_trigger_result() {
    let action = Action::TriggerChain { chain_id: 42 };
    let mut ctx = ExecutionContext::new();
    match action.execute(&mut ctx) {
        ActionResult::ChainTrigger(id) => assert_eq!(id, 42),
        other => panic!("Expected ChainTrigger(42), got {:?}", other),
    }
}

#[test]
fn action_serialization_roundtrip() {
    let action = Action::GrantXP { amount: 500 };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    let _ = format!("{:?}", deserialized);
}

#[test]
fn property_op_serialization_roundtrip() {
    let ops = vec![
        PropertyOp::Set,
        PropertyOp::Add,
        PropertyOp::Subtract,
        PropertyOp::Multiply,
    ];
    for op in &ops {
        let json = serde_json::to_string(op).unwrap();
        let deserialized: PropertyOp = serde_json::from_str(&json).unwrap();
        let _ = format!("{:?}", deserialized);
    }
}

#[test]
fn complex_action_serialization() {
    let action = Action::ModifyProperty {
        property: "health".to_string(),
        operation: PropertyOp::Subtract,
        value: serde_json::json!(25),
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("health"));
    assert!(json.contains("Subtract"));
    assert!(json.contains("25"));
}

#[test]
fn teleport_action_serialization() {
    let action = Action::Teleport {
        space_id: 7,
        position: [100.0, 200.0, 300.0],
    };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::Teleport { space_id, position } => {
            assert_eq!(space_id, 7);
            assert_eq!(position, [100.0, 200.0, 300.0]);
        }
        _ => panic!("Expected Teleport variant"),
    }
}

#[test]
fn grant_item_with_container() {
    let action = Action::GrantItem {
        item_id: 55,
        count: 1,
        container_id: Some(3),
    };
    let json = serde_json::to_string(&action).unwrap();
    assert!(json.contains("container_id"));
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::GrantItem {
            item_id,
            count,
            container_id,
        } => {
            assert_eq!(item_id, 55);
            assert_eq!(count, 1);
            assert_eq!(container_id, Some(3));
        }
        _ => panic!("Expected GrantItem"),
    }
}

#[test]
fn grant_item_without_container_defaults_none() {
    let json = r#"{"GrantItem": {"item_id": 55, "count": 1}}"#;
    let deserialized: Action = serde_json::from_str(json).unwrap();
    match deserialized {
        Action::GrantItem { container_id, .. } => assert_eq!(container_id, None),
        _ => panic!("Expected GrantItem"),
    }
}

#[test]
fn move_waypoint_serialization() {
    let action = Action::MoveWaypoint {
        entity_tag: "NID_Guard_01".to_string(),
        destination: [-296.715, 68.511, -166.125],
        speed: 1.5,
    };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::MoveWaypoint {
            entity_tag,
            destination,
            speed,
        } => {
            assert_eq!(entity_tag, "NID_Guard_01");
            assert_eq!(destination, [-296.715, 68.511, -166.125]);
            assert!((speed - 1.5).abs() < f32::EPSILON);
        }
        _ => panic!("Expected MoveWaypoint"),
    }
}

#[test]
fn set_active_slot_serialization() {
    let action = Action::SetActiveSlot { bag_id: 3, slot: 0 };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::SetActiveSlot { bag_id, slot } => {
            assert_eq!(bag_id, 3);
            assert_eq!(slot, 0);
        }
        _ => panic!("Expected SetActiveSlot"),
    }
}

#[test]
fn launch_ability_serialization() {
    let action = Action::LaunchAbility {
        ability_id: 1372,
        entity_tag: Some("NID_Guard_01".to_string()),
    };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::LaunchAbility {
            ability_id,
            entity_tag,
        } => {
            assert_eq!(ability_id, 1372);
            assert_eq!(entity_tag, Some("NID_Guard_01".to_string()));
        }
        _ => panic!("Expected LaunchAbility"),
    }
}

#[test]
fn launch_ability_without_entity_tag() {
    let json = r#"{"LaunchAbility": {"ability_id": 500}}"#;
    let deserialized: Action = serde_json::from_str(json).unwrap();
    match deserialized {
        Action::LaunchAbility {
            ability_id,
            entity_tag,
        } => {
            assert_eq!(ability_id, 500);
            assert_eq!(entity_tag, None);
        }
        _ => panic!("Expected LaunchAbility"),
    }
}

#[test]
fn accept_mission_serialization() {
    let action = Action::AcceptMission { mission_id: 622 };
    let json = serde_json::to_string(&action).unwrap();
    let deserialized: Action = serde_json::from_str(&json).unwrap();
    match deserialized {
        Action::AcceptMission { mission_id } => assert_eq!(mission_id, 622),
        _ => panic!("Expected AcceptMission"),
    }
}

/// A payload serialized before `difficulty` existed must still
/// deserialize, taking the same default the DB-row loader applies.
/// Without `#[serde(default)]` this fails on a missing field, so an
/// older stored `Action` would break rather than degrade.
#[test]
fn start_minigame_deserializes_a_payload_without_difficulty() {
    let json = r#"{"StartMinigame":{"minigame_type":"Livewire","on_victory_chains":[1017]}}"#;
    let action: Action =
        serde_json::from_str(json).expect("a pre-difficulty payload must still deserialize");
    match action {
        Action::StartMinigame {
            minigame_type,
            difficulty,
            on_victory_chains,
        } => {
            assert_eq!(minigame_type, "Livewire");
            assert_eq!(difficulty, 1, "the omitted field must default to 1");
            assert_eq!(on_victory_chains, vec![1017]);
        }
        other => panic!("expected StartMinigame, got {other:?}"),
    }
}

/// An explicit difficulty must survive a round trip unchanged -- the
/// default must not shadow an authored value.
#[test]
fn start_minigame_round_trips_an_explicit_difficulty() {
    let original = Action::StartMinigame {
        minigame_type: "Livewire".to_string(),
        difficulty: 4,
        on_victory_chains: vec![1042],
    };
    let json = serde_json::to_string(&original).unwrap();
    let back: Action = serde_json::from_str(&json).unwrap();
    match back {
        Action::StartMinigame { difficulty, .. } => assert_eq!(difficulty, 4),
        other => panic!("expected StartMinigame, got {other:?}"),
    }
}
