//! Discriminant lookup and runtime event matching for [`Trigger`].

use super::{Trigger, TriggerEvent, TriggerType};

impl Trigger {
    /// Returns the [`TriggerType`] discriminant for this trigger.
    pub fn trigger_type(&self) -> TriggerType {
        match self {
            Trigger::OnEntityCreated { .. } => TriggerType::EntityCreated,
            Trigger::OnEntityDestroyed { .. } => TriggerType::EntityDestroyed,
            Trigger::OnEntityDeath { .. } => TriggerType::EntityDeath,
            Trigger::OnAbilityUsed { .. } => TriggerType::AbilityUsed,
            Trigger::OnInteraction { .. } => TriggerType::Interaction,
            Trigger::OnRegionEnter { .. } => TriggerType::RegionEnter,
            Trigger::OnRegionExit { .. } => TriggerType::RegionExit,
            Trigger::OnMissionStep { .. } => TriggerType::MissionStep,
            Trigger::OnItemAcquired { .. } => TriggerType::ItemAcquired,
            Trigger::OnTimer { .. } => TriggerType::Timer,
            Trigger::OnCustomEvent { .. } => TriggerType::CustomEvent,
            Trigger::OnPlayerLoaded { .. } => TriggerType::PlayerLoaded,
            Trigger::OnDialogOpen { .. } => TriggerType::DialogOpen,
            Trigger::OnDialogChoice { .. } => TriggerType::DialogChoice,
            Trigger::OnInteractTag { .. } => TriggerType::InteractTag,
            Trigger::OnInteractTemplate { .. } => TriggerType::InteractTemplate,
            Trigger::OnItemUse { .. } => TriggerType::ItemUse,
            Trigger::OnItemEquipped { .. } => TriggerType::ItemEquipped,
            Trigger::OnTeleportIn { .. } => TriggerType::TeleportIn,
            Trigger::OnEffectInit => TriggerType::EffectInit,
            Trigger::OnEffectPulseBegin => TriggerType::EffectPulseBegin,
            Trigger::OnEffectPulseEnd => TriggerType::EffectPulseEnd,
            Trigger::OnEffectRemoved => TriggerType::EffectRemoved,
            Trigger::OnMissionCompleted { .. } => TriggerType::MissionCompleted,
            Trigger::OnDialogSetOpen { .. } => TriggerType::DialogSetOpen,
            Trigger::OnMissionAccepted { .. } => TriggerType::MissionAccepted,
            Trigger::OnPlayerEnteredCover { .. } => TriggerType::PlayerEnteredCover,
            Trigger::OnPlayerLeftCover { .. } => TriggerType::PlayerLeftCover,
            Trigger::OnPlayerInCoverDuration { .. } => TriggerType::PlayerInCoverDuration,
            Trigger::OnNpcFlanked { .. } => TriggerType::NpcFlanked,
        }
    }

