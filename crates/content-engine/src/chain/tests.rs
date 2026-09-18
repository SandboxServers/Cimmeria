//! Tests for [`super::Chain`] / [`super::ChainEngine`] — registration,
//! priority ordering, `fire_event`, `resolve_event`, and JSON
//! (de)serialization round-tripping. Split out of `chain/mod.rs` per
//! `CLAUDE.md`'s file-size guidance once this block crossed the 500-line
//! soft cap — content unchanged from before the split.

use super::*;
use crate::triggers::TriggerType;

/// Helper to build a simple chain with no conditions.
fn make_chain(id: i64, trigger: Trigger, actions: Vec<Action>, priority: i32) -> Chain {
    Chain {
        id,
        name: format!("test_chain_{}", id),
        enabled: true,
        trigger,
        conditions: Vec::new(),
        actions,
        action_delays: Vec::new(),
        priority,
    }
}

#[test]
fn new_engine_has_no_chains() {
    let engine = ChainEngine::new();
    assert_eq!(engine.chain_count(), 0);
}

#[test]
fn register_chain_increases_count() {
    let mut engine = ChainEngine::new();
    let chain = make_chain(1, Trigger::OnEntityCreated { entity_type: None }, vec![], 0);
    engine.register_chain(chain);
    assert_eq!(engine.chain_count(), 1);
    assert_eq!(engine.chains_for_trigger(&TriggerType::EntityCreated), 1);
}

#[test]
fn chains_sorted_by_priority_descending() {
    let mut engine = ChainEngine::new();
    engine.register_chain(make_chain(
        1,
        Trigger::OnEntityCreated { entity_type: None },
        vec![],
        10,
    ));
    engine.register_chain(make_chain(
        2,
        Trigger::OnEntityCreated { entity_type: None },
        vec![],
        50,
    ));
    engine.register_chain(make_chain(
        3,
        Trigger::OnEntityCreated { entity_type: None },
        vec![],
        30,
    ));

    let chains = engine
        .chains_by_trigger
        .get(&TriggerType::EntityCreated)
        .unwrap();
    assert_eq!(chains[0].id, 2); // priority 50
    assert_eq!(chains[1].id, 3); // priority 30
    assert_eq!(chains[2].id, 1); // priority 10
}

#[test]
fn fire_event_with_no_matching_chains() {
    let engine = ChainEngine::new();
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityCreated,
        source_entity: None,
        target_entity: None,
        params: HashMap::new(),
    };
    let mut ctx = ExecutionContext::new();
    // Should not panic - just a no-op.
    engine.fire_event(&event, &mut ctx);
    assert!(ctx.results.is_empty());
}

#[test]
fn disabled_chain_is_skipped() {
    let mut engine = ChainEngine::new();
    let mut chain = make_chain(
        1,
        Trigger::OnEntityCreated { entity_type: None },
        vec![Action::TriggerChain { chain_id: 99 }],
        0,
    );
    chain.enabled = false;
    engine.register_chain(chain);

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityCreated,
        source_entity: None,
        target_entity: None,
        params: HashMap::new(),
    };
    let mut ctx = ExecutionContext::new();
    engine.fire_event(&event, &mut ctx);
    // No actions should have executed.
    assert!(ctx.results.is_empty());
}

#[test]
fn trigger_chain_action_produces_chain_trigger_result() {
    let mut engine = ChainEngine::new();
    let chain = make_chain(
        1,
        Trigger::OnCustomEvent {
            event_name: "test".to_string(),
        },
        vec![Action::TriggerChain { chain_id: 42 }],
        0,
    );
    engine.register_chain(chain);

    let mut params = HashMap::new();
    params.insert(
        "event_name".to_string(),
        serde_json::Value::String("test".to_string()),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::CustomEvent,
        source_entity: None,
        target_entity: None,
        params,
    };
    let mut ctx = ExecutionContext::new();
    engine.fire_event(&event, &mut ctx);

    assert_eq!(ctx.results.len(), 1);
    match &ctx.results[0] {
        ActionResult::ChainTrigger(id) => assert_eq!(*id, 42),
        other => panic!("Expected ChainTrigger(42), got {:?}", other),
    }
}

