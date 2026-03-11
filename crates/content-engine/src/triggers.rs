//! Trigger definitions and event matching.
//!
//! Triggers define _when_ a chain activates. Each chain has exactly one trigger.
//! When a game event fires, the chain engine matches it against registered
//! triggers to determine which chains should be evaluated.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use cimmeria_common::EntityId;

/// A trigger attached to a chain, defining when the chain activates.
///
/// Each variant may carry optional filter fields that narrow the match. For
/// example, `OnEntityCreated { entity_type: Some("SGWMob") }` only fires when
/// an `SGWMob` is created, while `OnEntityCreated { entity_type: None }` fires
/// for every entity creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    /// Fires when an entity is created in the world.
    OnEntityCreated { entity_type: Option<String> },

    /// Fires when an entity is destroyed/removed from the world.
    OnEntityDestroyed { entity_type: Option<String> },

    /// Fires when an entity dies (health reaches zero).
    /// `entity_tag` filters by a specific tagged NPC (DB `entity_dead_tag`).
    OnEntityDeath {
        entity_type: Option<String>,
        #[serde(default)]
        entity_tag: Option<String>,
    },

    /// Fires when an ability is used by any entity.
    OnAbilityUsed { ability_id: Option<i32> },

    /// Fires when a player interacts with an entity (NPC dialog, object use).
    OnInteraction { interaction_type: Option<String> },

    /// Fires when an entity enters a spatial region.
    /// Uses string keys like `"Castle_Cellblock.Region2"`.
    OnRegionEnter { region_key: String },

    /// Fires when an entity exits a spatial region.
    OnRegionExit { region_key: String },

    /// Fires when a mission reaches a specific step.
    OnMissionStep { mission_id: i32, step: i32 },

    /// Fires when an item is acquired by an entity.
    OnItemAcquired { item_id: Option<i32> },

    /// Fires when a named timer expires.
    OnTimer { timer_name: String },

    /// Fires on an arbitrary named event (extensibility hook).
    OnCustomEvent { event_name: String },

    /// Fires when a player completes world entry (mapLoaded).
    OnPlayerLoaded { world_name: Option<String> },

    /// Fires when the server sends `onDialogDisplay` to a player.
    OnDialogOpen { dialog_id: i32 },

    /// Fires when a player selects a choice/button in a dialog.
    OnDialogChoice { dialog_id: i32 },

    /// Fires when a player interacts with a tagged NPC or object.
    OnInteractTag { entity_tag: String },

    /// Fires when a player interacts with an entity from a named template.
    OnInteractTemplate { template_name: String },

    /// Fires when a player uses an inventory item.
    OnItemUse { item_id: i32 },

    /// Fires when a player arrives via teleporter at a destination region.
    OnTeleportIn { region_id: i32 },

    /// Fires when an effect is first initialized on an entity.
    OnEffectInit,

    /// Fires at the start of each pulse of a periodic effect.
    OnEffectPulseBegin,

    /// Fires at the end of each pulse of a periodic effect.
    OnEffectPulseEnd,

    /// Fires when an effect is removed from an entity.
    OnEffectRemoved,

    /// Fires when a mission is completed.
    OnMissionCompleted { mission_id: i32 },

    /// Fires when a dialog set is opened for a player.
    OnDialogSetOpen { dialog_set_name: String },
}

/// Runtime event payload passed to the chain engine when a game event occurs.
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    /// Discriminant identifying which trigger type this event corresponds to.
    pub trigger_type: TriggerType,

    /// The entity that caused the event.
    pub source_entity: Option<EntityId>,

    /// The entity the event targets.
    pub target_entity: Option<EntityId>,

    /// Event-specific parameters.
    pub params: HashMap<String, serde_json::Value>,
}

