//! Trigger definitions and event matching.
//!
//! Triggers define _when_ a chain activates. Each chain has exactly one trigger.
//! When a game event fires, the chain engine matches it against registered
//! triggers to determine which chains should be evaluated.
//!
//! The 11 trigger types cover the core gameplay events from the original Python
//! scripting layer: entity lifecycle, combat, interaction, spatial, mission
//! progression, inventory, timers, and arbitrary custom events.

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
    OnEntityDeath { entity_type: Option<String> },

    /// Fires when an ability is used by any entity.
    OnAbilityUsed { ability_id: Option<i32> },

    /// Fires when a player interacts with an entity (NPC dialog, object use).
    OnInteraction { interaction_type: Option<String> },

    /// Fires when an entity enters a spatial region.
    OnRegionEnter { region_id: i32 },

    /// Fires when an entity exits a spatial region.
    OnRegionExit { region_id: i32 },

    /// Fires when a mission reaches a specific step.
    OnMissionStep { mission_id: i32, step: i32 },

    /// Fires when an item is acquired by an entity.
    OnItemAcquired { item_id: Option<i32> },

    /// Fires when a named timer expires.
    OnTimer { timer_name: String },

    /// Fires on an arbitrary named event (extensibility hook).
    OnCustomEvent { event_name: String },
}

/// Runtime event payload passed to the chain engine when a game event occurs.
///
/// Contains the trigger type discriminant, optional source/target entities,
/// and a free-form parameter map for event-specific data.
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    /// Discriminant identifying which trigger type this event corresponds to.
    pub trigger_type: TriggerType,

    /// The entity that caused the event (e.g., the player who used an ability).
    pub source_entity: Option<EntityId>,

    /// The entity the event targets (e.g., the NPC being interacted with).
    pub target_entity: Option<EntityId>,

    /// Event-specific parameters. Keys and value types depend on the trigger:
    ///
    /// - `OnEntityCreated`: `"entity_type"` (string)
    /// - `OnAbilityUsed`: `"ability_id"` (number)
    /// - `OnRegionEnter`/`OnRegionExit`: `"region_id"` (number)
    /// - `OnMissionStep`: `"mission_id"` (number), `"step"` (number)
    /// - `OnItemAcquired`: `"item_id"` (number)
    /// - `OnTimer`: `"timer_name"` (string)
    /// - `OnCustomEvent`: `"event_name"` (string)
    /// - `OnInteraction`: `"interaction_type"` (string)
    pub params: HashMap<String, serde_json::Value>,
}

/// Discriminant enum for trigger types, used as a grouping key in the chain
/// engine's index. Unlike [`Trigger`], this carries no filter data -- it is
/// purely a type tag for fast lookup.
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
        }
    }

    /// Returns `true` if this trigger matches the given runtime event.
    ///
    /// Matching is two-phase: first the trigger type discriminant must match,
    /// then any optional filter fields are checked against the event's params.
    /// A `None` filter is a wildcard that matches any value.
    pub fn matches(&self, event: &TriggerEvent) -> bool {
        if self.trigger_type() != event.trigger_type {
            return false;
        }

        match self {
            Trigger::OnEntityCreated { entity_type } => {
                match entity_type {
                    Some(expected) => event.params.get("entity_type")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnEntityDestroyed { entity_type } => {
                match entity_type {
                    Some(expected) => event.params.get("entity_type")
                        .and_then(|v| v.as_str())
                        .map_or(false, |actual| actual == expected),
                    None => true,
                }
            }
            Trigger::OnEntityDeath { entity_type } => {
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
            Trigger::OnRegionEnter { region_id } => {
                event.params.get("region_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *region_id as i64)
            }
            Trigger::OnRegionExit { region_id } => {
                event.params.get("region_id")
                    .and_then(|v| v.as_i64())
                    .map_or(false, |actual| actual == *region_id as i64)
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
    fn region_enter_matches_correct_region() {
        let trigger = Trigger::OnRegionEnter { region_id: 42 };
        let event = make_event(TriggerType::RegionEnter, vec![
            ("region_id", serde_json::json!(42)),
        ]);
        assert!(trigger.matches(&event));
    }

    #[test]
    fn region_enter_rejects_wrong_region() {
        let trigger = Trigger::OnRegionEnter { region_id: 42 };
        let event = make_event(TriggerType::RegionEnter, vec![
            ("region_id", serde_json::json!(99)),
        ]);
        assert!(!trigger.matches(&event));
    }

    #[test]
    fn mission_step_requires_both_fields() {
        let trigger = Trigger::OnMissionStep { mission_id: 10, step: 3 };

        // Both match
        let event = make_event(TriggerType::MissionStep, vec![
            ("mission_id", serde_json::json!(10)),
            ("step", serde_json::json!(3)),
        ]);
        assert!(trigger.matches(&event));

        // Wrong step
        let event = make_event(TriggerType::MissionStep, vec![
            ("mission_id", serde_json::json!(10)),
            ("step", serde_json::json!(1)),
        ]);
        assert!(!trigger.matches(&event));

        // Missing step
        let event = make_event(TriggerType::MissionStep, vec![
            ("mission_id", serde_json::json!(10)),
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
    fn timer_trigger_matches_name() {
        let trigger = Trigger::OnTimer { timer_name: "respawn_wave".to_string() };
        let event = make_event(TriggerType::Timer, vec![
            ("timer_name", serde_json::Value::String("respawn_wave".to_string())),
        ]);
        assert!(trigger.matches(&event));
    }
}
