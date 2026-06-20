//! Chain-level loading: JSON deserialization, DB-row assembly into
//! `Chain`s (including multi-trigger expansion and triggerless
//! custom-event fallback), and the step-status parser.

use super::super::condition::parse_step_status;
use super::super::*;
use crate::conditions::StepStatusValue;

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
