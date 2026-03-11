//! Chain definition and engine.
//!
//! A **chain** is the fundamental unit of data-driven content: a trigger, a set
//! of conditions, and a list of actions. The [`ChainEngine`] indexes chains by
//! trigger type and provides the main [`fire_event`](ChainEngine::fire_event)
//! entry point that the game servers call when gameplay events occur.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::actions::{Action, ActionResult};
use crate::conditions::Condition;
use crate::context::ExecutionContext;
use crate::triggers::{Trigger, TriggerEvent, TriggerType};

/// A single content chain: trigger + conditions + actions.
///
/// Chains are typically loaded from the database at server startup. Each chain
/// has a unique `id`, a human-readable `name`, an `enabled` flag for toggling
/// without deletion, a `priority` for ordering when multiple chains match the
/// same event, and the trigger/conditions/actions triple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain {
    /// Unique database identifier for this chain.
    pub id: i64,

    /// Human-readable name for this chain (e.g., "Grant XP on Jaffa kill").
    pub name: String,

    /// Whether this chain is active. Disabled chains are skipped during event
    /// processing.
    pub enabled: bool,

    /// The event that activates this chain.
    pub trigger: Trigger,

    /// Conditions that must all be true for the actions to execute.
    pub conditions: Vec<Condition>,

    /// Actions to execute in order when the chain fires.
    pub actions: Vec<Action>,

    /// Ordering priority. Higher values execute first when multiple chains
    /// match the same event.
    pub priority: i32,
}

/// The chain engine: indexes chains by trigger type and dispatches events.
///
/// The engine is the central runtime component of the content system. Game
/// services register chains at startup (loaded from the database), then call
/// [`fire_event`](Self::fire_event) whenever gameplay events occur. The engine
/// finds matching chains, evaluates conditions, and executes actions.
pub struct ChainEngine {
    /// Chains grouped by their trigger type for efficient lookup.
    chains_by_trigger: HashMap<TriggerType, Vec<Chain>>,
}

impl ChainEngine {
    /// Create a new empty chain engine with no registered chains.
    pub fn new() -> Self {
        Self {
            chains_by_trigger: HashMap::new(),
        }
    }