    /// Returns `true` if this trigger matches the given runtime event.
    pub fn matches(&self, event: &TriggerEvent) -> bool {
        if self.trigger_type() != event.trigger_type {
            return false;
        }

        match self {
            Trigger::OnEntityCreated { entity_type }
            | Trigger::OnEntityDestroyed { entity_type } => match entity_type {
                Some(expected) => event
                    .params
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .is_some_and(|actual| actual == expected),
                None => true,
            },
            Trigger::OnEntityDeath {
                entity_type,
                entity_tag,
            } => {
                // If entity_tag is set, match on tag (DB entity_dead_tag pattern)
                if let Some(tag) = entity_tag {
                    return event
                        .params
                        .get("entity_tag")
                        .and_then(|v| v.as_str())
                        .is_some_and(|actual| actual == tag);
                }
                // Otherwise match on entity_type (original pattern)
                match entity_type {
                    Some(expected) => event
                        .params
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .is_some_and(|actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnAbilityUsed { ability_id } => match ability_id {
                Some(expected) => {
                    event.params.get("ability_id").and_then(|v| v.as_i64())
                        == Some(*expected as i64)
                }
                None => true,
            },
            Trigger::OnInteraction { interaction_type } => match interaction_type {
                Some(expected) => event
                    .params
                    .get("interaction_type")
                    .and_then(|v| v.as_str())
                    .is_some_and(|actual| actual == expected),
                None => true,
            },
            Trigger::OnRegionEnter { region_key } => event
                .params
                .get("region_key")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == region_key),
            Trigger::OnRegionExit { region_key } => event
                .params
                .get("region_key")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == region_key),
            Trigger::OnMissionStep { mission_id, step } => {
                let mission_match = event.params.get("mission_id").and_then(|v| v.as_i64())
                    == Some(*mission_id as i64);
                let step_match =
                    event.params.get("step").and_then(|v| v.as_i64()) == Some(*step as i64);
                mission_match && step_match
            }
            Trigger::OnItemAcquired { item_id } => match item_id {
                Some(expected) => {
                    event.params.get("item_id").and_then(|v| v.as_i64()) == Some(*expected as i64)
                }
                None => true,
            },
            Trigger::OnTimer { timer_name } => event
                .params
                .get("timer_name")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == timer_name),
            Trigger::OnCustomEvent { event_name } => event
                .params
                .get("event_name")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == event_name),
            Trigger::OnPlayerLoaded { world_name } => match world_name {
                Some(expected) => event
                    .params
                    .get("world_name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|actual| actual == expected),
                None => true,
            },
            Trigger::OnDialogOpen { dialog_id } | Trigger::OnDialogChoice { dialog_id } => {
                event.params.get("dialog_id").and_then(|v| v.as_i64()) == Some(*dialog_id as i64)
            }
            Trigger::OnInteractTag { entity_tag } => event
                .params
                .get("entity_tag")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == entity_tag),
            Trigger::OnInteractTemplate { template_name } => event
                .params
                .get("template_name")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == template_name),
            Trigger::OnItemUse { item_id } => {
                event.params.get("item_id").and_then(|v| v.as_i64()) == Some(*item_id as i64)
            }
            Trigger::OnItemEquipped { item_id } => match item_id {
                Some(expected) => {
                    event.params.get("item_id").and_then(|v| v.as_i64()) == Some(*expected as i64)
                }
                None => true,
            },
            Trigger::OnTeleportIn { region_id } => {
                event.params.get("region_id").and_then(|v| v.as_i64()) == Some(*region_id as i64)
            }
            // Unit triggers match any event of the right type
            Trigger::OnEffectInit
            | Trigger::OnEffectPulseBegin
            | Trigger::OnEffectPulseEnd
            | Trigger::OnEffectRemoved => true,
            Trigger::OnMissionCompleted { mission_id } => {
                event.params.get("mission_id").and_then(|v| v.as_i64()) == Some(*mission_id as i64)
            }
            Trigger::OnDialogSetOpen { dialog_set_name } => event
                .params
                .get("dialog_set_name")
                .and_then(|v| v.as_str())
                .is_some_and(|actual| actual == dialog_set_name),
            Trigger::OnMissionAccepted { mission_id } => {
                event.params.get("mission_id").and_then(|v| v.as_i64()) == Some(*mission_id as i64)
            }
            Trigger::OnPlayerEnteredCover { cover_set_id }
            | Trigger::OnPlayerLeftCover { cover_set_id } => match cover_set_id {
                Some(expected) => {
                    event.params.get("cover_set_id").and_then(|v| v.as_i64())
                        == Some(*expected as i64)
                }
                None => true,
            },
            Trigger::OnPlayerInCoverDuration {
                cover_set_id,
                seconds,
            } => {
                let seconds_match =
                    event.params.get("seconds").and_then(|v| v.as_i64()) == Some(*seconds as i64);
                let set_match = match cover_set_id {
                    Some(expected) => {
                        event.params.get("cover_set_id").and_then(|v| v.as_i64())
                            == Some(*expected as i64)
                    }
                    None => true,
                };
                seconds_match && set_match
            }
            Trigger::OnNpcFlanked { npc_template } => match npc_template {
                Some(expected) => event
                    .params
                    .get("npc_template")
                    .and_then(|v| v.as_str())
                    .is_some_and(|actual| actual == expected),
                None => true,
            },
        }
    }
}
