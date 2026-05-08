//! Chain loading from JSON and database row structs.
//!
//! Provides deserialization of chain definitions from JSON (for testing,
//! import/export, and the ServerEd chain editor) and conversion from
//! database row structs into typed `Chain` objects.
//!
//! Conversion logic is split by row family so each converter can grow with
//! its own feature surface without bloating one match arm:
//! - [`trigger`]   — `convert_trigger`
//! - [`condition`] — `convert_condition` + `parse_*` helpers for comparison
//!   operators and mission/step status enums
//! - [`action`]    — `convert_action` + `parse_destination`
//!
//! `mod.rs` keeps the orchestration ([`build_chains_from_rows`]),
//! the JSON loader, and the public DB row structs.

use std::collections::HashMap;

use tracing::warn;

use crate::actions::Action;
use crate::chain::Chain;
use crate::conditions::Condition;
use crate::triggers::Trigger;

mod action;
mod condition;
mod trigger;

#[cfg(test)]
mod tests;

/// Deserialize a list of chains from a JSON string.
pub fn load_chains_from_json(json: &str) -> Result<Vec<Chain>, serde_json::Error> {
    serde_json::from_str(json)
}

// ── Database row structs ──────────────────────────────────────────────────────

/// A row from `content_chains`.
#[derive(Debug)]
pub struct DbChainRow {
    pub chain_id: i32,
    pub description: Option<String>,
    pub scope_type: String,
    pub scope_id: Option<i32>,
    pub enabled: bool,
    pub priority: i32,
}

/// A row from `content_triggers`.
#[derive(Debug)]
pub struct DbTriggerRow {
    pub chain_id: i32,
    pub event_type: String,
    pub event_key: Option<String>,
    pub scope: String,
    pub once: bool,
    pub sort_order: i32,
}

/// A row from `content_conditions`.
#[derive(Debug)]
pub struct DbConditionRow {
    pub chain_id: i32,
    pub condition_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub operator: String,
    pub value: Option<String>,
    pub sort_order: i32,
}

/// A row from `content_actions`.
#[derive(Debug)]
pub struct DbActionRow {
    pub chain_id: i32,
    pub action_type: String,
    pub target_id: Option<i32>,
    pub target_key: Option<String>,
    pub params: serde_json::Value,
    pub delay_ms: i32,
    pub sort_order: i32,
}

// ── Conversion ────────────────────────────────────────────────────────────────

/// Build a Vec<Chain> from separate vectors of DB rows.
///
/// Chains that have no trigger rows are included (triggerless chains can be
/// invoked directly via `on_victory_chains`).
pub fn build_chains_from_rows(
    chain_rows: Vec<DbChainRow>,
    trigger_rows: Vec<DbTriggerRow>,
    condition_rows: Vec<DbConditionRow>,
    action_rows: Vec<DbActionRow>,
) -> Vec<Chain> {
    // Group triggers, conditions, actions by chain_id
    let mut triggers_by_chain: HashMap<i32, Vec<DbTriggerRow>> = HashMap::new();
    for row in trigger_rows {
        triggers_by_chain.entry(row.chain_id).or_default().push(row);
    }

    let mut conditions_by_chain: HashMap<i32, Vec<DbConditionRow>> = HashMap::new();
    for row in condition_rows {
        conditions_by_chain
            .entry(row.chain_id)
            .or_default()
            .push(row);
    }

    let mut actions_by_chain: HashMap<i32, Vec<DbActionRow>> = HashMap::new();
    for row in action_rows {
        actions_by_chain.entry(row.chain_id).or_default().push(row);
    }

    let mut chains = Vec::with_capacity(chain_rows.len());

    for row in chain_rows {
        let chain_id = row.chain_id;
        let name = row
            .description
            .unwrap_or_else(|| format!("chain_{}", chain_id));

        // Build conditions (shared across all triggers for this chain).
        let mut cond_list = conditions_by_chain.remove(&chain_id).unwrap_or_default();
        cond_list.sort_by_key(|c| c.sort_order);
        let conditions: Vec<Condition> = cond_list.iter().filter_map(|c_row| {
            let result = condition::convert_condition(c_row);
            if result.is_none() {
                warn!(chain_id, condition_type = %c_row.condition_type, "Unknown condition_type, skipping");
            }
            result
        }).collect();

        // Build actions (shared across all triggers for this chain).
        let mut act_list = actions_by_chain.remove(&chain_id).unwrap_or_default();
        act_list.sort_by_key(|a| a.sort_order);
        let actions: Vec<Action> = act_list.iter().filter_map(|a_row| {
            let result = action::convert_action(a_row);
            if result.is_none() {
                warn!(chain_id, action_type = %a_row.action_type, "Unknown action_type, skipping");
            }
            result
        }).collect();

        // Triggers — chains can declare multiple `content_triggers` rows
        // for OR-semantics (e.g., "fire on either MessHall_Guard1 OR
        // MessHall_Guard2 death"). Materialize one in-memory `Chain`
        // per trigger row, all sharing the same conditions and actions.
        // The runtime resolver doesn't need to know about the
        // multi-trigger origin — it sees N independent chains keyed by
        // each trigger's `TriggerType`. Same chain ID is reused so
        // chain-replay tests and logs identify them as one logical
        // chain.
        //
        // A chain with zero trigger rows is treated as triggerless
        // (invoked directly via `on_victory_chains` or `TriggerChain`)
        // and gets a single never-firing `OnCustomEvent` so it's
        // present in the engine but inert without explicit invocation.
        let mut trigger_list = triggers_by_chain.remove(&chain_id).unwrap_or_default();
        trigger_list.sort_by_key(|t| t.sort_order);

        let triggers: Vec<Trigger> = if trigger_list.is_empty() {
            vec![Trigger::OnCustomEvent {
                event_name: format!("__direct_invoke_{}", chain_id),
            }]
        } else {
            trigger_list
                .iter()
                .filter_map(|t_row| {
                    let result = trigger::convert_trigger(t_row);
                    if result.is_none() {
                        warn!(
                            chain_id,
                            event_type = %t_row.event_type,
                            "Unknown trigger event_type, skipping this trigger row"
                        );
                    }
                    result
                })
                .collect()
        };

        if triggers.is_empty() {
            warn!(
                chain_id,
                "All trigger rows failed to convert — skipping chain"
            );
            continue;
        }

        for trigger in triggers {
            chains.push(Chain {
                id: chain_id as i64,
                name: name.clone(),
                enabled: row.enabled,
                trigger,
                conditions: conditions.clone(),
                actions: actions.clone(),
                priority: row.priority,
            });
        }
    }

    chains
}
