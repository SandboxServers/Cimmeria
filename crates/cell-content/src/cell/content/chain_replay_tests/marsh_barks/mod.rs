//! DU-07 — Col. Marsh companion barks, chains 1176-1178 in
//! `castle_cellblock_chains.sql`.
//!
//! Three `npc_bark` rows speak dialog 5019's screens 96351, 96352 and
//! 96354 into the escorting player's chat window at three points on the
//! Castle_CellBlock escape route. Dialog 5019 itself must never be
//! displayed (its last three screens are excluded "Future Self"
//! content), so these lines have no other delivery route.
//!
//! One module per chain, plus one for the guards that are not about any
//! single chain:
//!
//! - [`escort_start`] — chain 1176 on the ring ride, and the one test in
//!   the packet that runs the executor.
//! - [`mess_hall`] — chain 1177 on the `Castle_Cellblock.Region3`
//!   crossing.
//! - [`second_flank`] — chain 1178 on the `Castle_Cellblock.Region5`
//!   crossing, including its co-gating with chain 1083.
//! - [`seed_shape`] — relog, and the two live-DB guards over every
//!   `npc_bark` row in the seed rather than over one chain.
//!
//! What the two halves of each chain module are guarding:
//!
//! - The **positive** cases pin that each seed row survives the loader
//!   and resolves to an `Action::NpcBark` carrying the right `screen_id`.
//!   A row that names the wrong screen still resolves, so the screen id
//!   is asserted rather than the action kind.
//! - The **negative** cases are the load-bearing half. The engine has no
//!   fire-once primitive — `content_triggers.once` is read out of the DB
//!   and never consulted again (`loader/mod.rs`, `engine_loader.rs`) —
//!   so "at most once per mission run" rests entirely on each chain's
//!   mission/step gate. Each chain therefore gets the adjacent
//!   wrong-state case its own gate is supposed to refuse: the phase not
//!   yet reached, and the phase already passed.
//!
//! Chain-level guards live here rather than in [`super::npc_bark`],
//! which owns the verb itself against a sentinel chain.
//!
//! This module holds only what more than one sibling needs. The
//! executor staging (`SpaceManager` fixture, wire reader) lives in
//! [`escort_start`] beside its single caller.

mod escort_start;
mod mess_hall;
mod second_flank;
mod seed_shape;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::assert_no_deferred_actions;

/// Escort start, on the ring ride to the topside route.
const CHAIN_MOVE_OUT: i32 = 1176;
/// Mess Hall threshold (`Castle_Cellblock.Region3`).
const CHAIN_MESS_HALL: i32 = 1177;
/// Hallway05 threshold (`Castle_Cellblock.Region5`).
const CHAIN_HALLWAY05: i32 = 1178;

/// Dialog 5019's screens, in `db/resources/Dialogs/Seed/dialog_screens.sql`.
const SCREEN_MOVE_OUT: i32 = 96351;
const SCREEN_MESS_HALL: i32 = 96352;
const SCREEN_HALLWAY05: i32 = 96354;

/// `speakers.speaker_id 261`, the spelling dialog 2309 ships on its Marsh
/// lines in this same phase.
const MARSH_SPEAKER: &str = "Col. Marsh";
/// `CHAN_say`. The loader accepts no other channel.
const CHAN_SAY: u8 = 0;

/// Region keys, byte-identical to `point_sets.sql`. Note the lowercase
/// `b` — the same seed file also contains `Castle_CellBlock.Region8` with
/// a capital one, and trigger matching is an exact string compare.
const REGION_MESS_HALL: &str = "Castle_Cellblock.Region3";
const REGION_HALLWAY05: &str = "Castle_Cellblock.Region5";

async fn load(pool: &PgPool, chain_id: i32) -> Chain {
    load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains and assemble \
                 successfully — a None here means the trigger or action row was \
                 rejected at load (check the npc_bark params)"
            )
        })
}

fn engine_with(chain: Chain) -> ChainEngine {
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Resolve a `teleport_in` arrival with the given mission-step context.
fn resolve_teleport_in(
    engine: &ChainEngine,
    region_id: i32,
    params: &[(&str, &str)],
) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(region_id));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Resolve a region crossing with the given mission-status context.
fn resolve_region_enter(
    engine: &ChainEngine,
    region_key: &str,
    params: &[(&str, &str)],
) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Every `NpcBark` screen id the given chain resolved.
fn bark_screens(resolved: &ResolvedActions, chain_id: i64) -> Vec<i32> {
    resolved
        .actions
        .iter()
        .filter_map(|(id, action)| match action {
            Action::NpcBark { screen_id, .. } if *id == chain_id => Some(*screen_id),
            _ => None,
        })
        .collect()
}

/// Assert the chain resolved exactly one bark, with the right screen,
/// speaker, channel and no delay.
fn assert_single_bark(resolved: &ResolvedActions, chain_id: i64, screen_id: i32) {
    let barks: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, action)| *id == chain_id && matches!(action, Action::NpcBark { .. }))
        .map(|(_, action)| action)
        .collect();
    assert_eq!(
        barks.len(),
        1,
        "chain {chain_id} must resolve exactly one NpcBark; got {}. Zero \
         means either the gate refused the happy path or the loader dropped \
         the row (a bad `speaker`/`channel` param is rejected, not defaulted \
         through). Resolved: {:?}",
        barks.len(),
        resolved.actions,
    );
    match barks[0] {
        Action::NpcBark {
            screen_id: got,
            speaker,
            channel,
        } => {
            assert_eq!(
                *got, screen_id,
                "chain {chain_id} must speak dialog 5019 screen {screen_id}, not {got}"
            );
            assert_eq!(
                speaker, MARSH_SPEAKER,
                "chain {chain_id}'s chat prefix must be the speaker-261 spelling \
                 dialog 2309 ships in this same phase"
            );
            assert_eq!(
                *channel, CHAN_SAY,
                "chain {chain_id} must ride CHAN_say; the loader accepts no other \
                 channel, so anything else means the row was rewritten"
            );
        }
        other => panic!("filtered for NpcBark but got {other:?}"),
    }
    assert_no_deferred_actions(resolved, chain_id);
}

/// Assert the chain resolved nothing at all.
fn assert_refused(resolved: &ResolvedActions, chain_id: i64, why: &str) {
    let actions: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, a)| a)
        .collect();
    assert!(
        actions.is_empty(),
        "chain {chain_id} must NOT resolve when {why} — the engine has no \
         fire-once primitive, so this gate is the only thing stopping the \
         line repeating. Got {actions:?}",
    );
}