/// Discriminant enum for trigger types, used as a grouping key in the chain
/// engine's index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TriggerType {
    EntityCreated,
    EntityDestroyed,
    EntityDeath,
    AbilityUsed,
    Interaction,
    RegionEnter,
    RegionExit,
    MissionStep,
    ItemAcquired,
    Timer,
    CustomEvent,
    PlayerLoaded,
    DialogOpen,
    DialogChoice,
    InteractTag,
    InteractTemplate,
    ItemUse,
    TeleportIn,
    EffectInit,
    EffectPulseBegin,
    EffectPulseEnd,
    EffectRemoved,
    MissionCompleted,
    DialogSetOpen,
}

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
            Trigger::OnTeleportIn { .. } => TriggerType::TeleportIn,
            Trigger::OnEffectInit => TriggerType::EffectInit,
            Trigger::OnEffectPulseBegin => TriggerType::EffectPulseBegin,
            Trigger::OnEffectPulseEnd => TriggerType::EffectPulseEnd,
            Trigger::OnEffectRemoved => TriggerType::EffectRemoved,
            Trigger::OnMissionCompleted { .. } => TriggerType::MissionCompleted,
            Trigger::OnDialogSetOpen { .. } => TriggerType::DialogSetOpen,
        }
    }

    /// Returns `true` if this trigger matches the given runtime event.
    pub fn matches(&self, event: &TriggerEvent) -> bool {
        if self.trigger_type() != event.trigger_type {
            return false;
        }

        match self {
            Trigger::OnEntityCreated { entity_type } | Trigger::OnEntityDestroyed { entity_type } => {
                match entity_type {
                    Some(expected) => event.params.get("entity_type")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnEntityDeath { entity_type, entity_tag } => {
                // If entity_tag is set, match on tag (DB entity_dead_tag pattern)
                if let Some(tag) = entity_tag {
                    return event.params.get("entity_tag")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == tag);
                }
                // Otherwise match on entity_type (original pattern)
                match entity_type {
                    Some(expected) => event.params.get("entity_type")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnAbilityUsed { ability_id } => {
                match ability_id {
                    Some(expected) => event.params.get("ability_id")
                        .and_then(|v| v.as_i64())
                        .map_or(false, |actual| actual == *expected as i64),
                    None => true,
                }
            }
            Trigger::OnInteraction { interaction_type } => {
                match interaction_type {
                    Some(expected) => event.params.get("interaction_type")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnRegionEnter { region_key } => {
                event.params.get("region_key")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == region_key)
            }
            Trigger::OnRegionExit { region_key } => {
                event.params.get("region_key")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == region_key)
            }
            Trigger::OnMissionStep { mission_id, step } => {
                let mission_match = event.params.get("mission_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *mission_id as i64);
                let step_match = event.params.get("step")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *step as i64);
                mission_match && step_match
            }
            Trigger::OnItemAcquired { item_id } => {
                match item_id {
                    Some(expected) => event.params.get("item_id")
                        .and_then(|v| v.as_i64())
                        .map_or(false, |actual| actual == *expected as i64),
                    None => true,
                }
            }
            Trigger::OnTimer { timer_name } => {
                event.params.get("timer_name")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == timer_name)
            }
            Trigger::OnCustomEvent { event_name } => {
                event.params.get("event_name")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == event_name)
            }
            Trigger::OnPlayerLoaded { world_name } => {
                match world_name {
                    Some(expected) => event.params.get("world_name")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnDialogOpen { dialog_id } | Trigger::OnDialogChoice { dialog_id } => {
                event.params.get("dialog_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *dialog_id as i64)
            }
            Trigger::OnInteractTag { entity_tag } => {
                event.params.get("entity_tag")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == entity_tag)
            }
            Trigger::OnInteractTemplate { template_name } => {
                event.params.get("template_name")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == template_name)
            }
            Trigger::OnItemUse { item_id } => {
                event.params.get("item_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *item_id as i64)
            }
            Trigger::OnTeleportIn { region_id } => {
                event.params.get("region_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *region_id as i64)
            }
            // Unit triggers match any event of the right type
            Trigger::OnEffectInit | Trigger::OnEffectPulseBegin |
            Trigger::OnEffectPulseEnd | Trigger::OnEffectRemoved => true,
            Trigger::OnMissionCompleted { mission_id } => {
                event.params.get("mission_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *mission_id as i64)
            }
            Trigger::OnDialogSetOpen { dialog_set_name } => {
                event.params.get("dialog_set_name")
                    .and_then(|v| v.as_str())
                    .map_or(false, |actual| actual == dialog_set_name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(trigger_type: TriggerType, params: Vec<(&str, serde_json::Value)>) -> TriggerEvent {
        TriggerEvent {
            trigger_type,
            source_entity: None,
            target_entity: None,
            params: params.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    #[test]
    fn trigger_type_discriminant() {
        let t = Trigger::OnEntityCreated { entity_type: None };
        assert_eq!(t.trigger_type(), TriggerType::EntityCreated);

        let t = Trigger::OnTimer { timer_name: "respawn".to_string() };
        assert_eq!(t.trigger_type(), TriggerType::Timer);

        let t = Trigger::OnDialogChoice { dialog_id: 100 };
        assert_eq!(t.trigger_type(), TriggerType::DialogChoice);

        assert_eq!(Trigger::OnEffectInit.trigger_type(), TriggerType::EffectInit);
    }

    #[test]
    fn wildcard_trigger_matches_any() {
        let trigger = Trigger::OnEntityCreated { entity_type: None };
        let event = make_event(TriggerType::EntityCreated, vec![
            ("entity_type", serde_json::Value::String("SGWMob".to_string())),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn filtered_trigger_matches_correct_type() {
        let trigger = Trigger::OnEntityCreated {
            entity_type: Some("SGWMob".to_string()),
        };
        let event = make_event(TriggerType::EntityCreated, vec![
            ("entity_type", serde_json::Value::String("SGWMob".to_string())),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn filtered_trigger_rejects_wrong_type() {
        let trigger = Trigger::OnEntityCreated {
            entity_type: Some("SGWMob".to_string()),
        };
        let event = make_event(TriggerType::EntityCreated, vec![
            ("entity_type", serde_json::Value::String("SGWPlayer".to_string())),
        ]);
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
        let trigger = Trigger::OnRegionEnter { region_key: "Castle_Cellblock.Region2".to_string() };
        let event = make_event(TriggerType::RegionEnter, vec![
            ("region_key", serde_json::json!("Castle_Cellblock.Region2")),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn region_enter_rejects_wrong_key() {
        let trigger = Trigger::OnRegionEnter { region_key: "Castle_Cellblock.Region2".to_string() };
        let event = make_event(TriggerType::RegionEnter, vec![
            ("region_key", serde_json::json!("SGC_W1.Region1")),
        ]);
        assert!(!trigger.matches(&event));
    }

    #[test]
    fn mission_step_requires_both_fields() {
        let trigger = Trigger::OnMissionStep { mission_id: 10, step: 3 };

        let event = make_event(TriggerType::MissionStep, vec![
            ("mission_id", serde_json::json!(10)),
            ("step", serde_json::json!(3)),
        ]);
        assert!(trigger.matches(&event));

        let event = make_event(TriggerType::MissionStep, vec![
            ("mission_id", serde_json::json!(10)),
            ("step", serde_json::json!(1)),
        ]);
        assert!(!trigger.matches(&event));
    }

    #[test]
    fn custom_event_matches_name() {
        let trigger = Trigger::OnCustomEvent { event_name: "boss_phase_2".to_string() };
        let event = make_event(TriggerType::CustomEvent, vec![
            ("event_name", serde_json::Value::String("boss_phase_2".to_string())),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn dialog_choice_matches() {
        let trigger = Trigger::OnDialogChoice { dialog_id: 5021 };
        let event = make_event(TriggerType::DialogChoice, vec![
            ("dialog_id", serde_json::json!(5021)),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn interact_tag_matches() {
        let trigger = Trigger::OnInteractTag { entity_tag: "ArmYourself_FrostBody".to_string() };
        let event = make_event(TriggerType::InteractTag, vec![
            ("entity_tag", serde_json::json!("ArmYourself_FrostBody")),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn entity_death_by_tag() {
        let trigger = Trigger::OnEntityDeath {
            entity_type: None,
            entity_tag: Some("Hallway01_Guard".to_string()),
        };
        let event = make_event(TriggerType::EntityDeath, vec![
            ("entity_tag", serde_json::json!("Hallway01_Guard")),
        ]);
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
        let event = make_event(TriggerType::MissionCompleted, vec![
            ("mission_id", serde_json::json!(1559)),
        ]);
        assert!(trigger.matches(&event));
    }
}
