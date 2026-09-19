//! Chain definition and engine.
//!
//! A **chain** is the fundamental unit of data-driven content: a trigger, a set
//! of conditions, and a list of actions. The [`ChainEngine`] indexes chains by
//! trigger type and provides the main [`fire_event`](ChainEngine::fire_event)
//! entry point that the game servers call when gameplay events occur.
//!
//! Split from a single `chain.rs` once the `#[cfg(test)] mod tests` block
//! crossed the 500-line soft cap from `CLAUDE.md` — the natural seam is
//! definition/engine vs. tests, so the test body moved to [`tests`]
//! unchanged, with no behavior change on this side.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, trace, warn};

use crate::actions::{Action, ActionResult};
use crate::conditions::Condition;
use crate::context::ExecutionContext;
use crate::triggers::{Trigger, TriggerEvent, TriggerType};

#[cfg(test)]
mod tests;

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

    /// Per-action delay in milliseconds, index-aligned with `actions`
    /// (`action_delays[i]` is the delay for `actions[i]`). A missing index
    /// — e.g. a hand-built chain in a test that never sets this field —
    /// means `delay_ms == 0` for that action, which is the overwhelming
    /// majority case and matches pre-C08a behavior exactly. Sourced from
    /// `content_actions.delay_ms`; see `resolve_event` and
    /// `services::cell::content::executor::execute_actions` for how a
    /// nonzero delay defers execution instead of running inline.
    #[serde(default)]
    pub action_delays: Vec<i32>,

    /// Ordering priority. Higher values execute first when multiple chains
    /// match the same event.
    pub priority: i32,
}

