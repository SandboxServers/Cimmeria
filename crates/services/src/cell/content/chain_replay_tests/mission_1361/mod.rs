//! Mission 1361 — Meet The Praxis (packet H31,
//! `db/resources/Content/Seed/harset_opcore_chains.sql` chains 6511-6527).
//!
//! Six strictly ordered talk steps across two worlds:
//!
//! | Step | Beat | World | Dialog | Chains |
//! |---|---|---|---|---|
//! | (offer) | Marsh briefing | 68 | 4457 | 6511, 6512, 6513 |
//! | 4040 | Moh'katan asks for weapon samples | 68 | 4458 | 6514, 6515 |
//! | 4041 | Convince Hansen | **57** | 4459 -> 4460 | 6516, 6518 |
//! | 4042 | Deliver the samples | 68 | 4466 | 6519, 6520 |
//! | 4043 | Ba'al | 68 | 4461 | 6521, 6522 |
//! | 4693 | Anat | 68 | 4462 -> 4463 | 6523, 6525 |
//! | 4694 | Return to Marsh | 68 | 4465 | 6526, 6527 |
//!
//! Chain ids 6517 and 6524 are deliberately unused — Hansen's and Anat's
//! beats use the *bind path* (no `interact_tag` chain). Their step is
//! driven by a button on a dialog that only the bind can open:
//! `fire_interact_tag` short-circuits `interactions::handle_interact`,
//! which is the sole code that opens a bound dsm's dialog, so an
//! `interact_tag` chain on either NPC would suppress 4459/4462 and the
//! `dialog_choice` chains 6518/6525 could never fire. See the seed file's
//! note (C).
//!
//! The acceptance trio (6511-6513) ships **disabled**: step 4041 is in
//! world 57 and its neighbours are in world 68, so the mission requires a
//! working 68 -> 57 crossing, and the only one — chain 6007 in
//! `harset_space_chains.sql` — is itself disabled pending an M0
//! coordinate pin. Shipping acceptance live against a dark return leg
//! would soft-stick every player at step 4041 with no recovery (there is
//! no `fail_objective` executor arm and no chain-authorable abandon).
//!
//! Five guard families, one file each. This module holds only the shared
//! context builders, constants and resolve helpers they all use.
//!
//! 1. [`progression`] — **ordered progression.** Each step chain resolves its exact action
//!    list on its own step and nothing on any other step.
//! 2. [`disjointness`] — **cross-chain disjointness.** Three chains key on
//!    `interact_tag 'CmdCenter_Marsh'` and `resolve_event` APPENDS every
//!    matching chain's actions with no first-match break, so one
//!    right-click could otherwise run two of them. Asserted through
//!    `build_engine`, against the whole seeded DB, because a per-chain
//!    test cannot see a collision by construction.
//! 3. [`acceptance`] — **the parked acceptance path**, including the biconditional with
//!    chain 6007 so M0 cannot flip one without the other.
//! 4. [`bind_hygiene`] — no template slot ever holds two live binds, and
//!    every indicator a step sets is cleared and re-paintable on relog.
//! 5. [`handoff`] — **playtest finding H9 in its `player_loaded` form.** A chain keyed
//!    on an edge never fires when its gate opens while the player is
//!    already past that edge. Every bind, offer and restore chain in the
//!    packet is walked for that shape: the ones whose gate opens in the
//!    same world must bind in-chain (or carry a second trigger row), and
//!    the two cross-world ones must NOT, because the bind dies with the
//!    cell entity. Plus an anti-vacuity guard proving the negatives in
//!    families 1-4 fail on their *condition*, not on a missing trigger key.

use std::collections::HashMap;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::{
    build_engine, load_chain_expansions_for_test, load_single_chain_for_test,
};
use crate::test_support::require_db_or_skip;

/// `resources.worlds.world_id` for `Harset_CmdCenter` (Marsh, Moh'katan,
/// Ba'al, Anat) and `Harset` (Hansen).
const CMD_CENTER: i32 = 68;
const HARSET: i32 = 57;

/// `EArchetype` values from `entities/defs/enumerations.xml:364,366`.
/// 1361 is the Human/OP-CORE arrival mission, so both of these are
/// excluded by the offer chains.
const ARCHETYPE_JAFFA: i64 = 8;
const ARCHETYPE_GOAULD: i64 = 6;
/// Any Human archetype — `ARCHETYPE_Soldier`. The gate is two `neq` rows,
/// not an `eq`, because "Human" is four archetypes.
const ARCHETYPE_SOLDIER: i64 = 5;

/// Every step id in 1361, in play order. Used to prove each step chain is
/// silent on every step but its own.
const STEPS: [&str; 6] = ["4040", "4041", "4042", "4043", "4693", "4694"];

/// Build a context with 1361 on `current_step`, in `world_id`.
///
/// Mirrors `populate_mission_context`: the current step is written
/// `active` and every other step is left absent, which the evaluator reads
/// through its `unwrap_or("not_active")` fallback.
fn praxis_ctx(world_id: i32, current_step: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(world_id);
    ctx.set_param(
        "mission_1361_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        format!("mission_1361_step_{current_step}_status"),
        serde_json::json!("active"),
    );
    ctx
}

fn with_tag(mut ctx: ExecutionContext, tag: &str) -> ExecutionContext {
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx
}

fn with_dialog(mut ctx: ExecutionContext, dialog_id: i32) -> ExecutionContext {
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx
}

fn with_world_name(mut ctx: ExecutionContext, name: &str) -> ExecutionContext {
    ctx.set_param("world_name".to_string(), serde_json::json!(name));
    ctx
}

/// Load one chain, or fail with a message that separates "row missing"
/// from "row present but the loader rejected it".
async fn load(pool: &sqlx::PgPool, chain_id: i32) -> Chain {
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
async fn resolve_one(
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
fn actions_of(resolved: &ResolvedActions) -> Vec<Action> {
    resolved.actions.iter().map(|(_, a)| a.clone()).collect()
}

// Guard families, one file each. `cargo fmt` sorts these, so append
// nothing here that depends on order.
mod acceptance;
mod bind_hygiene;
mod disjointness;
mod handoff;
mod progression;
