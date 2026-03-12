//! Action executors for chain effects.
//!
//! Actions are the "do" part of a chain -- they modify game state when a
//! trigger fires and all conditions pass. The action types cover the breadth
//! of the original Python scripting layer: XP/item grants, effects,
//! teleportation, spawning, dialog, missions, loot, timers, and extensibility
//! hooks.
//!
//! Actions are resolved by the engine but executed by the caller (CellService),
//! which has access to game state. The `execute()` method on each action is
//! preserved for backward compatibility but should not be called for DB-driven
//! chains — use `ChainEngine::resolve_event()` instead.

use serde::{Deserialize, Serialize};

use crate::context::ExecutionContext;

/// An action to execute when a chain's trigger fires and conditions pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    // ── Original generic actions ──────────────────────────────────────────

    /// Award experience points to the source entity.
    GrantXP { amount: u64 },

    /// Add items to the source entity's inventory.
    GrantItem {
        item_id: i32,
        count: i32,
        #[serde(default)]
        container_id: Option<i32>,
    },

    /// Remove items from the source entity's inventory.
    RemoveItem { item_id: i32, count: i32 },

    /// Apply a timed or permanent effect to the source entity.
    ApplyEffect { effect_id: i32, duration_secs: Option<f32> },

    /// Remove an active effect from the source entity.
    RemoveEffect { effect_id: i32 },

    /// Teleport the source entity to a position in a target space.
    Teleport { space_id: i32, position: [f32; 3] },

    /// Spawn a new entity from a template at the given position.
    SpawnEntity { template_id: i32, position: [f32; 3] },

    /// Despawn the target entity (from context).
    DespawnEntity,

    /// Open a dialog set for the source entity (player).
    StartDialog { dialog_set_id: i32 },

    /// Advance the specified mission to its next step (legacy; prefer AcceptMission/AdvanceStep).
    AdvanceMission { mission_id: i32 },

    /// Mark the specified mission as complete.
    CompleteMission { mission_id: i32 },

    /// Play an animation on the source or target entity.
    PlayAnimation { animation: String },

    /// Play a sound effect at the source entity's position.
    PlaySound { sound: String },

    /// Send a text message on a named channel.
    SendMessage { channel: String, message: String },

    /// Modify a property on the source entity.
    ModifyProperty {
        property: String,
        operation: PropertyOp,
        value: serde_json::Value,
    },

    /// Roll a loot table and grant results.
    RollLootTable { table_id: i32 },

    /// Spawn a loot bag entity at the given position.
    SpawnLootBag { position: [f32; 3] },

    /// Start a named timer.
    StartTimer {
        name: String,
        duration_secs: f32,
        repeat: bool,
    },

    /// Cancel a running named timer.
    CancelTimer { name: String },

    /// Trigger another chain by ID.
    TriggerChain { chain_id: i64 },

    /// Execute a custom handler function.
    ExecuteCustom {
        handler: String,
        params: serde_json::Value,
    },

    // ── DB-driven action types ────────────────────────────────────────────

    /// Accept and start tracking a mission.
    AcceptMission { mission_id: i32 },

    /// Display a specific dialog to the player.
    DisplayDialog { dialog_id: i32 },

    /// Add a dialog set entry to an NPC.
    AddDialogSet {
        dialog_set_id: i32,
        slot: i32,
        mission_id: Option<i32>,
    },

    /// Remove a dialog set entry from an NPC.
    RemoveDialogSet {
        dialog_set_id: i32,
        slot: i32,
    },

    /// Play a cinematic sequence/cutscene.
    PlaySequence { sequence_id: i32 },

    /// Advance a mission to a specific step.
    AdvanceStep {
        mission_id: i32,
        step_id: i32,
    },

    /// Set or modify interaction type flags on a tagged entity.
    SetInteractionType {
        entity_tag: String,
        operation: String,
        mask: i64,
    },

    /// Start a minigame for the player.
    StartMinigame {
        minigame_type: String,
        on_victory_chains: Vec<i64>,
    },

    /// Set the aggression level on a tagged NPC.
    SetAggression {
        entity_tag: String,
        level: i32,
    },

    /// Destroy a tagged entity (remove from world).
    DestroyTaggedEntity { entity_tag: String },

    /// Activate a transporter to move the player to a region.
    TriggerTransporter { region_id: i32 },

    /// Send a system message to the player.
    SystemMessage { message_id: i32 },

    /// Apply QR combat damage to a stat.
    QrCombatDamage {
        stat_id: i32,
        source_id: i32,
        amount_nvp: String,
    },

    /// Change a stat on the entity.
    ChangeStat {
        stat_id: i32,
        min: Option<i32>,
        max: Option<i32>,
        use_ammo_stat: Option<bool>,
        set_to_max: Option<bool>,
    },

    /// Abandon an active mission.
    AbandonMission { mission_id: i32 },

    /// Fail a specific objective within a mission.
    FailObjective {
        mission_id: i32,
        objective_id: i32,
    },

    /// Increment a named counter.
    IncrementCounter {
        counter_name: String,
        amount: i32,
    },

    /// Reset a named counter to zero.
    ResetCounter { counter_name: String },

    /// Complete a specific objective within a mission.
    CompleteObjective {
        mission_id: i32,
        objective_id: i32,
    },

    /// Set the visibility of a tagged entity.
    SetVisible {
        entity_tag: String,
        visible: bool,
    },

    /// Move a tagged entity or the player to a destination.
    MoveEntity {
        entity_tag: Option<String>,
        destination: [f32; 3],
        world: Option<String>,
        use_player: Option<bool>,
    },

    // ── Space script action types ────────────────────────────────────────

    /// Animated NPC pathing — moves a tagged entity along a path with walk animation.
    /// Unlike MoveEntity (instant teleport), this triggers movement over time.
    MoveWaypoint {
        entity_tag: String,
        destination: [f32; 3],
        speed: f32,
    },

    /// Equip an item to an equipment slot (typically Bandolier bag_id=3).
    SetActiveSlot {
        bag_id: i32,
        slot: i32,
    },

    /// Force-fire an ability on an entity (or self if entity_tag is None).
    LaunchAbility {
        ability_id: i32,
        entity_tag: Option<String>,
    },
}

