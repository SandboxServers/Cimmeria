//! `trigger_type()` discriminant mapping — one assertion per variant family.

use super::super::*;

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
