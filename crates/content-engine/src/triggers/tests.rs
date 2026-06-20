use super::*;

fn make_event(trigger_type: TriggerType, params: Vec<(&str, serde_json::Value)>) -> TriggerEvent {
    TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: params
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    }
}

#[test]
fn trigger_type_discriminant() {
    let t = Trigger::OnEntityCreated { entity_type: None };
    assert_eq!(t.trigger_type(), TriggerType::EntityCreated);

    let t = Trigger::OnTimer {
        timer_name: "respawn".to_string(),
    };
    assert_eq!(t.trigger_type(), TriggerType::Timer);

    let t = Trigger::OnDialogChoice { dialog_id: 100 };
    assert_eq!(t.trigger_type(), TriggerType::DialogChoice);

    assert_eq!(
        Trigger::OnEffectInit.trigger_type(),
        TriggerType::EffectInit
    );
}

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
fn region_enter_matches_correct_key() {
    let trigger = Trigger::OnRegionEnter {
        region_key: "Castle_Cellblock.Region2".to_string(),
    };
    let event = make_event(
        TriggerType::RegionEnter,
        vec![("region_key", serde_json::json!("Castle_Cellblock.Region2"))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn region_enter_rejects_wrong_key() {
    let trigger = Trigger::OnRegionEnter {
        region_key: "Castle_Cellblock.Region2".to_string(),
    };
    let event = make_event(
        TriggerType::RegionEnter,
        vec![("region_key", serde_json::json!("SGC_W1.Region1"))],
    );
    assert!(!trigger.matches(&event));
}

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
fn custom_event_matches_name() {
    let trigger = Trigger::OnCustomEvent {
        event_name: "boss_phase_2".to_string(),
    };
    let event = make_event(
        TriggerType::CustomEvent,
        vec![(
            "event_name",
            serde_json::Value::String("boss_phase_2".to_string()),
        )],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn dialog_choice_matches() {
    let trigger = Trigger::OnDialogChoice { dialog_id: 5021 };
    let event = make_event(
        TriggerType::DialogChoice,
        vec![("dialog_id", serde_json::json!(5021))],
    );
    assert!(trigger.matches(&event));
}

#[test]
fn interact_tag_matches() {
    let trigger = Trigger::OnInteractTag {
        entity_tag: "ArmYourself_FrostBody".to_string(),
    };
    let event = make_event(
        TriggerType::InteractTag,
        vec![("entity_tag", serde_json::json!("ArmYourself_FrostBody"))],
    );
    assert!(trigger.matches(&event));
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

#[test]
fn effect_init_matches() {
    let trigger = Trigger::OnEffectInit;
    let event = make_event(TriggerType::EffectInit, vec![]);
    assert!(trigger.matches(&event));
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

#[test]
fn item_equipped_filters_by_item_id() {
    let trigger = Trigger::OnItemEquipped { item_id: Some(55) };
    let pistol_event = make_event(
        TriggerType::ItemEquipped,
        vec![("item_id", serde_json::json!(55))],
    );
    assert!(trigger.matches(&pistol_event));

    let p90_event = make_event(
        TriggerType::ItemEquipped,
        vec![("item_id", serde_json::json!(21))],
    );
    assert!(!trigger.matches(&p90_event));
}

#[test]
fn item_equipped_wildcard_matches_any() {
    let trigger = Trigger::OnItemEquipped { item_id: None };
    let event = make_event(
        TriggerType::ItemEquipped,
        vec![("item_id", serde_json::json!(123))],
    );
    assert!(trigger.matches(&event));
}

// ─── Cover triggers ────────────────────────────────────────────

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

#[test]
fn cover_triggers_have_correct_discriminants() {
    assert_eq!(
        Trigger::OnPlayerEnteredCover { cover_set_id: None }.trigger_type(),
        TriggerType::PlayerEnteredCover
    );
    assert_eq!(
        Trigger::OnPlayerLeftCover { cover_set_id: None }.trigger_type(),
        TriggerType::PlayerLeftCover
    );
    assert_eq!(
        Trigger::OnPlayerInCoverDuration {
            cover_set_id: None,
            seconds: 3,
        }
        .trigger_type(),
        TriggerType::PlayerInCoverDuration
    );
    assert_eq!(
        Trigger::OnNpcFlanked { npc_template: None }.trigger_type(),
        TriggerType::NpcFlanked
    );
}
