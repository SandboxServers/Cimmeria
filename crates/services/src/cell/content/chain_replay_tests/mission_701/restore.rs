//! Relog restoration for mission 701 (chains 1240-1243).
//!
//! Per-player dialog binds live in `available_interactions` on the cell
//! entity and are not persisted, so every step of 701 that needs
//! Copplemann to be clickable needs a `player_loaded` chain to re-paint
//! the bind. Step 2401 is the odd one: there is no bind to restore
//! during the escort, but the deferred timer that ends it is scrubbed on
//! disconnect, so chain 1242 re-arms it instead.
//!
//! Each test pins one step's restore and asserts the neighbouring steps
//! do NOT also fire, because two restore chains firing on one login
//! would push duplicate binds and, for 1242, arm a second escort timer.

use cimmeria_content_engine::context::ExecutionContext;

use super::{
    castle_login_ctx, delays, engine_for, fire, summarized, with_mission, with_step, LOGIN,
};
use crate::test_support::require_db_or_skip;

/// Every step of 701 paired with the chain that restores it on login.
///
/// Only the four in-progress steps appear here. The not-yet-accepted case
/// has no restore chain of its own: chain 1201 already fires on
/// `player_loaded` when `mission_status 701` is `not_active`, and it is
/// covered in the [`super::arrival`] module.
const RESTORES: [(i32, i32); 4] = [(2399, 1240), (2400, 1241), (2401, 1242), (2421, 1243)];

/// Chain 1240 — logging in mid-step-2399 re-binds the Copplemann
/// in-progress topic so she is clickable and chain 1231 can fire.
#[tokio::test]
async fn chain_1240_restores_copplemann_topic_on_step_2399() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1240).await;

    let mut ctx = castle_login_ctx();
    with_mission(&mut ctx, 701, "active");
    with_step(&mut ctx, 701, 2399, "active");

    assert_eq!(
        summarized(&fire(&engine, LOGIN, &ctx), 1240),
        vec!["add_dialog_set(3062, slot=48)"],
        "a login on step 2399 must re-bind the in-progress topic to \
         Copplemann's template",
    );
}

/// Chain 1241 — same bind on step 2400, which is what makes the Livewire
/// launcher reachable after a relog. Losing this strands the player with
/// an unclickable Copplemann and no way to free her.
#[tokio::test]
async fn chain_1241_restores_copplemann_topic_on_step_2400() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1241).await;

    let mut ctx = castle_login_ctx();
    with_mission(&mut ctx, 701, "active");
    with_step(&mut ctx, 701, 2400, "active");

    assert_eq!(
        summarized(&fire(&engine, LOGIN, &ctx), 1241),
        vec!["add_dialog_set(3062, slot=48)"],
        "a login on step 2400 must re-bind the in-progress topic so the \
         Livewire launcher stays reachable",
    );
}

/// Chain 1242 — the escort re-arm. A logout during the 10.5 s walk
/// scrubs the deferred queue (`SpaceManager::disconnect_entity`), so
/// without this chain step 2401 has nothing left that can advance it and
/// the mission is permanently stuck. The re-armed actions must be
/// identical to chain 1235's, delays included.
#[tokio::test]
async fn chain_1242_rearms_the_escort_timer_on_step_2401() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1242).await;

    let mut ctx = castle_login_ctx();
    with_mission(&mut ctx, 701, "active");
    with_step(&mut ctx, 701, 2401, "active");

    let resolved = fire(&engine, LOGIN, &ctx);
    assert_eq!(
        summarized(&resolved, 1242),
        vec!["advance_step(701, 2421)", "add_dialog_set(3063, slot=48)"],
        "the re-arm must reproduce chain 1235's escort actions exactly",
    );
    assert_eq!(
        delays(&resolved, 1242),
        vec![10_500, 10_500],
        "the re-armed escort must keep the authored delay — an immediate \
         advance on login would skip the walk entirely",
    );
}

/// Chain 1243 — the turn-in "?" comes back on login so the player can
/// hand 701 in after a relog.
#[tokio::test]
async fn chain_1243_restores_the_turn_in_topic_on_step_2421() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1243).await;

    let mut ctx = castle_login_ctx();
    with_mission(&mut ctx, 701, "active");
    with_step(&mut ctx, 701, 2421, "active");

    assert_eq!(
        summarized(&fire(&engine, LOGIN, &ctx), 1243),
        vec!["add_dialog_set(3063, slot=48)"],
        "a login on step 2421 must re-bind the turn-in topic (3063)",
    );
}

/// Each restore chain must fire on its own step and no other. Two
/// restores on one login would double-bind (and for 1242, arm a second
/// escort timer that advances the mission a second time).
#[tokio::test]
async fn each_restore_chain_fires_only_on_its_own_step() {
    let pool = require_db_or_skip!();

    for (own_step, chain_id) in RESTORES {
        let engine = engine_for(&pool, chain_id).await;

        for (other_step, _) in RESTORES {
            if other_step == own_step {
                continue;
            }
            let mut ctx = castle_login_ctx();
            with_mission(&mut ctx, 701, "active");
            with_step(&mut ctx, 701, other_step, "active");

            assert!(
                summarized(&fire(&engine, LOGIN, &ctx), chain_id as i64).is_empty(),
                "chain {chain_id} restores step {own_step} and must not fire \
                 while the player is on step {other_step}",
            );
        }
    }
}

/// No restore chain may fire once 701 is finished — a completed mission
/// must not leave an indicator floating over Copplemann forever.
/// `MissionInstance::complete()` marks the final step `completed`, which
/// is the state asserted here.
#[tokio::test]
async fn no_restore_chain_fires_after_701_completes() {
    let pool = require_db_or_skip!();

    for (step, chain_id) in RESTORES {
        let engine = engine_for(&pool, chain_id).await;

        let mut ctx = castle_login_ctx();
        with_mission(&mut ctx, 701, "completed");
        with_step(&mut ctx, 701, step, "completed");

        assert!(
            summarized(&fire(&engine, LOGIN, &ctx), chain_id as i64).is_empty(),
            "chain {chain_id} must not fire on login after 701 is completed",
        );
    }
}

/// Every restore chain is scoped to world 8. A login into the Cellblock
/// (where the player spends missions 622-688) must not paint Castle's
/// indicators.
#[tokio::test]
async fn restore_chains_are_scoped_to_the_castle_world() {
    let pool = require_db_or_skip!();

    for (step, chain_id) in RESTORES {
        let engine = engine_for(&pool, chain_id).await;

        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        with_mission(&mut ctx, 701, "active");
        with_step(&mut ctx, 701, step, "active");

        assert!(
            summarized(&fire(&engine, LOGIN, &ctx), chain_id as i64).is_empty(),
            "chain {chain_id} must only fire on a login into Castle",
        );
    }
}
