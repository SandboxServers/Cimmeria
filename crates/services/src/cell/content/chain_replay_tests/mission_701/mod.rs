//! Mission 701 — "Reinforce Copplemann" (Castle, World 8). Live-DB
//! chain-replay guards for packets CA01 (arrival + Sgt. Gerschon) and
//! CA03 (mission body, steps 2399 → 2400 → 2401 → 2421), seeded in
//! `db/resources/Content/Seed/castle_701_chains.sql`.
//!
//! Split by lifecycle phase rather than kept in one file: [`arrival`]
//! pins CA01's offer/accept branches, [`body`] pins CA03's step
//! progression including the Livewire launcher and the deferred escort
//! walk, and [`restore`] pins the `player_loaded` re-binds that every
//! step depends on. All three share the helpers below.
//!
//! Every happy-path test asserts the **exact, ordered** resolved action
//! list via [`summarized`], not just "contains an AcceptMission" — the
//! bug shape these guard against is silent seed drift (a dropped
//! condition row, a reordered action, a changed dialog-set id), and a
//! `contains` assertion would sail past all three.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

mod arrival;
mod body;
mod dialog_buttons;
mod persistence;
mod restore;

/// `EArchetype` value for Jaffa — the discriminator for D-CA13's
/// Human/Jaffa split on Gerschon.
pub(super) const JAFFA: i64 = 8;
/// Any non-Jaffa archetype (Soldier). The `archetype neq 8` arm must
/// accept every one of these; 1 is the representative.
pub(super) const NON_JAFFA: i64 = 1;

/// Load one seeded chain from the live DB through the same
/// `build_chains_from_rows` pipeline the cell service uses at startup,
/// and register it in a fresh engine.
///
/// The `expect` on the `None` case is the regression reproduction: delete
/// the chain's seed rows and every test built on this helper fails here
/// rather than silently asserting against an empty action list.
pub(super) async fn engine_for(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = super::super::engine_loader::load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| {
            panic!(
                "chain {chain_id} must exist in seeded content_chains — a None here \
                 means castle_701_chains.sql was not loaded or its rows were removed"
            )
        });
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Fire a synthetic trigger event built from `ctx` and return what the
/// engine resolves.
pub(super) fn fire(
    engine: &ChainEngine,
    trigger_type: TriggerType,
    ctx: &ExecutionContext,
) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// Render one action as a short stable string.
///
/// Comparing rendered strings rather than matching each variant keeps the
/// exact-list assertions to one line each and makes a failure print the
/// whole diff (`["accept_mission(701)", …]` vs what was resolved) instead
/// of a bare count. The `UNEXPECTED(..)` arm is deliberate: an action verb
/// this mission never uses showing up in a 701 chain is itself the
/// failure, and it lands in the diff with its full `Debug`.
pub(super) fn summarize(action: &Action) -> String {
    match action {
        Action::AcceptMission { mission_id } => format!("accept_mission({mission_id})"),
        Action::CompleteMission { mission_id } => format!("complete_mission({mission_id})"),
        Action::DisplayDialog { dialog_id } => format!("display_dialog({dialog_id})"),
        Action::AddDialogSet {
            dialog_set_id,
            slot,
            ..
        } => format!("add_dialog_set({dialog_set_id}, slot={slot})"),
        Action::RemoveDialogSet {
            dialog_set_id,
            slot,
        } => format!("remove_dialog_set({dialog_set_id}, slot={slot})"),
        Action::AdvanceStep {
            mission_id,
            step_id,
        } => format!("advance_step({mission_id}, {step_id})"),
        Action::StartMinigame {
            minigame_type,
            on_victory_chains,
            ..
        } => format!("start_minigame({minigame_type}, victory={on_victory_chains:?})"),
        other => format!("UNEXPECTED({other:?})"),
    }
}

/// The rendered, ordered action list contributed by `chain_id`.
pub(super) fn summarized(resolved: &ResolvedActions, chain_id: i64) -> Vec<String> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, action)| summarize(action))
        .collect()
}

/// The `delay_ms` of each action contributed by `chain_id`, in the same
/// order as [`summarized`].
///
/// `ResolvedActions::action_delays` is a parallel vec indexed against
/// `actions`, so the index has to be taken before filtering by chain.
pub(super) fn delays(resolved: &ResolvedActions, chain_id: i64) -> Vec<i32> {
    resolved
        .actions
        .iter()
        .enumerate()
        .filter(|(_, (id, _))| *id == chain_id)
        .map(|(i, _)| resolved.action_delays.get(i).copied().unwrap_or(0))
        .collect()
}

/// Context for a `player_loaded` into Castle with no 701 state at all.
///
/// World 8 is named `Castle` in `worlds.sql`, which is what
/// `fire_player_loaded` passes as `world_name` and what every
/// `player_loaded` trigger row in `castle_701_chains.sql` keys on.
pub(super) fn castle_login_ctx() -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("world_name".to_string(), serde_json::json!("Castle"));
    ctx
}

/// Context for an `interact_tag` click on `tag` by a player of
/// `archetype`.
pub(super) fn interact_ctx(tag: &str, archetype: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Context for a `dialog_choice` on `dialog_id`.
///
/// No `archetype` param is set, and that is not an oversight:
/// `fire_dialog_choice` genuinely does not populate one
/// (`content/event_dispatch/dialog.rs`), so a chain gated on archetype
/// over a dialog choice could never match in production. Setting it here
/// would let such a chain pass a test it would fail in the game.
pub(super) fn dialog_choice_ctx(dialog_id: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx
}

/// Set `mission_{mission}_step_{step}_status` the way
/// `populate_mission_context` does.
pub(super) fn with_step(ctx: &mut ExecutionContext, mission: i32, step: i32, status: &str) {
    ctx.set_param(
        format!("mission_{mission}_step_{step}_status"),
        serde_json::json!(status),
    );
}

/// Set `mission_{mission}_status` the way `populate_mission_context` does.
pub(super) fn with_mission(ctx: &mut ExecutionContext, mission: i32, status: &str) {
    ctx.set_param(
        format!("mission_{mission}_status"),
        serde_json::json!(status),
    );
}

/// Convenience for the common "click this tag while this step is active"
/// shape.
pub(super) fn interact_at_step(
    tag: &str,
    archetype: i64,
    step: i32,
    status: &str,
) -> ExecutionContext {
    let mut ctx = interact_ctx(tag, archetype);
    with_step(&mut ctx, 701, step, status);
    ctx
}

/// Convenience for "choose on this dialog while this step is active".
pub(super) fn choice_at_step(dialog_id: i64, step: i32, status: &str) -> ExecutionContext {
    let mut ctx = dialog_choice_ctx(dialog_id);
    with_step(&mut ctx, 701, step, status);
    ctx
}

/// Re-export the trigger types the submodules use, so each file's import
/// block stays short.
pub(super) const INTERACT: TriggerType = TriggerType::InteractTag;
pub(super) const CHOICE: TriggerType = TriggerType::DialogChoice;
pub(super) const LOGIN: TriggerType = TriggerType::PlayerLoaded;
