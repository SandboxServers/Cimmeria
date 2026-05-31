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
    ApplyEffect {
        effect_id: i32,
        duration_secs: Option<f32>,
    },

    /// Remove an active effect from the source entity.
    RemoveEffect { effect_id: i32 },

    /// Teleport the source entity to a position in a target space.
    Teleport { space_id: i32, position: [f32; 3] },

    /// Cross-world teleport: tear down the player's cell entity on this
    /// world and re-create on `world_name` at `position` via the
    /// `CellToBaseMsg::GateTravel` pipeline (same flow used by stargate
    /// dial). `destination_ring_id` is `None` so base does not send
    /// `BaseToCellMsg::AdvanceRingDestination` — there's no destination
    /// ring FSM to advance. Use this for chain-driven cross-world hops
    /// where a ring ceremony is unnecessary or unavailable (e.g., the
    /// mission 688 armory exit, where the Castle map has no ring
    /// transporter prefab actor in its kismet).
    CrossWorldTeleport {
        world_name: String,
        position: [f32; 3],
    },

    /// Spawn a new entity from a template at the given position.
    SpawnEntity {
        template_id: i32,
        position: [f32; 3],
    },

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
    RemoveDialogSet { dialog_set_id: i32, slot: i32 },

    /// Play a cinematic sequence/cutscene.
    PlaySequence { sequence_id: i32 },

    /// Advance a mission to a specific step.
    AdvanceStep { mission_id: i32, step_id: i32 },

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
    SetAggression { entity_tag: String, level: i32 },

    /// Push a tagged NPC into `AiState::Investigating` with the given
    /// world-space point of interest. The NPC pathfinds to the POI,
    /// dwells `investigate_dwell_secs`, and returns to `AiState::Idle`.
    /// Threat preemption converts to Fighting; the POI is preserved
    /// so the post-fight return to Idle could route back to it (not
    /// implemented in this PR — content authors fire a fresh
    /// `SetNpcPoi` instead).
    SetNpcPoi {
        entity_tag: String,
        x: f32,
        y: f32,
        z: f32,
    },

    /// Set or clear the follow target for a tagged NPC. When
    /// `target_tag` resolves to an entity, the NPC transitions to
    /// `AiState::Follow` and maintains the distance band defined by
    /// `follow_min_distance` / `follow_max_distance` on the template.
    /// Passing `target_tag = None` clears the follow state and
    /// returns the NPC to Idle.
    ///
    /// Threat preemption converts Follow → Fighting; the follow
    /// target persists on the entity but the post-fight return is
    /// to Idle (re-fire the action if a continued follow is
    /// desired).
    SetFollowTarget {
        entity_tag: String,
        target_tag: Option<String>,
    },

    /// Push a tagged NPC into a specific AI state. Supports the
    /// terminal / scripted states: `Despawning`, `Submit`, `Error`,
    /// and `Idle` (for cleanup). Other states should be reached via
    /// their behavior-specific actions (`SetNpcPoi` for Investigating,
    /// `SetFollowTarget` for Follow, etc.) so the per-state scratch
    /// fields are populated correctly.
    ///
    /// - `Despawning` → AI tick removes the entity from the space
    ///   on the next pass. Witnesses get an AoI-left event.
    /// - `Submit` → clears combat state, broadcasts movement-type
    ///   None; NPC sits inert until destroyed or transitioned.
    /// - `Error` → halts AI ticking on the NPC, logs the inconsistency.
    ///   Used by `enterErrorAIState` slash commands and by the AI tick
    ///   itself when it detects unrecoverable state.
    /// - `Idle` → clean fallback that lets the AI tick re-route.
    ///
    /// Other state values (Fighting/Leashing/etc.) are rejected with
    /// a warn log — those are owned by the runtime, not content.
    SetNpcAiState {
        entity_tag: String,
        state: NpcAiStateAction,
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
    ///
    /// Application order in the executor: `min` / `max` (bounds), then
    /// `set_to_max` (sets `cur` to the new `max`), then `amount`
    /// (additive delta, clamped to `[min, max]`). `amount` is the
    /// "delta" path consumables use (e.g. Health Slappack TC1: +500
    /// HP); the bounds-modifying fields are for buffs/debuffs that
    /// shift the cap.
    ChangeStat {
        stat_id: i32,
        min: Option<i32>,
        max: Option<i32>,
        use_ammo_stat: Option<bool>,
        set_to_max: Option<bool>,
        /// Additive delta applied to `cur` after bounds adjustments.
        /// Positive heals, negative damages. Clamped via `Stat::change`.
        amount: Option<i32>,
    },

    /// Abandon an active mission.
    AbandonMission { mission_id: i32 },

    /// Fail a specific objective within a mission.
    FailObjective { mission_id: i32, objective_id: i32 },

    /// Increment a named counter.
    IncrementCounter { counter_name: String, amount: i32 },

    /// Reset a named counter to zero.
    ResetCounter { counter_name: String },

    /// Complete a specific objective within a mission.
    CompleteObjective { mission_id: i32, objective_id: i32 },

    /// Set the visibility of a tagged entity.
    SetVisible { entity_tag: String, visible: bool },

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
    SetActiveSlot { bag_id: i32, slot: i32 },

    /// Force-fire an ability on an entity (or self if entity_tag is None).
    LaunchAbility {
        ability_id: i32,
        entity_tag: Option<String>,
    },

    /// Map a dialog set to an NPC entity template (archetype-conditional dialog).
    AddDialog {
        dialog_set_id: i32,
        entity_template: Option<i32>,
        mission_id: Option<i32>,
    },

    /// Generate threat/aggro on a target entity from the instigator.
    GenerateThreat {
        entity_tag: Option<String>,
        threat_level: i32,
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

/// Subset of [`cimmeria_entity::cell_entity::AiState`] reachable from
/// content actions. Other states (Fighting/Leashing/Patrol/Wander/
/// Investigating/Follow/Dead/Spawning) are owned by the runtime and
/// must be reached via their behavior-specific paths so the per-state
/// scratch fields are populated correctly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NpcAiStateAction {
    Idle,
    Despawning,
    Submit,
    Error,
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
                ActionResult::Error(format!(
                    "Action {:?} must be executed via resolve_event()",
                    self
                ))
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
        let action = Action::GrantItem {
            item_id: 55,
            count: 1,
            container_id: Some(3),
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("container_id"));
        let deserialized: Action = serde_json::from_str(&json).unwrap();
        match deserialized {
            Action::GrantItem {
                item_id,
                count,
                container_id,
            } => {
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
            Action::MoveWaypoint {
                entity_tag,
                destination,
                speed,
            } => {
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
            Action::LaunchAbility {
                ability_id,
                entity_tag,
            } => {
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
            Action::LaunchAbility {
                ability_id,
                entity_tag,
            } => {
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
