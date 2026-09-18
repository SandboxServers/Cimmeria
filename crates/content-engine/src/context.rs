//! Execution context for chain evaluation and action dispatch.
//!
//! The [`ExecutionContext`] carries the runtime state needed to evaluate
//! conditions and execute actions within a chain. It holds references to the
//! source and target entities, the current space, arbitrary parameters, and
//! the accumulated results from executed actions.
//!
//! The context is created fresh for each chain evaluation pass and passed
//! mutably through the condition/action pipeline.

use std::collections::HashMap;

use cimmeria_common::{EntityId, SpaceId};

use crate::actions::ActionResult;

/// Runtime context for evaluating conditions and executing actions.
///
/// Created by the [`ChainEngine`](crate::chain::ChainEngine) before processing
/// a chain and passed through each condition check and action execution. Actions
/// may modify the context (e.g., setting parameters that later actions read).
pub struct ExecutionContext {
    /// The entity that caused the triggering event (e.g., the player who used
    /// an ability, the mob that died).
    pub source_entity_id: Option<EntityId>,

    /// The entity that the event targets (e.g., the NPC being interacted with,
    /// the entity receiving damage).
    pub target_entity_id: Option<EntityId>,

    /// The game space in which the event occurred.
    pub space_id: Option<SpaceId>,

    /// The numeric `resources.worlds.world_id` of the world the acting
    /// player is in, resolved from their space.
    ///
    /// Deliberately **not** derivable from [`Self::space_id`]: that is a
    /// runtime space *instance* id (`(cell_id << 16) | local_index`), and
    /// several instances of the same world share one world id. Only the
    /// services-side populator can do the mapping, because the id lives in
    /// the database and `spaces.xml` carries world *names* only.
    ///
    /// `None` means "nobody told us" — [`Condition::World`](crate::conditions::Condition::World)
    /// treats that as fail-closed (see its evaluator arm).
    pub world_id: Option<i32>,

    /// Arbitrary key-value parameters carried through the chain. Triggers
    /// populate initial values; actions may add or modify them.
    pub params: HashMap<String, serde_json::Value>,

    /// Accumulated results from each action executed in this chain pass.
    pub results: Vec<ActionResult>,
}

impl ExecutionContext {
    /// Create a new empty execution context.
    pub fn new() -> Self {
        Self {
            source_entity_id: None,
            target_entity_id: None,
            space_id: None,
            world_id: None,
            params: HashMap::new(),
            results: Vec::new(),
        }
    }

    /// Set the source entity and return `self` for builder-style chaining.
    pub fn with_source(mut self, id: EntityId) -> Self {
        self.source_entity_id = Some(id);
        self
    }

    /// Set the target entity and return `self` for builder-style chaining.
    pub fn with_target(mut self, id: EntityId) -> Self {
        self.target_entity_id = Some(id);
        self
    }

    /// Set the space ID and return `self` for builder-style chaining.
    pub fn with_space(mut self, id: SpaceId) -> Self {
        self.space_id = Some(id);
        self
    }

    /// Set the world ID and return `self` for builder-style chaining.
    ///
    /// The value is `resources.worlds.world_id` — the same id space
    /// `spawnlist`, `stargates` and `ring_transport_regions` use, and the
    /// one a `world` condition row is authored against.
    pub fn with_world(mut self, world_id: i32) -> Self {
        self.world_id = Some(world_id);
        self
    }

    /// Retrieve a parameter value by key.
    pub fn get_param(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.get(key)
    }

    /// Set a parameter value, replacing any existing value for that key.
    pub fn set_param(&mut self, key: String, value: serde_json::Value) {
        self.params.insert(key, value);
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_context_is_empty() {
        let ctx = ExecutionContext::new();
        assert!(ctx.source_entity_id.is_none());
        assert!(ctx.target_entity_id.is_none());
        assert!(ctx.space_id.is_none());
        assert!(ctx.world_id.is_none());
        assert!(ctx.params.is_empty());
        assert!(ctx.results.is_empty());
    }

    #[test]
    fn builder_pattern() {
        let ctx = ExecutionContext::new()
            .with_source(EntityId(1))
            .with_target(EntityId(2))
            .with_space(SpaceId(100))
            .with_world(57);
        assert_eq!(ctx.source_entity_id, Some(EntityId(1)));
        assert_eq!(ctx.target_entity_id, Some(EntityId(2)));
        assert_eq!(ctx.space_id, Some(SpaceId(100)));
        // `space_id` is a runtime instance id and `world_id` is the DB
        // world id; they are independent on purpose (several instances of
        // Harset share world 57).
        assert_eq!(ctx.world_id, Some(57));
    }

    #[test]
    fn set_and_get_param() {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("damage".to_string(), serde_json::json!(42));
        assert_eq!(ctx.get_param("damage"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn get_missing_param_returns_none() {
        let ctx = ExecutionContext::new();
        assert!(ctx.get_param("nonexistent").is_none());
    }

    #[test]
    fn set_param_overwrites() {
        let mut ctx = ExecutionContext::new();
        ctx.set_param("level".to_string(), serde_json::json!(1));
        ctx.set_param("level".to_string(), serde_json::json!(50));
        assert_eq!(ctx.get_param("level"), Some(&serde_json::json!(50)));
    }
}