    /// Register a chain with the engine.
    ///
    /// The chain is indexed by its trigger type. If the chain is disabled, it
    /// is still registered but will be skipped during event processing.
    pub fn register_chain(&mut self, chain: Chain) {
        let trigger_type = chain.trigger.trigger_type();
        debug!(
            chain_id = chain.id,
            chain_name = %chain.name,
            trigger = ?trigger_type,
            enabled = chain.enabled,
            priority = chain.priority,
            "Registering chain"
        );
        let bucket = self.chains_by_trigger.entry(trigger_type).or_default();
        bucket.push(chain);
        // Re-sort by priority descending so higher-priority chains run first.
        bucket.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Return the total number of registered chains (enabled and disabled).
    pub fn chain_count(&self) -> usize {
        self.chains_by_trigger.values().map(|v| v.len()).sum()
    }

    /// Return the number of registered chains for a specific trigger type.
    pub fn chains_for_trigger(&self, trigger_type: &TriggerType) -> usize {
        self.chains_by_trigger
            .get(trigger_type)
            .map_or(0, |v| v.len())
    }

    /// Process a game event through all matching chains.
    ///
    /// The engine:
    /// 1. Looks up chains registered for the event's trigger type.
    /// 2. Filters to enabled chains whose trigger matches the event.
    /// 3. Evaluates all conditions on each matching chain.
    /// 4. Executes the action list for chains where all conditions pass.
    /// 5. Collects action results into the execution context.
    ///
    /// Chains are processed in priority order (highest first). If any action
    /// returns [`ActionResult::ChainTrigger`], the triggered chain ID is logged
    /// but not recursively evaluated in this call (to prevent infinite loops).
    /// The caller is responsible for re-dispatching triggered chains.
    pub fn fire_event(&self, event: &TriggerEvent, ctx: &mut ExecutionContext) {
        let chains = match self.chains_by_trigger.get(&event.trigger_type) {
            Some(chains) => chains,
            None => {
                trace!(trigger = ?event.trigger_type, "No chains registered for trigger type");
                return;
            }
        };

        for chain in chains {
            // Skip disabled chains.
            if !chain.enabled {
                trace!(chain_id = chain.id, chain_name = %chain.name, "Skipping disabled chain");
                continue;
            }

            // Check if the trigger's filter criteria match the event.
            if !chain.trigger.matches(event) {
                trace!(
                    chain_id = chain.id,
                    chain_name = %chain.name,
                    "Trigger filter did not match"
                );
                continue;
            }

            debug!(
                chain_id = chain.id,
                chain_name = %chain.name,
                condition_count = chain.conditions.len(),
                "Evaluating chain conditions"
            );

            // Evaluate all conditions (logical AND). Short-circuit on first failure.
            let conditions_met = chain.conditions.iter().all(|condition| {
                let result = condition.evaluate(ctx);
                if !result {
                    trace!(
                        chain_id = chain.id,
                        condition = ?condition,
                        "Condition failed"
                    );
                }
                result
            });

            if !conditions_met {
                debug!(
                    chain_id = chain.id,
                    chain_name = %chain.name,
                    "Chain conditions not met, skipping actions"
                );
                continue;
            }

            debug!(
                chain_id = chain.id,
                chain_name = %chain.name,
                action_count = chain.actions.len(),
                "Executing chain actions"
            );

            // Execute all actions in order.
            for (i, action) in chain.actions.iter().enumerate() {
                let result = action.execute(ctx);
                match &result {
                    ActionResult::Success => {
                        trace!(
                            chain_id = chain.id,
                            action_index = i,
                            action = ?action,
                            "Action succeeded"
                        );
                    }
                    ActionResult::Error(msg) => {
                        warn!(
                            chain_id = chain.id,
                            chain_name = %chain.name,
                            action_index = i,
                            error = %msg,
                            "Action failed"
                        );
                    }
                    ActionResult::ChainTrigger(target_id) => {
                        debug!(
                            chain_id = chain.id,
                            target_chain_id = target_id,
                            "Action requested chain trigger (caller must re-dispatch)"
                        );
                    }
                }
                ctx.results.push(result);
            }
        }
    }
}

/// Actions collected by [`ChainEngine::resolve_event`] without executing them.
///
/// Each entry pairs a chain ID with the action to execute, preserving the
/// ordering (highest-priority chain first, actions in declaration order).
pub struct ResolvedActions {
    pub actions: Vec<(i64, Action)>,
}

impl ChainEngine {
    /// Match triggers and evaluate conditions like [`fire_event`](Self::fire_event),
    /// but return the actions instead of executing them.
    ///
    /// This lets the caller (CellService) execute actions in a context where it
    /// has access to game state (SpaceManager, channels, etc.) that the engine
    /// itself doesn't know about.
    pub fn resolve_event(&self, event: &TriggerEvent, ctx: &ExecutionContext) -> ResolvedActions {
        let mut resolved = ResolvedActions { actions: Vec::new() };

        let chains = match self.chains_by_trigger.get(&event.trigger_type) {
            Some(chains) => chains,
            None => return resolved,
        };

        for chain in chains {
            if !chain.enabled || !chain.trigger.matches(event) {
                continue;
            }

            let conditions_met = chain.conditions.iter().all(|c| c.evaluate(ctx));
            if !conditions_met {
                debug!(chain_id = chain.id, chain_name = %chain.name, "resolve_event: conditions not met");
                continue;
            }

            debug!(chain_id = chain.id, chain_name = %chain.name, actions = chain.actions.len(), "resolve_event: chain matched");
            for action in &chain.actions {
                resolved.actions.push((chain.id, action.clone()));
            }
        }

        resolved
    }
}

impl Default for ChainEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triggers::TriggerType;

    /// Helper to build a simple chain with no conditions.
    fn make_chain(id: i64, trigger: Trigger, actions: Vec<Action>, priority: i32) -> Chain {
        Chain {
            id,
            name: format!("test_chain_{}", id),
            enabled: true,
            trigger,
            conditions: Vec::new(),
            actions,
            priority,
        }
    }

    #[test]
    fn new_engine_has_no_chains() {
        let engine = ChainEngine::new();
        assert_eq!(engine.chain_count(), 0);
    }

    #[test]
    fn register_chain_increases_count() {
        let mut engine = ChainEngine::new();
        let chain = make_chain(
            1,
            Trigger::OnEntityCreated { entity_type: None },
            vec![],
            0,
        );
        engine.register_chain(chain);
        assert_eq!(engine.chain_count(), 1);
        assert_eq!(engine.chains_for_trigger(&TriggerType::EntityCreated), 1);
    }