#[test]
fn chain_serialization_roundtrip() {
    let chain = Chain {
        id: 1,
        name: "Test Chain".to_string(),
        enabled: true,
        trigger: Trigger::OnEntityDeath {
            entity_type: Some("SGWMob".to_string()),
            entity_tag: None,
        },
        conditions: vec![Condition::HasItem {
            item_id: 10,
            min_count: Some(1),
        }],
        actions: vec![
            Action::GrantXP { amount: 100 },
            Action::GrantItem {
                item_id: 20,
                count: 1,
                container_id: None,
            },
        ],
        action_delays: Vec::new(),
        priority: 5,
    };

    let json = serde_json::to_string_pretty(&chain).unwrap();
    let deserialized: Chain = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, 1);
    assert_eq!(deserialized.name, "Test Chain");
    assert!(deserialized.enabled);
    assert_eq!(deserialized.priority, 5);
    assert_eq!(deserialized.conditions.len(), 1);
    assert_eq!(deserialized.actions.len(), 2);
}

#[test]
fn multiple_trigger_types_are_independent() {
    let mut engine = ChainEngine::new();
    engine.register_chain(make_chain(
        1,
        Trigger::OnEntityCreated { entity_type: None },
        vec![],
        0,
    ));
    engine.register_chain(make_chain(
        2,
        Trigger::OnEntityDeath {
            entity_type: None,
            entity_tag: None,
        },
        vec![],
        0,
    ));
    engine.register_chain(make_chain(
        3,
        Trigger::OnEntityDeath {
            entity_type: None,
            entity_tag: None,
        },
        vec![],
        0,
    ));

    assert_eq!(engine.chain_count(), 3);
    assert_eq!(engine.chains_for_trigger(&TriggerType::EntityCreated), 1);
    assert_eq!(engine.chains_for_trigger(&TriggerType::EntityDeath), 2);
    assert_eq!(engine.chains_for_trigger(&TriggerType::Timer), 0);
}

/// `resolve_event` must thread each action's `action_delays[i]` into
/// the resolved `(chain_id, action, delay_ms)` triple, index-aligned
/// with `actions`. C08a regression guard: before this field existed,
/// `content_actions.delay_ms` was loaded into `DbActionRow` and then
/// silently dropped — `resolve_event` had no way to carry it forward
/// at all (`ResolvedActions.actions` was a 2-tuple).
#[test]
fn resolve_event_threads_per_action_delay_ms() {
    let mut engine = ChainEngine::new();
    let chain = Chain {
        id: 42,
        name: "test: mixed delay actions".to_string(),
        enabled: true,
        trigger: Trigger::OnCustomEvent {
            event_name: "test".to_string(),
        },
        conditions: vec![],
        actions: vec![
            Action::GrantXP { amount: 1 },
            Action::GrantXP { amount: 2 },
            Action::GrantXP { amount: 3 },
        ],
        // Index-aligned: action 0 fires now, action 1 is deferred
        // 10.1s (the Straegis-scene shape), action 2 has no entry —
        // must default to 0, not panic or misalign.
        action_delays: vec![0, 10_100],
        priority: 0,
    };
    engine.register_chain(chain);

    let mut params = HashMap::new();
    params.insert(
        "event_name".to_string(),
        serde_json::Value::String("test".to_string()),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::CustomEvent,
        source_entity: None,
        target_entity: None,
        params,
    };
    let ctx = ExecutionContext::new();
    let resolved = engine.resolve_event(&event, &ctx);

    assert_eq!(resolved.actions.len(), 3);
    assert_eq!(
        resolved.action_delays.len(),
        3,
        "action_delays must be index-aligned with actions"
    );

    let (chain_id_0, action_0) = &resolved.actions[0];
    assert_eq!(*chain_id_0, 42);
    assert!(matches!(action_0, Action::GrantXP { amount: 1 }));
    assert_eq!(
        resolved.action_delays[0], 0,
        "action 0 has no delay row → fires now"
    );

    let (chain_id_1, action_1) = &resolved.actions[1];
    assert_eq!(*chain_id_1, 42);
    assert!(matches!(action_1, Action::GrantXP { amount: 2 }));
    assert_eq!(
        resolved.action_delays[1], 10_100,
        "action 1 must carry its 10.1s delay"
    );

    let (chain_id_2, action_2) = &resolved.actions[2];
    assert_eq!(*chain_id_2, 42);
    assert!(matches!(action_2, Action::GrantXP { amount: 3 }));
    assert_eq!(
        resolved.action_delays[2], 0,
        "action_delays shorter than actions must default missing entries to 0"
    );
}