/// Arithmetic/assignment operation for [`Action::ModifyProperty`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyOp {
    Set,
    Add,
    Subtract,
    Multiply,
}

/// Result of executing a single action.
#[derive(Debug, Clone)]
pub enum ActionResult {
    /// The action completed successfully.
    Success,
    /// The action failed with a descriptive error message.
    Error(String),
    /// The action requests that another chain be evaluated.
    ChainTrigger(i64),
}

impl Action {
    /// Execute this action against the given execution context.
    ///
    /// Most variants are `todo!()` stubs — real execution happens in the
    /// CellService via `resolve_event()` + `execute_actions()`.
    pub fn execute(&self, ctx: &mut ExecutionContext) -> ActionResult {
        let _ = ctx;
        match self {
            Action::TriggerChain { chain_id } => ActionResult::ChainTrigger(*chain_id),
            _ => {
                // All other actions are executed by the CellService via resolve_event().
                // Calling execute() directly on them is not supported for DB-driven chains.
                ActionResult::Error(format!("Action {:?} must be executed via resolve_event()", self))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_chain_returns_chain_trigger_result() {
        let action = Action::TriggerChain { chain_id: 42 };
        let mut ctx = ExecutionContext::new();
        match action.execute(&mut ctx) {
            ActionResult::ChainTrigger(id) => assert_eq!(id, 42),
            other => panic!("Expected ChainTrigger(42), got {:?}", other),
        }
    }

    #[test]
    fn action_serialization_roundtrip() {
        let action = Action::GrantXP { amount: 500 };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        let _ = format!("{:?}", deserialized);
    }

    #[test]
    fn property_op_serialization_roundtrip() {
        let ops = vec![
            PropertyOp::Set,
            PropertyOp::Add,
            PropertyOp::Subtract,
            PropertyOp::Multiply,
        ];
        for op in &ops {
            let json = serde_json::to_string(op).unwrap();
            let deserialized: PropertyOp = serde_json::from_str(&json).unwrap();
            let _ = format!("{:?}", deserialized);
        }
    }

    #[test]
    fn complex_action_serialization() {
        let action = Action::ModifyProperty {
            property: "health".to_string(),
            operation: PropertyOp::Subtract,
            value: serde_json::json!(25),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("health"));
        assert!(json.contains("Subtract"));
        assert!(json.contains("25"));
    }

    #[test]
    fn teleport_action_serialization() {
        let action = Action::Teleport {
            space_id: 7,
            position: [100.0, 200.0, 300.0],
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::Teleport { space_id, position } => {
                assert_eq!(space_id, 7);
                assert_eq!(position, [100.0, 200.0, 300.0]);
            }
            _ => panic!("Expected Teleport variant"),
        }
    }

    #[test]
    fn grant_item_with_container() {
        let action = Action::GrantItem { item_id: 55, count: 1, container_id: Some(3) };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("container_id"));
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::GrantItem { item_id, count, container_id } => {
                assert_eq!(item_id, 55);
                assert_eq!(count, 1);
                assert_eq!(container_id, Some(3));
            }
            _ => panic!("Expected GrantItem"),
        }
    }

    #[test]
    fn grant_item_without_container_defaults_none() {
        let json = r#"{"GrantItem": {"item_id": 55, "count": 1}}"#;
        let deserialized: Action = serde_json::from_str(json).unwrap();
        match deserialized {
            Action::GrantItem { container_id, .. } => assert_eq!(container_id, None),
            _ => panic!("Expected GrantItem"),
        }
    }

    #[test]
    fn move_waypoint_serialization() {
        let action = Action::MoveWaypoint {
            entity_tag: "NID_Guard_01".to_string(),
            destination: [-296.715, 68.511, -166.125],
            speed: 1.5,
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::MoveWaypoint { entity_tag, destination, speed } => {
                assert_eq!(entity_tag, "NID_Guard_01");
                assert_eq!(destination, [-296.715, 68.511, -166.125]);
                assert!((speed - 1.5).abs() < f32::EPSILON);
            }
            _ => panic!("Expected MoveWaypoint"),
        }
    }

    #[test]
    fn set_active_slot_serialization() {
        let action = Action::SetActiveSlot { bag_id: 3, slot: 0 };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::SetActiveSlot { bag_id, slot } => {
                assert_eq!(bag_id, 3);
                assert_eq!(slot, 0);
            }
            _ => panic!("Expected SetActiveSlot"),
        }
    }

    #[test]
    fn launch_ability_serialization() {
        let action = Action::LaunchAbility {
            ability_id: 1372,
            entity_tag: Some("NID_Guard_01".to_string()),
        };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::LaunchAbility { ability_id, entity_tag } => {
                assert_eq!(ability_id, 1372);
                assert_eq!(entity_tag, Some("NID_Guard_01".to_string()));
            }
            _ => panic!("Expected LaunchAbility"),
        }
    }

    #[test]
    fn launch_ability_without_entity_tag() {
        let json = r#"{"LaunchAbility": {"ability_id": 500}}"#;
        let deserialized: Action = serde_json::from_str(json).unwrap();
        match deserialized {
            Action::LaunchAbility { ability_id, entity_tag } => {
                assert_eq!(ability_id, 500);
                assert_eq!(entity_tag, None);
            }
            _ => panic!("Expected LaunchAbility"),
        }
    }

    #[test]
    fn accept_mission_serialization() {
        let action = Action::AcceptMission { mission_id: 622 };
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::AcceptMission { mission_id } => assert_eq!(mission_id, 622),
            _ => panic!("Expected AcceptMission"),
        }
    }
}
