//! `Trigger::matches()` coverage for the remaining trigger families:
//! regions, dialogs, items, timers, abilities, interactions, teleports,
//! effects, custom events, and player-loaded.

use super::super::*;
use super::make_event;

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
fn effect_init_matches() {
    let trigger = Trigger::OnEffectInit;
    let event = make_event(TriggerType::EffectInit, vec![]);
    assert!(trigger.matches(&event));
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
