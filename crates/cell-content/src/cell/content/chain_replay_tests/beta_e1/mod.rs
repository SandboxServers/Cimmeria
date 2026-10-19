//! Beta Site E1 (world 23, `Beta_Site_Evo_1`) chain-replay guards, Union
//! Beta E1 campaign (docs/analysis/union-zone-planning/beta-work-packets.md).
//! Each mission packet owns one seed file, `db/resources/Content/Seed/`
//! `beta_e1_<slug>_chains.sql`, and one submodule here. UB-01b owns
//! `beta_e1_travel_chains.sql` (chains 23001-23003) in [`travel`].
//!
//! The helpers below are copied from `mission_701/mod.rs`, with the Beta
//! constants and context builders the Beta packets share. Every
//! happy-path test asserts the exact, ordered action list, and every
//! exclusivity test loads the whole seed, so a sibling chain that fires
//! on the same event fails the test.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

mod m1423;
mod travel;

/// The Beta Site E1 world name, as `worlds.sql` and `player_loaded` key on it.
pub(super) const BETA: &str = "Beta_Site_Evo_1";
/// `EArchetype` values for the Union Human gate (`gte 1` and `lte 4`).
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) const HUMANS: [i64; 4] = [1, 2, 3, 4];
/// `EArchetype` value for Asgard.
pub(super) const ASGARD: i64 = 5;
/// `EArchetype` value for Goa'uld (Praxis). Not a Union race.
pub(super) const GOAULD: i64 = 6;
/// `EArchetype` value for Free Jaffa (SGU Jaffa, Union).
pub(super) const FREE_JAFFA: i64 = 7;
/// `EArchetype` value for Jaffa (Praxis). Not a Union race.
pub(super) const JAFFA: i64 = 8;
/// Every non-Human archetype (5-8: Asgard, Goa'uld, Free Jaffa, Jaffa): use to
/// prove a Human-only gate.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) const NON_HUMANS: [i64; 4] = [5, 6, 7, 8];
/// Every archetype, 0 (Any) through 8: filter out the ones a gate admits to
/// prove a race gate.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) const ALL_ARCHETYPES: [i64; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 8];

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
                 means beta_e1_*_chains.sql was not loaded or its rows were removed"
            )
        });
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// The whole seeded engine, every Beta chain and every chain in the other
/// seed files, so "resolves exactly these" is a statement about the seed.
pub(super) async fn whole_seed(pool: &PgPool) -> ChainEngine {
    super::super::engine_loader::build_engine(Some(pool)).await
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

/// Every chain id that contributed at least one action.
pub(super) fn chains_matched(resolved: &ResolvedActions) -> std::collections::BTreeSet<i64> {
    resolved.actions.iter().map(|(id, _)| *id).collect()
}

/// Render one action as a short stable string.
///
/// Comparing rendered strings rather than matching each variant keeps the
/// exact-list assertions to one line each and makes a failure print the
/// whole diff. The `UNEXPECTED(..)` arm is deliberate: an action verb a
/// Beta chain never uses showing up is itself the failure.
pub(super) fn summarize(action: &Action) -> String {
    match action {
        Action::AcceptMission { mission_id } => format!("accept_mission({mission_id})"),
        Action::CompleteMission { mission_id } => format!("complete_mission({mission_id})"),
        Action::CompleteObjective {
            mission_id,
            objective_id,
        } => format!("complete_objective({mission_id}, {objective_id})"),
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
        Action::GrantItem { item_id, count, .. } => format!("add_item({item_id}, {count})"),
        Action::RemoveItem { item_id, count } => format!("remove_item({item_id}, {count})"),
        Action::IncrementCounter { counter_name, .. } => {
            format!("increment_counter({counter_name})")
        }
        Action::GrantStargateAddress { stargate_id } => {
            format!("grant_stargate_address({stargate_id})")
        }
        Action::MoveEntity { destination, .. } => format!(
            "move_entity({}, {}, {})",
            destination[0], destination[1], destination[2]
        ),
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

/// Context for a `player_loaded` into Beta, optionally with the archetype
/// the login sets (`None` leaves it unset, which reads as Any).
pub(super) fn login_ctx(world: &str, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("world_name".to_string(), serde_json::json!(world));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// Context for an `interact_tag` click on `tag` by a player of `archetype`.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn interact_ctx(tag: &str, archetype: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Context for a `dialog_choice` on `dialog_id` by a player of `archetype`.
///
/// CS-01b sets `archetype` on dialog events (`event_dispatch/dialog.rs`),
/// so a chain gated on race over a dialog choice matches in production.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn choice_ctx(dialog_id: i64, archetype: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Context for an `item_use` of `item_id` by a player of `archetype`.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn item_use_ctx(item_id: i64, archetype: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("item_id".to_string(), serde_json::json!(item_id));
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Context for an `enter_region` on `region_key` by a player of `archetype`.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn region_ctx(region_key: &str, archetype: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Context for a `mission_completed` of `mission_id` by a player of `archetype`.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn completed_ctx(mission_id: i64, archetype: i64) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(mission_id));
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    ctx
}

/// Set `mission_{mission}_status` the way `populate_mission_context` does.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn with_mission(ctx: &mut ExecutionContext, mission: i32, status: &str) {
    ctx.set_param(
        format!("mission_{mission}_status"),
        serde_json::json!(status),
    );
}

/// Set `mission_{mission}_step_{step}_status` the way `populate_mission_context` does.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn with_step(ctx: &mut ExecutionContext, mission: i32, step: i32, status: &str) {
    ctx.set_param(
        format!("mission_{mission}_step_{step}_status"),
        serde_json::json!(status),
    );
}

/// Set `mission_{mission}_obj_{obj}_status` the way `populate_mission_context` does.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn with_obj(ctx: &mut ExecutionContext, mission: i32, obj: i32, status: &str) {
    ctx.set_param(
        format!("mission_{mission}_obj_{obj}_status"),
        serde_json::json!(status),
    );
}

/// Set `counter_{name}`, the key the `counter` condition reads.
#[allow(dead_code)] // used by later Beta packets (UB-10+)
pub(super) fn with_counter(ctx: &mut ExecutionContext, name: &str, value: i64) {
    ctx.set_param(format!("counter_{name}"), serde_json::json!(value));
}
