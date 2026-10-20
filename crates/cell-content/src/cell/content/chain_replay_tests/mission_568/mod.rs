//! Mission 568 — Finding Tollan Technology (Tollana, world 19).
//!
//! Spec: `docs/analysis/tollana-continuation/w1-packets.md` (T-03a1) and the
//! frozen literals in `docs/analysis/tollana-continuation/w1-frozen.md` (F3,
//! F5). Seed: `db/resources/Content/Seed/tollana_568_chains.sql`, chains
//! 9001-9010 (Copplemann click, accept, turn-in, progress bind, Tollana
//! entry restore, abandon repaint). Chains 9011-9018 (food-unit props) are
//! T-03a2.
//!
//! Slot 420 holds at most one live bind at a time. Its state per mission
//! step is `S0` (offer 2656), `S1964`/`S1965` (progress 4959), `S1966`
//! (turn-in 6393) and `Done` (none). The Copplemann click is disjoint by
//! state: exactly one of 9001-9003 fires in every state but `Done`.
//!
//! Shapes and gates are asserted at resolve level here (D-TW11): which
//! chains fire for an event in a given state. State transitions through the
//! executor are covered by the rig siblings in this directory.
//!
//! Helpers below are `pub(super)` so the sibling modules reuse them.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;

mod copplemann;
mod props;
mod relog_abandon;
mod rig;
mod two_player;
mod walk;

/// `resources.worlds.world_id` for Tollana.
pub(super) const TOLLANA: i32 = 19;

/// Every step id in 568, in play order. `1963` is completed before any
/// state below; the step ids are read by `ctx` and the rig siblings.
pub(super) const STEPS: [&str; 4] = ["1963", "1964", "1965", "1966"];

/// The mission states the chain tests walk. `S0` is not active; the
/// `S1964`..`S1966` states have that step active and every earlier step
/// completed; `Done` has the mission and all four steps completed.
#[derive(Clone, Copy, Debug)]
pub(super) enum S {
    S0,
    S1964,
    S1965,
    S1966,
    Done,
}

/// Every state, for exhaustive walks.
pub(super) const ALL: [S; 5] = [S::S0, S::S1964, S::S1965, S::S1966, S::Done];

/// Build a context for mission 568 in `world_id`, in state `s`.
///
/// Mirrors `populate_mission_context`: the mission status is written, and
/// each step status is written `completed` or `active`. An absent step key
/// reads as `not_active` through the evaluator's fallback.
pub(super) fn ctx(world_id: i32, s: S) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(world_id);
    let (mission, completed, active) = match s {
        S::S0 => ("not_active", 0, None),
        S::S1964 => ("active", 1, Some(1)),
        S::S1965 => ("active", 2, Some(2)),
        S::S1966 => ("active", 3, Some(3)),
        S::Done => ("completed", 4, None),
    };
    ctx.set_param("mission_568_status".to_string(), serde_json::json!(mission));
    for (i, step) in STEPS.iter().enumerate() {
        let status = if i < completed {
            Some("completed")
        } else if active == Some(i) {
            Some("active")
        } else {
            None
        };
        if let Some(status) = status {
            ctx.set_param(
                format!("mission_568_step_{step}_status"),
                serde_json::json!(status),
            );
        }
    }
    ctx
}

pub(super) fn with_tag(mut ctx: ExecutionContext, tag: &str) -> ExecutionContext {
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx
}

pub(super) fn with_dialog(mut ctx: ExecutionContext, dialog_id: i32) -> ExecutionContext {
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx
}

pub(super) fn with_world_name(mut ctx: ExecutionContext, name: &str) -> ExecutionContext {
    ctx.set_param("world_name".to_string(), serde_json::json!(name));
    ctx
}

pub(super) fn with_mission(mut ctx: ExecutionContext, mission_id: i32) -> ExecutionContext {
    ctx.set_param("mission_id".to_string(), serde_json::json!(mission_id));
    ctx
}

/// Load one chain, or fail with a message that separates "row missing"
/// from "row present but the loader rejected it".
pub(super) async fn load(pool: &sqlx::PgPool, chain_id: i32) -> Chain {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains AND load cleanly \
                 (an unknown trigger/condition/action type is dropped with a warn, which \
                 looks identical to a missing row from here)"
            )
        })
}

/// Resolve one event against a single loaded chain.
pub(super) async fn resolve_one(
    pool: &sqlx::PgPool,
    chain_id: i32,
    ctx: &ExecutionContext,
    tt: TriggerType,
) -> ResolvedActions {
    let mut engine = ChainEngine::new();
    engine.register_chain(load(pool, chain_id).await);
    let event = TriggerEvent {
        trigger_type: tt,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// The actions a chain resolves, stripped of chain ids.
pub(super) fn actions_of(resolved: &ResolvedActions) -> Vec<Action> {
    resolved.actions.iter().map(|(_, a)| a.clone()).collect()
}

/// Every 568 chain id that `tt` with `ctx` fires on the whole seeded engine:
/// the ids in `9001..=9018`, sorted and de-duplicated.
pub(super) async fn fired(
    engine: &ChainEngine,
    tt: TriggerType,
    ctx: &ExecutionContext,
) -> Vec<i64> {
    let event = TriggerEvent {
        trigger_type: tt,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, ctx);
    let mut ids: Vec<i64> = resolved
        .actions
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| (9001..=9018).contains(id))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}