impl Chain {
    /// Is at least one of this chain's conditions a read of per-player
    /// mission state (see [`Condition::gates_on_mission_state`])?
    ///
    /// The property this answers is "would running this chain a second time
    /// for the same event be harmless?". A mission-gated chain says yes: its
    /// actions advance the step or complete the objective its own gate reads,
    /// so the second evaluation fails the gate and nothing runs. A chain with
    /// no mission gate says no — a bare `enter_region` → `display_dialog`
    /// would show the dialog twice.
    ///
    /// Only the server-side *replay* of an already-spent edge event consults
    /// this (Harset H52). Ordinary event dispatch runs every matching chain
    /// regardless.
    pub fn is_mission_gated(&self) -> bool {
        self.conditions
            .iter()
            .any(Condition::gates_on_mission_state)
    }
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
        bucket.sort_by_key(|c| std::cmp::Reverse(c.priority));
    }

    /// Return the total number of registered chains (enabled and disabled).
    pub fn chain_count(&self) -> usize {
        self.chains_by_trigger.values().map(|v| v.len()).sum()
    }

    /// Get the actions for a chain by its ID, bypassing trigger matching,
    /// paired with each action's `delay_ms` (0 when `action_delays` has no
    /// entry for that index — see the field doc on [`Chain::action_delays`]).
    /// Used for direct invocation (e.g., minigame victory callbacks).
    pub fn get_chain_actions(&self, chain_id: i64) -> Vec<(Action, i32)> {
        for chains in self.chains_by_trigger.values() {
            for chain in chains {
                if chain.id == chain_id {
                    return chain
                        .actions
                        .iter()
                        .cloned()
                        .enumerate()
                        .map(|(i, action)| {
                            let delay_ms = chain.action_delays.get(i).copied().unwrap_or(0);
                            (action, delay_ms)
                        })
                        .collect();
                }
            }
        }
        vec![]
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
///
/// `params` carries forward the resolution-time `ExecutionContext.params`
/// so action executors can read trigger-time state without holding the
/// original context. This is how `Action::RemoveItem` looks up
/// `instance_id` set by `fire_item_use` to consume the exact stack the
/// player clicked instead of the player's first-by-type instance.
///
/// `action_delays` is index-aligned with `actions` (kept as a parallel
/// vec, not a wider tuple, so the many existing test call sites that
/// build a `ResolvedActions` literal by hand only need one new field
/// added, not every `(chain_id, action)` pair touched). `action_delays[i]`
/// is the `content_actions.delay_ms` value for `actions[i]`; a missing
/// index (an empty `action_delays`, the common case in hand-built test
/// fixtures) means `delay_ms == 0`. The caller
/// (`services::cell::content::executor::execute_actions`) runs
/// `delay_ms == 0` actions inline and defers the rest.
#[derive(Default)]
pub struct ResolvedActions {
    pub actions: Vec<(i64, Action)>,
    pub action_delays: Vec<i32>,
    pub params: std::collections::HashMap<String, serde_json::Value>,
}

impl ChainEngine {
    /// Match triggers and evaluate conditions like [`fire_event`](Self::fire_event),
    /// but return the actions instead of executing them.
    ///
    /// This lets the caller (CellService) execute actions in a context where it
    /// has access to game state (SpaceManager, channels, etc.) that the engine
    /// itself doesn't know about.
    pub fn resolve_event(&self, event: &TriggerEvent, ctx: &ExecutionContext) -> ResolvedActions {
        self.resolve_event_filtered(event, ctx, |_| true)
    }

    /// [`resolve_event`](Self::resolve_event), but a chain whose trigger
    /// matched is only admitted when `admit` returns `true` for it.
    ///
    /// The one caller that passes a real filter is the H52 step-activation
    /// replay, which re-fires `enter_region` for volumes the player is
    /// already standing in when a mission step activates. That event has
    /// *already* been delivered once (or is about to be, when the client's
    /// own hint arrives a moment later), so the replay admits only
    /// [`Chain::is_mission_gated`] chains — the ones for which a double
    /// delivery is a no-op. See the Harset H52 worknote.
    ///
    /// A rejected chain is logged at `debug` on the same
    /// `target: "content.resolve"` stream as a failed condition, with
    /// `reason = "filtered_out"`, so an author whose chain did not replay can
    /// see that it matched and was refused rather than never matching.
    pub fn resolve_event_filtered(
        &self,
        event: &TriggerEvent,
        ctx: &ExecutionContext,
        admit: impl Fn(&Chain) -> bool,
    ) -> ResolvedActions {
        let mut resolved = ResolvedActions::default();

        let chains = match self.chains_by_trigger.get(&event.trigger_type) {
            Some(chains) => chains,
            None => return resolved,
        };

        for chain in chains {
            if !chain.enabled || !chain.trigger.matches(event) {
                continue;
            }

            if !admit(chain) {
                debug!(
                    target: "content.resolve",
                    chain_id = chain.id,
                    chain_name = %chain.name,
                    trigger_type = ?event.trigger_type,
                    source_entity = ?ctx.source_entity_id,
                    reason = "filtered_out",
                    "content resolve: trigger matched but the caller's filter refused the chain"
                );
                continue;
            }

            // Name the FIRST failing condition. "no chains matched" at the
            // dispatch site cannot distinguish "no chain listens for this"
            // from "a chain listens but its step is not active yet" -- the
            // second is an ordering bug (2026-09-18: cover-entered fired 1 s
            // before the step that consumes it), and only this line shows it.
            if let Some((idx, failed)) = chain
                .conditions
                .iter()
                .enumerate()
                .find(|(_, c)| !c.evaluate(ctx))
            {
                debug!(
                    target: "content.resolve",
                    chain_id = chain.id,
                    chain_name = %chain.name,
                    trigger_type = ?event.trigger_type,
                    source_entity = ?ctx.source_entity_id,
                    reason = "condition_failed",
                    failed_condition_index = idx,
                    failed_condition = ?failed,
                    conditions_total = chain.conditions.len(),
                    "content resolve: trigger matched but a condition failed -- chain skipped"
                );
                continue;
            }

            debug!(chain_id = chain.id, chain_name = %chain.name, actions = chain.actions.len(), "resolve_event: chain matched");
            for (i, action) in chain.actions.iter().enumerate() {
                let delay_ms = chain.action_delays.get(i).copied().unwrap_or(0);
                resolved.actions.push((chain.id, action.clone()));
                resolved.action_delays.push(delay_ms);
            }
        }

        // Defer the params clone until at least one chain matched.
        // `resolve_event` runs on every gameplay tick that produces an
        // event (entity death, region cross, item use, …), most of
        // which return zero actions; cloning the populated context
        // unconditionally wasted bytes on every miss.
        if !resolved.actions.is_empty() {
            resolved.params = ctx.params.clone();
        }

        resolved
    }
}

impl Default for ChainEngine {
    fn default() -> Self {
        Self::new()
    }
}
