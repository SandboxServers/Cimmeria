//! The fire-once gate (`content_triggers.once`, #802).
//!
//! A chain loaded with [`Chain::once`](cimmeria_content_engine::chain::Chain)
//! fires once per `(entity the actions execute for, chain id)` and then
//! disarms for the rest of that cell entity's life — once per player per
//! space visit or login, the lifetime of the 2009 per-player level script
//! whose `once = True` subscriptions this models.
//!
//! Why here and not in `resolve_event`: [`super::execute_actions`] is the one
//! choke point every dispatcher already calls with the entity whose state the
//! chain acts on, and deferred actions do not re-enter it
//! (`deferred.rs` calls `execute_one`). A filter inside resolution would need
//! every dispatcher to populate per-player state first, and forgetting one
//! site would silently re-open the bug.
//!
//! A chain only reaches the executor when its trigger matched and every
//! condition passed, so a once-chain whose conditions fail is never recorded
//! and stays armed.

use std::collections::HashSet;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;

/// What the gate kept, and which spent once-chains it dropped.
#[derive(Debug)]
pub(crate) struct GatedActions {
    /// Surviving `(chain_id, action)` pairs, in their original order.
    pub(crate) actions: Vec<(i64, Action)>,
    /// Delays for `actions`, still index-aligned. Shorter than `actions`
    /// only where the input was (a missing index means `delay_ms == 0`).
    pub(crate) action_delays: Vec<i32>,
    /// Once-chains already spent for this entity, each listed once, in the
    /// order they first appeared.
    pub(crate) dropped_chain_ids: Vec<i64>,
}

/// Drop every action of a once-chain already in `spent`, and record the
/// once-chains that fire now.
///
/// A once-chain present in this call is recorded even if all its actions are
/// delayed: scheduling them is the fire, so a second event cannot schedule
/// them again. Non-once chains are never touched.
pub(crate) fn apply_once_gate(
    actions: Vec<(i64, Action)>,
    action_delays: Vec<i32>,
    engine: &ChainEngine,
    spent: &mut HashSet<i64>,
) -> GatedActions {
    // Fast path: most resolved lists carry no once-chain at all.
    if !actions.iter().any(|(id, _)| engine.is_once(*id)) {
        return GatedActions {
            actions,
            action_delays,
            dropped_chain_ids: Vec::new(),
        };
    }

    let delays_len = action_delays.len();
    let mut kept = Vec::with_capacity(actions.len());
    let mut kept_delays = Vec::with_capacity(delays_len);
    let mut dropped_chain_ids = Vec::new();
    let mut firing_now = Vec::new();
    for (i, (chain_id, action)) in actions.into_iter().enumerate() {
        if engine.is_once(chain_id) {
            if spent.contains(&chain_id) {
                if !dropped_chain_ids.contains(&chain_id) {
                    dropped_chain_ids.push(chain_id);
                }
                continue;
            }
            if !firing_now.contains(&chain_id) {
                firing_now.push(chain_id);
            }
        }
        // Keep the delays index-aligned with the survivors. Every index
        // before `i` was also inside the input delays, so the two vectors
        // stay the same length until the input delays run out; past that
        // point a missing entry already means `delay_ms == 0`.
        if i < delays_len {
            kept_delays.push(action_delays[i]);
        }
        kept.push((chain_id, action));
    }
    spent.extend(firing_now);
    GatedActions {
        actions: kept,
        action_delays: kept_delays,
        dropped_chain_ids,
    }
}

/// [`apply_once_gate`] against `entity_id`'s own fired-once set, with the
/// `once_spent` debug line per dropped chain. An entity that is not in the
/// space manager passes through ungated: there is nowhere to record the fire,
/// and nothing downstream can act for it anyway.
pub(crate) fn gate_for_entity(
    actions: Vec<(i64, Action)>,
    action_delays: Vec<i32>,
    entity_id: u32,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    engine: &ChainEngine,
) -> (Vec<(i64, Action)>, Vec<i32>) {
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return (actions, action_delays);
    };
    let gated = apply_once_gate(
        actions,
        action_delays,
        engine,
        &mut entity.fired_once_chains,
    );
    for chain_id in &gated.dropped_chain_ids {
        // Same target and field shape as `filtered_out` / `condition_failed`
        // in the engine's resolver, so one SigNoz query shows every reason a
        // matched chain did not run.
        tracing::debug!(
            target: "content.resolve",
            chain_id,
            entity_id,
            reason = "once_spent",
            "chain skipped: fire-once chain already fired for this entity"
        );
    }
    (gated.actions, gated.action_delays)
}
