//! Seed guard for mission 1521's Livewire pair (CS-03c1): starter 1374
//! (symbiote tank, step 4456) and victory 1375 (step 4456 to 4457).
//!
//! Mission 1521 is the Castle symbiote-tank mission. Its victory chain 1375 has
//! no trigger and is invoked only through `on_victory_chains`, so the
//! `step_status` condition on it is the only gate between a replayed
//! minigame win and an unguarded `advance_step`. These assertions are the
//! static half of that guard. The behavioural half (1375 at 4457, or with
//! 1521 completed, emits nothing) lands with CS-03c2.
//!
//! Each assertion is read straight from the seed, and each fails when its
//! row is reverted: a condition row of 1375, a non-zero `delay_ms` on one of
//! its actions, a trigger row on 1375, or a change to 1374's game name,
//! difficulty or victory list.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::conditions::{Condition, StepStatusValue};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Starter: the symbiote tank interact, gated on step 4456.
const STARTER: i32 = 1374;
/// Victory: the Livewire win, advancing 1521 from 4456 to 4457.
const VICTORY: i32 = 1375;
const MISSION: i32 = 1521;
const STEP: i32 = 4456;
const WORLD_CASTLE: i32 = 8;

/// Count rows in `resources.content_actions` for `chain_id` matching `sql`.
async fn count_rows(pool: &PgPool, sql: &'static str, chain_id: i32) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .bind(chain_id)
        .fetch_one(pool)
        .await
        .expect("count query must succeed")
}

/// **Guard (a): the victory chain carries conditions.** `load_single_chain_for_test`
/// reads `resources.content_conditions`, so an empty list here means the
/// `world` and `step_status` rows of 1375 are gone.
#[tokio::test]
async fn live_db_chain_1375_loads_with_conditions() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, VICTORY)
        .await
        .expect("DB query for chain 1375 must succeed")
        .expect("chain 1375 must exist in seeded content_chains");

    assert!(
        !chain.conditions.is_empty(),
        "chain 1375 must carry conditions; an empty list makes its \
         advance_step unguarded: {:?}",
        chain.conditions,
    );
}

/// **Guard (b): the victory conditions are the world and the exact step gate.**
/// The `world 8` check keeps the chain in Castle; the `step_status` check is
/// the one that stops a replayed win from advancing 1521 again.
#[tokio::test]
async fn live_db_chain_1375_conditions_pin_world_8_and_step_4456_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, VICTORY)
        .await
        .expect("DB query for chain 1375 must succeed")
        .expect("chain 1375 must exist in seeded content_chains");

    assert!(
        chain.conditions.iter().any(|c| matches!(
            c,
            Condition::World { world_id, .. } if *world_id == WORLD_CASTLE
        )),
        "chain 1375 must require world 8; got {:?}",
        chain.conditions,
    );
    assert!(
        chain.conditions.iter().any(|c| matches!(
            c,
            Condition::StepStatus {
                mission_id: MISSION,
                step_id: STEP,
                expected_status: StepStatusValue::Active,
                ..
            }
        )),
        "chain 1375 must require mission 1521 step 4456 to be active; got {:?}",
        chain.conditions,
    );
}

/// **Guard (c): no delayed action on the victory chain.** `execute_actions`
/// queues a non-zero `delay_ms` and runs it later with no re-check, so a
/// delayed `advance_step` would outlive the gate that protects it.
#[tokio::test]
async fn live_db_chain_1375_has_no_delayed_actions() {
    let pool = require_db_or_skip!();
    let delayed = count_rows(
        &pool,
        "SELECT count(*) FROM resources.content_actions \
         WHERE chain_id = $1 AND delay_ms <> 0",
        VICTORY,
    )
    .await;
    assert_eq!(
        delayed, 0,
        "chain 1375 has {delayed} action(s) with delay_ms <> 0; a delayed \
         action runs after its step gate has been checked",
    );
}

/// **Guard (d): the victory chain has no trigger row.** Its only route in is
/// the synthetic `__direct_invoke_1375` event that `on_victory_chains` uses,
/// so a `content_triggers` row would open a second, ungated route to the
/// advance.
#[tokio::test]
async fn live_db_chain_1375_has_no_trigger_rows() {
    let pool = require_db_or_skip!();
    let triggers = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM resources.content_triggers WHERE chain_id = $1",
    )
    .bind(VICTORY)
    .fetch_one(&pool)
    .await
    .expect("count content_triggers");
    assert_eq!(
        triggers, 0,
        "chain 1375 must have no content_triggers row (direct invoke only); \
         got {triggers}",
    );
}

/// **Guard (e): the starter's minigame is exactly Livewire, difficulty 1,
/// and hands off to 1375.** The game name compares with `==`, never a
/// case-insensitive match. The loader defaults an omitted difficulty to 1,
/// so the raw `params` must also set it explicitly.
#[tokio::test]
async fn live_db_chain_1374_starts_exact_livewire_difficulty_1_into_1375() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, STARTER)
        .await
        .expect("DB query for chain 1374 must succeed")
        .expect("chain 1374 must exist in seeded content_chains");

    let starts: Vec<&Action> = chain
        .actions
        .iter()
        .filter(|a| matches!(a, Action::StartMinigame { .. }))
        .collect();
    assert_eq!(
        starts.len(),
        1,
        "chain 1374 must carry exactly one StartMinigame; got {:?}",
        chain.actions,
    );
    assert!(
        matches!(
            starts[0],
            Action::StartMinigame {
                minigame_type,
                difficulty: 1,
                on_victory_chains,
                ..
            } if minigame_type == "Livewire" && on_victory_chains == &vec![i64::from(VICTORY)]
        ),
        "chain 1374 must start Livewire at difficulty 1 with on_victory_chains \
         [1375]; got {:?}",
        starts[0],
    );

    let difficulty: Option<String> = sqlx::query_scalar(
        "SELECT params->>'difficulty' FROM resources.content_actions \
         WHERE chain_id = $1 AND action_type::text = 'start_minigame'",
    )
    .bind(STARTER)
    .fetch_optional(&pool)
    .await
    .expect("select start_minigame params");
    assert_eq!(
        difficulty.as_deref(),
        Some("1"),
        "chain 1374's raw params must set difficulty explicitly to 1; the \
         loader defaults an omitted value to 1 and would hide a removal",
    );
}
