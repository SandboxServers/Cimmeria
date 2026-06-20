//! `Trigger::matches()` coverage for the entity lifecycle triggers:
//! `OnEntityCreated`, `OnEntityDestroyed`, and `OnEntityDeath`.

use super::super::*;
use super::make_event;

#[test]
fn wildcard_trigger_matches_any() {
    let trigger = Trigger::OnEntityCreated { entity_type: None };
    let event = make_event(
        TriggerType::EntityCreated,
        vec![(
            "entity_type",
            serde_json::Value::String("SGWMob".to_string()),
        )],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn filtered_trigger_matches_correct_type() {
    let trigger = Trigger::OnEntityCreated {
        entity_type: Some("SGWMob".to_string()),
    };
    let event = make_event(
        TriggerType::EntityCreated,
        vec![(
            "entity_type",
            serde_json::Value::String("SGWMob".to_string()),
        )],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn filtered_trigger_rejects_wrong_type() {
    let trigger = Trigger::OnEntityCreated {
        entity_type: Some("SGWMob".to_string()),
    };
    let event = make_event(
        TriggerType::EntityCreated,
        vec![(
            "entity_type",
            serde_json::Value::String("SGWPlayer".to_string()),
        )],
    );
    assert!(!trigger.matches(&event));
}

#[test]
fn wrong_trigger_type_never_matches() {
    let trigger = Trigger::OnEntityCreated { entity_type: None };
    let event = make_event(TriggerType::EntityDeath, vec![]);
    assert!(!trigger.matches(&event));
}

#[test]
fn entity_death_by_tag() {
    let trigger = Trigger::OnEntityDeath {
        entity_type: None,
        entity_tag: Some("Hallway01_Guard".to_string()),
    };
    let event = make_event(
        TriggerType::EntityDeath,
        vec![("entity_tag", serde_json::json!("Hallway01_Guard"))],
    );
    assert!(trigger.matches(&event));
}

// ─── OnEntityCreated / OnEntityDestroyed (shared arm) ──────────

#[test]
fn entity_created_wildcard_matches_when_param_absent() {
    let trigger = Trigger::OnEntityCreated { entity_type: None };
    let event = make_event(TriggerType::EntityCreated, vec![]);
    assert!(trigger.matches(&event));
}

#[test]
fn entity_created_filtered_rejects_missing_param() {
    let trigger = Trigger::OnEntityCreated {
        entity_type: Some("SGWMob".to_string()),
    };
    let event = make_event(TriggerType::EntityCreated, vec![]);
    assert!(!trigger.matches(&event));
}

#[test]
fn entity_destroyed_wildcard_matches_any() {
    let trigger = Trigger::OnEntityDestroyed { entity_type: None };
    let event = make_event(
        TriggerType::EntityDestroyed,
        vec![("entity_type", serde_json::json!("SGWMob"))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn entity_destroyed_filtered_matches_and_rejects() {
    let trigger = Trigger::OnEntityDestroyed {
        entity_type: Some("SGWMob".to_string()),
    };
    let hit = make_event(
        TriggerType::EntityDestroyed,
        vec![("entity_type", serde_json::json!("SGWMob"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::EntityDestroyed,
        vec![("entity_type", serde_json::json!("SGWPlayer"))],
    );
    assert!(!trigger.matches(&miss));
}

// ─── OnEntityDeath: type path + wildcard ──────────────────────

#[test]
fn entity_death_by_type_matches_and_rejects() {
    let trigger = Trigger::OnEntityDeath {
        entity_type: Some("SGWMob".to_string()),
        entity_tag: None,
    };
    let hit = make_event(
        TriggerType::EntityDeath,
        vec![("entity_type", serde_json::json!("SGWMob"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::EntityDeath,
        vec![("entity_type", serde_json::json!("SGWPlayer"))],
    );
    assert!(!trigger.matches(&miss));
}

#[test]
fn entity_death_wildcard_matches_any() {
    let trigger = Trigger::OnEntityDeath {
        entity_type: None,
        entity_tag: None,
    };
    let event = make_event(TriggerType::EntityDeath, vec![]);
    assert!(trigger.matches(&event));
}

#[test]
fn entity_death_by_tag_rejects_wrong_tag() {
    let trigger = Trigger::OnEntityDeath {
        entity_type: None,
        entity_tag: Some("Hallway01_Guard".to_string()),
    };
    let event = make_event(
        TriggerType::EntityDeath,
        vec![("entity_tag", serde_json::json!("Hallway02_Guard"))],
    );
    assert!(!trigger.matches(&event));
}
