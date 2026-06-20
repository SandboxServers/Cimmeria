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

// ─── trigger_type() mapping (one assertion per variant) ────────

#[test]
fn trigger_type_covers_every_variant() {
    use TriggerType::*;
    let cases: Vec<(Trigger, TriggerType)> = vec![
        (
            Trigger::OnEntityCreated { entity_type: None },
            EntityCreated,
        ),
        (
            Trigger::OnEntityDestroyed { entity_type: None },
            EntityDestroyed,
        ),
        (
            Trigger::OnEntityDeath {
                entity_type: None,
                entity_tag: None,
            },
            EntityDeath,
        ),
        (Trigger::OnAbilityUsed { ability_id: None }, AbilityUsed),
        (
            Trigger::OnInteraction {
                interaction_type: None,
            },
            Interaction,
        ),
        (
            Trigger::OnRegionEnter {
                region_key: "r".to_string(),
            },
            RegionEnter,
        ),
        (
            Trigger::OnRegionExit {
                region_key: "r".to_string(),
            },
            RegionExit,
        ),
        (
            Trigger::OnMissionStep {
                mission_id: 1,
                step: 1,
            },
            MissionStep,
        ),
        (Trigger::OnItemAcquired { item_id: None }, ItemAcquired),
        (
            Trigger::OnTimer {
                timer_name: "t".to_string(),
            },
            Timer,
        ),
        (
            Trigger::OnCustomEvent {
                event_name: "e".to_string(),
            },
            CustomEvent,
        ),
        (Trigger::OnPlayerLoaded { world_name: None }, PlayerLoaded),
        (Trigger::OnDialogOpen { dialog_id: 1 }, DialogOpen),
        (Trigger::OnDialogChoice { dialog_id: 1 }, DialogChoice),
        (
            Trigger::OnInteractTag {
                entity_tag: "tag".to_string(),
            },
            InteractTag,
        ),
        (
            Trigger::OnInteractTemplate {
                template_name: "tpl".to_string(),
            },
            InteractTemplate,
        ),
        (Trigger::OnItemUse { item_id: 1 }, ItemUse),
        (Trigger::OnItemEquipped { item_id: None }, ItemEquipped),
        (Trigger::OnTeleportIn { region_id: 1 }, TeleportIn),
        (Trigger::OnEffectInit, EffectInit),
        (Trigger::OnEffectPulseBegin, EffectPulseBegin),
        (Trigger::OnEffectPulseEnd, EffectPulseEnd),
        (Trigger::OnEffectRemoved, EffectRemoved),
        (
            Trigger::OnMissionCompleted { mission_id: 1 },
            MissionCompleted,
        ),
        (
            Trigger::OnDialogSetOpen {
                dialog_set_name: "ds".to_string(),
            },
            DialogSetOpen,
        ),
        (
            Trigger::OnMissionAccepted { mission_id: 1 },
            MissionAccepted,
        ),
        (
            Trigger::OnPlayerEnteredCover { cover_set_id: None },
            PlayerEnteredCover,
        ),
        (
            Trigger::OnPlayerLeftCover { cover_set_id: None },
            PlayerLeftCover,
        ),
        (
            Trigger::OnPlayerInCoverDuration {
                cover_set_id: None,
                seconds: 1,
            },
            PlayerInCoverDuration,
        ),
        (Trigger::OnNpcFlanked { npc_template: None }, NpcFlanked),
    ];
    for (trigger, expected) in cases {
        assert_eq!(trigger.trigger_type(), expected, "for {trigger:?}");
    }
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

// ─── OnAbilityUsed ────────────────────────────────────────────

#[test]
fn ability_used_filtered_matches_and_rejects() {
    let trigger = Trigger::OnAbilityUsed {
        ability_id: Some(42),
    };
    let hit = make_event(
        TriggerType::AbilityUsed,
        vec![("ability_id", serde_json::json!(42))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::AbilityUsed,
        vec![("ability_id", serde_json::json!(7))],
    );
    assert!(!trigger.matches(&miss));
}

#[test]
fn ability_used_wildcard_matches_any() {
    let trigger = Trigger::OnAbilityUsed { ability_id: None };
    let event = make_event(
        TriggerType::AbilityUsed,
        vec![("ability_id", serde_json::json!(999))],
    );
    assert!(trigger.matches(&event));
}

// ─── OnInteraction ────────────────────────────────────────────

#[test]
fn interaction_filtered_matches_and_rejects() {
    let trigger = Trigger::OnInteraction {
        interaction_type: Some("dialog".to_string()),
    };
    let hit = make_event(
        TriggerType::Interaction,
        vec![("interaction_type", serde_json::json!("dialog"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::Interaction,
        vec![("interaction_type", serde_json::json!("use"))],
    );
    assert!(!trigger.matches(&miss));
}

#[test]
fn interaction_wildcard_matches_any() {
    let trigger = Trigger::OnInteraction {
        interaction_type: None,
    };
    let event = make_event(
        TriggerType::Interaction,
        vec![("interaction_type", serde_json::json!("anything"))],
    );
    assert!(trigger.matches(&event));
}

// ─── OnRegionExit ─────────────────────────────────────────────

#[test]
fn region_exit_matches_and_rejects() {
    let trigger = Trigger::OnRegionExit {
        region_key: "Castle_Cellblock.Region2".to_string(),
    };
    let hit = make_event(
        TriggerType::RegionExit,
        vec![("region_key", serde_json::json!("Castle_Cellblock.Region2"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::RegionExit,
        vec![("region_key", serde_json::json!("SGC_W1.Region1"))],
    );
    assert!(!trigger.matches(&miss));
}

// ─── OnRegionEnter: missing param rejects ─────────────────────

#[test]
fn region_enter_rejects_missing_param() {
    let trigger = Trigger::OnRegionEnter {
        region_key: "R".to_string(),
    };
    let event = make_event(TriggerType::RegionEnter, vec![]);
    assert!(!trigger.matches(&event));
}

// ─── OnMissionStep: mission match but step mismatch ───────────

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

// ─── OnItemAcquired ───────────────────────────────────────────

#[test]
fn item_acquired_filtered_matches_and_rejects() {
    let trigger = Trigger::OnItemAcquired { item_id: Some(55) };
    let hit = make_event(
        TriggerType::ItemAcquired,
        vec![("item_id", serde_json::json!(55))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::ItemAcquired,
        vec![("item_id", serde_json::json!(21))],
    );
    assert!(!trigger.matches(&miss));
}

#[test]
fn item_acquired_wildcard_matches_any() {
    let trigger = Trigger::OnItemAcquired { item_id: None };
    let event = make_event(
        TriggerType::ItemAcquired,
        vec![("item_id", serde_json::json!(7))],
    );
    assert!(trigger.matches(&event));
}

// ─── OnTimer ──────────────────────────────────────────────────

#[test]
fn timer_matches_and_rejects() {
    let trigger = Trigger::OnTimer {
        timer_name: "respawn".to_string(),
    };
    let hit = make_event(
        TriggerType::Timer,
        vec![("timer_name", serde_json::json!("respawn"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::Timer,
        vec![("timer_name", serde_json::json!("despawn"))],
    );
    assert!(!trigger.matches(&miss));
}

// ─── OnCustomEvent: reject path ───────────────────────────────

#[test]
fn custom_event_rejects_wrong_name() {
    let trigger = Trigger::OnCustomEvent {
        event_name: "boss_phase_2".to_string(),
    };
    let event = make_event(
        TriggerType::CustomEvent,
        vec![("event_name", serde_json::json!("boss_phase_1"))],
    );
    assert!(!trigger.matches(&event));
}

// ─── OnPlayerLoaded ───────────────────────────────────────────

#[test]
fn player_loaded_filtered_matches_and_rejects() {
    let trigger = Trigger::OnPlayerLoaded {
        world_name: Some("Castle_Cellblock".to_string()),
    };
    let hit = make_event(
        TriggerType::PlayerLoaded,
        vec![("world_name", serde_json::json!("Castle_Cellblock"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::PlayerLoaded,
        vec![("world_name", serde_json::json!("SGC_W1"))],
    );
    assert!(!trigger.matches(&miss));
}

#[test]
fn player_loaded_wildcard_matches_any() {
    let trigger = Trigger::OnPlayerLoaded { world_name: None };
    let event = make_event(
        TriggerType::PlayerLoaded,
        vec![("world_name", serde_json::json!("anywhere"))],
    );
    assert!(trigger.matches(&event));
}

// ─── OnDialogOpen (shares arm with OnDialogChoice) ────────────

#[test]
fn dialog_open_matches_and_rejects() {
    let trigger = Trigger::OnDialogOpen { dialog_id: 5021 };
    let hit = make_event(
        TriggerType::DialogOpen,
        vec![("dialog_id", serde_json::json!(5021))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::DialogOpen,
        vec![("dialog_id", serde_json::json!(5022))],
    );
    assert!(!trigger.matches(&miss));
}

#[test]
fn dialog_choice_rejects_wrong_id() {
    let trigger = Trigger::OnDialogChoice { dialog_id: 5021 };
    let event = make_event(
        TriggerType::DialogChoice,
        vec![("dialog_id", serde_json::json!(1))],
    );
    assert!(!trigger.matches(&event));
}

// ─── OnInteractTag: reject path ───────────────────────────────

#[test]
fn interact_tag_rejects_wrong_tag() {
    let trigger = Trigger::OnInteractTag {
        entity_tag: "ArmYourself_FrostBody".to_string(),
    };
    let event = make_event(
        TriggerType::InteractTag,
        vec![("entity_tag", serde_json::json!("SomethingElse"))],
    );
    assert!(!trigger.matches(&event));
}

// ─── OnInteractTemplate ───────────────────────────────────────

#[test]
fn interact_template_matches_and_rejects() {
    let trigger = Trigger::OnInteractTemplate {
        template_name: "ConsolePanel".to_string(),
    };
    let hit = make_event(
        TriggerType::InteractTemplate,
        vec![("template_name", serde_json::json!("ConsolePanel"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::InteractTemplate,
        vec![("template_name", serde_json::json!("OtherPanel"))],
    );
    assert!(!trigger.matches(&miss));
}

// ─── OnItemUse (required item_id, no wildcard) ────────────────

#[test]
fn item_use_matches_and_rejects() {
    let trigger = Trigger::OnItemUse { item_id: 7001 };
    let hit = make_event(
        TriggerType::ItemUse,
        vec![("item_id", serde_json::json!(7001))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::ItemUse,
        vec![("item_id", serde_json::json!(7002))],
    );
    assert!(!trigger.matches(&miss));
}

// ─── OnTeleportIn ─────────────────────────────────────────────

#[test]
fn teleport_in_matches_and_rejects() {
    let trigger = Trigger::OnTeleportIn { region_id: 88 };
    let hit = make_event(
        TriggerType::TeleportIn,
        vec![("region_id", serde_json::json!(88))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::TeleportIn,
        vec![("region_id", serde_json::json!(89))],
    );
    assert!(!trigger.matches(&miss));
}

// ─── Unit effect triggers (match any event of right type) ─────

#[test]
fn effect_unit_triggers_match_any_event() {
    for (trigger, ty) in [
        (Trigger::OnEffectPulseBegin, TriggerType::EffectPulseBegin),
        (Trigger::OnEffectPulseEnd, TriggerType::EffectPulseEnd),
        (Trigger::OnEffectRemoved, TriggerType::EffectRemoved),
    ] {
        let event = make_event(ty, vec![]);
        assert!(
            trigger.matches(&event),
            "{trigger:?} should match {event:?}"
        );
    }
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

// ─── OnDialogSetOpen ──────────────────────────────────────────

#[test]
fn dialog_set_open_matches_and_rejects() {
    let trigger = Trigger::OnDialogSetOpen {
        dialog_set_name: "Cellblock_Intro".to_string(),
    };
    let hit = make_event(
        TriggerType::DialogSetOpen,
        vec![("dialog_set_name", serde_json::json!("Cellblock_Intro"))],
    );
    assert!(trigger.matches(&hit));
    let miss = make_event(
        TriggerType::DialogSetOpen,
        vec![("dialog_set_name", serde_json::json!("Cellblock_Outro"))],
    );
    assert!(!trigger.matches(&miss));
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