    #[test]
    fn chains_sorted_by_priority_descending() {
        let mut engine = ChainEngine::new();
        engine.register_chain(make_chain(
            1,
            Trigger::OnEntityCreated { entity_type: None },
            vec![],
            10,
        ));
        engine.register_chain(make_chain(
            2,
            Trigger::OnEntityCreated { entity_type: None },
            vec![],
            50,
        ));
        engine.register_chain(make_chain(
            3,
            Trigger::OnEntityCreated { entity_type: None },
            vec![],
            30,
        ));

        let chains = engine
            .chains_by_trigger
            .get(&TriggerType::EntityCreated)
            .unwrap();
        assert_eq!(chains[0].id, 2); // priority 50
        assert_eq!(chains[1].id, 3); // priority 30
        assert_eq!(chains[2].id, 1); // priority 10
    }

    #[test]
    fn fire_event_with_no_matching_chains() {
        let engine = ChainEngine::new();
        let event = TriggerEvent {
            trigger_type: TriggerType::EntityCreated,
            source_entity: None,
            target_entity: None,
            params: HashMap::new(),
        };
        let mut ctx = ExecutionContext::new();
        // Should not panic - just a no-op.
        engine.fire_event(&event, &mut ctx);
        assert!(ctx.results.is_empty());
    }

    #[test]
    fn disabled_chain_is_skipped() {
        let mut engine = ChainEngine::new();
        let mut chain = make_chain(
            1,
            Trigger::OnEntityCreated { entity_type: None },
            vec![Action::TriggerChain { chain_id: 99 }],
            0,
        );
        chain.enabled = false;
        engine.register_chain(chain);

        let event = TriggerEvent {
            trigger_type: TriggerType::EntityCreated,
            source_entity: None,
            target_entity: None,
            params: HashMap::new(),
        };
        let mut ctx = ExecutionContext::new();
        engine.fire_event(&event, &mut ctx);
        // No actions should have executed.
        assert!(ctx.results.is_empty());
    }

    #[test]
    fn trigger_chain_action_produces_chain_trigger_result() {
        let mut engine = ChainEngine::new();
        let chain = make_chain(
            1,
            Trigger::OnCustomEvent {
                event_name: "test".to_string(),
            },
            vec![Action::TriggerChain { chain_id: 42 }],
            0,
        );
        engine.register_chain(chain);

        let mut params = HashMap::new();
        params.insert(
            "event_name".to_string(),
            serde_json::Value::String("test".to_string()),
        );
        let event = TriggerEvent {
            trigger_type: TriggerType::CustomEvent,
            source_entity: None,
            target_entity: None,
            params,
        };
        let mut ctx = ExecutionContext::new();
        engine.fire_event(&event, &mut ctx);

        assert_eq!(ctx.results.len(), 1);
        match &ctx.results[0] {
            ActionResult::ChainTrigger(id) => assert_eq!(*id, 42),
            other => panic!("Expected ChainTrigger(42), got {:?}", other),
        }
    }

    #[test]
    fn chain_serialization_roundtrip() {
        let chain = Chain {
            id: 1,
            name: "Test Chain".to_string(),
            enabled: true,
            trigger: Trigger::OnEntityDeath {
                entity_type: Some("SGWMob".to_string()),
                entity_tag: None,
            },
            conditions: vec![Condition::HasItem {
                item_id: 10,
                min_count: Some(1),
            }],
            actions: vec![
                Action::GrantXP { amount: 100 },
                Action::GrantItem {
                    item_id: 20,
                    count: 1,
                    container_id: None,
                },
            ],
            priority: 5,
        };

        let json = serde_json::to_string_pretty(&chain).unwrap();
        let deserialized: Chain = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.name, "Test Chain");
        assert!(deserialized.enabled);
        assert_eq!(deserialized.priority, 5);
        assert_eq!(deserialized.conditions.len(), 1);
        assert_eq!(deserialized.actions.len(), 2);
    }

    #[test]
    fn multiple_trigger_types_are_independent() {
        let mut engine = ChainEngine::new();
        engine.register_chain(make_chain(
            1,
            Trigger::OnEntityCreated { entity_type: None },
            vec![],
            0,
        ));
        engine.register_chain(make_chain(
            2,
            Trigger::OnEntityDeath { entity_type: None, entity_tag: None },
            vec![],
            0,
        ));
        engine.register_chain(make_chain(
            3,
            Trigger::OnEntityDeath { entity_type: None, entity_tag: None },
            vec![],
            0,
        ));

        assert_eq!(engine.chain_count(), 3);
        assert_eq!(engine.chains_for_trigger(&TriggerType::EntityCreated), 1);
        assert_eq!(engine.chains_for_trigger(&TriggerType::EntityDeath), 2);
        assert_eq!(engine.chains_for_trigger(&TriggerType::Timer), 0);
    }
}
