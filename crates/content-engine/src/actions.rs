//! Action executors for chain effects.
//!
//! Actions are the "do" part of a chain -- they modify game state when a
//! trigger fires and all conditions pass. The 21 action types cover the
//! breadth of the original Python scripting layer: XP/item grants, effects,
//! teleportation, spawning, dialog, missions, loot, timers, and extensibility
//! hooks.

use serde::{Deserialize, Serialize};

use crate::context::ExecutionContext;

/// An action to execute when a chain's trigger fires and conditions pass.
///
/// Actions execute in order. Each returns an [`ActionResult`] that the chain
/// engine collects. The special `ChainTrigger` result causes the engine to
/// enqueue another chain for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Award experience points to the source entity.
    GrantXP { amount: u64 },

    /// Add items to the source entity's inventory.
    GrantItem { item_id: i32, count: i32 },

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

    /// Despawn the target entity (remove from world).
    DespawnEntity,

    /// Open a dialog set for the source entity (player).
    StartDialog { dialog_set_id: i32 },

    /// Advance the specified mission to its next step.
    AdvanceMission { mission_id: i32 },

    /// Mark the specified mission as complete.
    CompleteMission { mission_id: i32 },

    /// Play an animation on the source or target entity.
    PlayAnimation { animation: String },

    /// Play a sound effect at the source entity's position.
    PlaySound { sound: String },

    /// Send a text message on a named channel (system, chat, etc.).
    SendMessage { channel: String, message: String },

    /// Modify a property on the source entity using the given operation.
    ModifyProperty {
        property: String,
        operation: PropertyOp,
        value: serde_json::Value,
    },

    /// Roll a loot table and grant results to the source entity.
    RollLootTable { table_id: i32 },

    /// Spawn a loot bag entity at the given position.
    SpawnLootBag { position: [f32; 3] },

    /// Start a named timer that will fire an `OnTimer` trigger when it expires.
    StartTimer {
        name: String,
        duration_secs: f32,
        repeat: bool,
    },

    /// Cancel a running named timer.
    CancelTimer { name: String },

    /// Trigger another chain by ID (allows chain composition).
    TriggerChain { chain_id: i64 },

    /// Execute a custom handler function with arbitrary parameters.
    ExecuteCustom {
        handler: String,
        params: serde_json::Value,
    },
}

/// Arithmetic/assignment operation for [`Action::ModifyProperty`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyOp {
    /// Replace the property value entirely.
    Set,
    /// Add the value to the current property (numeric).
    Add,
    /// Subtract the value from the current property (numeric).
    Subtract,
    /// Multiply the current property by the value (numeric).
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
    /// Returns an [`ActionResult`] indicating success, failure, or a request
    /// to trigger another chain.
    ///
    /// # Current Status
    ///
    /// All variants currently delegate to `todo!()` placeholders. Real
    /// implementations will call into game service interfaces (inventory,
    /// combat, spatial, mission, etc.) to apply their effects.
    pub fn execute(&self, ctx: &mut ExecutionContext) -> ActionResult {
        let _ = ctx;
        match self {
            Action::GrantXP { amount } => {
                let _ = amount;
                todo!("Action::GrantXP - award XP via combat/progression service")
            }
            Action::GrantItem { item_id, count } => {
                let _ = (item_id, count);
                todo!("Action::GrantItem - add items via inventory service")
            }
            Action::RemoveItem { item_id, count } => {
                let _ = (item_id, count);
                todo!("Action::RemoveItem - remove items via inventory service")
            }
            Action::ApplyEffect { effect_id, duration_secs } => {
                let _ = (effect_id, duration_secs);
                todo!("Action::ApplyEffect - apply effect via effects service")
            }
            Action::RemoveEffect { effect_id } => {
                let _ = effect_id;
                todo!("Action::RemoveEffect - remove effect via effects service")
            }
            Action::Teleport { space_id, position } => {
                let _ = (space_id, position);
                todo!("Action::Teleport - move entity via spatial service")
            }
            Action::SpawnEntity { template_id, position } => {
                let _ = (template_id, position);
                todo!("Action::SpawnEntity - create entity via entity manager")
            }
            Action::DespawnEntity => {
                todo!("Action::DespawnEntity - destroy target entity via entity manager")
            }
            Action::StartDialog { dialog_set_id } => {
                let _ = dialog_set_id;
                todo!("Action::StartDialog - open dialog via dialog service")
            }
            Action::AdvanceMission { mission_id } => {
                let _ = mission_id;
                todo!("Action::AdvanceMission - advance mission via mission service")
            }
            Action::CompleteMission { mission_id } => {
                let _ = mission_id;
                todo!("Action::CompleteMission - complete mission via mission service")
            }
            Action::PlayAnimation { animation } => {
                let _ = animation;
                todo!("Action::PlayAnimation - send animation command to client")
            }
            Action::PlaySound { sound } => {
                let _ = sound;
                todo!("Action::PlaySound - send sound event to client")
            }
            Action::SendMessage { channel, message } => {
                let _ = (channel, message);
                todo!("Action::SendMessage - send text message via chat service")
            }
            Action::ModifyProperty { property, operation, value } => {
                let _ = (property, operation, value);
                todo!("Action::ModifyProperty - modify entity property via entity system")
            }
            Action::RollLootTable { table_id } => {
                let _ = table_id;
                todo!("Action::RollLootTable - roll loot table via loot service")
            }
            Action::SpawnLootBag { position } => {
                let _ = position;
                todo!("Action::SpawnLootBag - spawn loot bag entity at position")
            }
            Action::StartTimer { name, duration_secs, repeat } => {
                let _ = (name, duration_secs, repeat);
                todo!("Action::StartTimer - register timer via timer service")
            }
            Action::CancelTimer { name } => {
                let _ = name;
                todo!("Action::CancelTimer - cancel timer via timer service")
            }
            Action::TriggerChain { chain_id } => {
                let _ = chain_id;
                ActionResult::ChainTrigger(*chain_id)
            }
            Action::ExecuteCustom { handler, params } => {
                let _ = (handler, params);
                todo!("Action::ExecuteCustom - dispatch to custom handler registry")
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
}
