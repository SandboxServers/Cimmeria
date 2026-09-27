//! `entity_health_below` chain-replay guard (Harset H04).
//!
//! Like [`super::grant_xp`], this module seeds its own chain: there are
//! zero `entity_health_below` rows in `db/resources/Content/Seed/` today
//! — the trigger is new and its first consumer is Harset mission 1325
//! (the Rin'la duel, packet H21), which has not been authored yet.
//! Pinning it against a sentinel chain is the only way to guard the
//! loader arm before content reaches it; when the real 1325 rows land,
//! these assertions move to `mission_1325.rs`.
//!
//! What a revert breaks:
//!
//! - Removing the `"entity_health_below"` arm from
//!   `crates/content-engine/src/loader/trigger.rs` makes `convert_trigger`
//!   return `None`, the chain is dropped at load, and
//!   `load_single_chain_for_test` hands back a chain the engine never
//!   matches — every positive assertion here fails.
//! - Loosening `Trigger::matches` (dropping either half of
//!   `pct_before > pct && pct_after <= pct`) fails one of the two
//!   negative assertions.
//!
//! The chain's shape is the one H21 will author: the ritual ends with
//! `set_npc_ai_state submit` plus `advance_step`, world-state mutation
//! first and mission-state mutation last (the ordering matters the
//! moment anyone adds a mission verb that re-enters the chain engine to
//! the same chain).
//!
//! Sentinel id range: `0x7004_0000..0x7004_ffff` (Harset H04's
//! allocation). Cleanup deletes the exact ids inserted, never a range,
//! and runs before seeding as well as after loading, so a previous
//! aborted run cannot poison this one.
//!
//! **The range isolates this module from other modules, not these tests
//! from each other** — all of them seed the same `TEST_CHAIN_ID` and
//! would collide on its primary key if run concurrently. That is safe
//! only because the `ci-live-db` nextest profile serialises every test
//! (`threads-required = "num-test-threads"`); with plain `cargo test`,
//! pass `-- --test-threads=1`.

use cimmeria_content_engine::actions::{Action, NpcAiStateAction};
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Sentinel `content_chains.chain_id`. Fits in `i32` (the column type).
const TEST_CHAIN_ID: i32 = 0x7004_1000;
/// Sentinel entity tag — namespaced so it can never collide with a real
/// `content_triggers.event_key`.
const TEST_TAG: &str = "CIMMERIA_TEST_H04_DUEL_NPC";
/// The authored threshold. 50 so the acceptance case (60% → 40%) sits
/// symmetrically either side of it.
const TEST_PCT: i32 = 50;
/// Stand-ins for mission 1325 / step 3959. Values only have to survive
/// the loader — these tests resolve, they don't execute.
const TEST_MISSION: i32 = 0x7004_1001;
const TEST_STEP: i32 = 0x7004_1002;

/// Seed the sentinel chain with the authored `event_key`.
async fn seed_sentinel_chain(pool: &PgPool) {
    seed_sentinel_chain_with_key(pool, &format!("{TEST_TAG}:{TEST_PCT}")).await;
}

/// Same, with the trigger's `event_key` under the caller's control, so a
/// malformed key can be pushed through the real loader.
async fn seed_sentinel_chain_with_key(pool: &PgPool, event_key: &str) {
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'entity_health_below chain-replay sentinel', 'space', NULL, true, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .execute(pool)
    .await
    .expect("sentinel content_chains insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, 'entity_health_below', $2, 'player', false, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(event_key)
    .execute(pool)
    .await
    .expect("sentinel content_triggers insert must succeed");

    // The one-shot guard is the author's job (`content_triggers.once` is
    // dead); this is the condition H21 will carry so a relog + re-cross
    // after the step has advanced resolves nothing.
    sqlx::query(
        "INSERT INTO resources.content_conditions \
         (chain_id, condition_type, target_id, target_key, operator, value, sort_order) \
         VALUES ($1, 'step_status', $2, $3, 'eq', 'active', 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(TEST_MISSION)
    .bind(TEST_STEP.to_string())
    .execute(pool)
    .await
    .expect("sentinel content_conditions insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'set_npc_ai_state', NULL, $2, '{\"state\": \"submit\"}'::jsonb, 0, 0)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(TEST_TAG)
    .execute(pool)
    .await
    .expect("sentinel set_npc_ai_state insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'advance_step', $2, $3, '{}'::jsonb, 0, 1)",
    )
    .bind(TEST_CHAIN_ID)
    .bind(TEST_MISSION)
    .bind(TEST_STEP.to_string())
    .execute(pool)
    .await
    .expect("sentinel advance_step insert must succeed");
}

/// Delete by exact chain id, children first (FK order).
async fn cleanup_sentinel_chain(pool: &PgPool) {
    for stmt in [
        "DELETE FROM resources.content_actions WHERE chain_id = $1",
        "DELETE FROM resources.content_conditions WHERE chain_id = $1",
        "DELETE FROM resources.content_triggers WHERE chain_id = $1",
        "DELETE FROM resources.content_chains WHERE chain_id = $1",
    ] {
        sqlx::query(stmt)
            .bind(TEST_CHAIN_ID)
            .execute(pool)
            .await
            .expect("sentinel cleanup must succeed");
    }
}

/// Seed, load, immediately clean up, and hand back an engine holding the
/// assembled chain. Cleanup runs before any assertion so a failing test
/// can't leave a live sentinel chain in the shared test database.
async fn engine_with_sentinel_chain(pool: &PgPool) -> ChainEngine {
    cleanup_sentinel_chain(pool).await;
    seed_sentinel_chain(pool).await;
    let loaded = load_single_chain_for_test(pool, TEST_CHAIN_ID).await;
    cleanup_sentinel_chain(pool).await;

    let chain = loaded
        .expect("DB query for the sentinel chain must succeed")
        .expect(
            "sentinel chain must load — a None here means convert_trigger \
             has no \"entity_health_below\" arm and dropped the trigger row",
        );
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

/// Build the event the damage path emits, with the step gate satisfied.
fn crossing_event(tag: &str, pct_before: f64, pct_after: f64) -> (TriggerEvent, ExecutionContext) {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx.set_param("pct_before".to_string(), serde_json::json!(pct_before));
    ctx.set_param("pct_after".to_string(), serde_json::json!(pct_after));
    ctx.set_param(
        format!("mission_{TEST_MISSION}_step_{TEST_STEP}_status"),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityHealthBelow,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    (event, ctx)
}

fn sentinel_actions(resolved: &cimmeria_content_engine::chain::ResolvedActions) -> Vec<&Action> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == i64::from(TEST_CHAIN_ID))
        .map(|(_, a)| a)
        .collect()
}

/// Happy path: the matching tag crossing 50% resolves exactly the two
/// authored actions, in order — `set_npc_ai_state submit` then
/// `advance_step`.
#[tokio::test]
async fn crossing_the_threshold_resolves_submit_then_advance_step() {
    let pool = require_db_or_skip!();
    let engine = engine_with_sentinel_chain(&pool).await;

    let (event, ctx) = crossing_event(TEST_TAG, 60.0, 40.0);
    let resolved = engine.resolve_event(&event, &ctx);
    let actions = sentinel_actions(&resolved);

    assert_eq!(
        actions.len(),
        2,
        "a 60% → 40% crossing on the seeded tag must resolve both authored \
         actions; got {actions:?}",
    );
    match actions[0] {
        Action::SetNpcAiState { entity_tag, state } => {
            assert_eq!(entity_tag, TEST_TAG);
            assert!(
                matches!(state, NpcAiStateAction::Submit),
                "the ritual end is `submit`, got {state:?}",
            );
        }
        other => panic!("expected SetNpcAiState first, got {other:?}"),
    }
    match actions[1] {
        Action::AdvanceStep {
            mission_id,
            step_id,
        } => {
            assert_eq!(*mission_id, TEST_MISSION);
            assert_eq!(*step_id, TEST_STEP);
        }
        other => panic!("expected AdvanceStep second, got {other:?}"),
    }
}

/// A crossing on a different NPC must not resolve this chain — the tag
/// half of the `event_key` is load-bearing, and a duel chain firing on an
/// unrelated mob's wound would advance the mission from across the zone.
#[tokio::test]
async fn a_crossing_on_a_different_tag_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with_sentinel_chain(&pool).await;

    let (event, ctx) = crossing_event("CIMMERIA_TEST_H04_OTHER_NPC", 60.0, 40.0);
    let resolved = engine.resolve_event(&event, &ctx);

    assert!(
        sentinel_actions(&resolved).is_empty(),
        "the chain is keyed on its own tag; a crossing elsewhere must \
         resolve nothing",
    );
}

/// A hit on the right NPC that does not reach the authored percentage
/// resolves nothing. Pins the `pct_after <= pct` half of the predicate:
/// drop it and every wounding hit fires the chain.
#[tokio::test]
async fn a_hit_that_does_not_reach_the_threshold_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with_sentinel_chain(&pool).await;

    let (event, ctx) = crossing_event(TEST_TAG, 90.0, 55.0);
    let resolved = engine.resolve_event(&event, &ctx);

    assert!(
        sentinel_actions(&resolved).is_empty(),
        "90% → 55% never reaches the seeded 50% threshold",
    );
}

/// A follow-up hit while the NPC is already below resolves nothing.
/// Pins the `pct_before > pct` half: drop it and the duel re-advances
/// its step on every subsequent shot.
#[tokio::test]
async fn a_follow_up_hit_below_the_threshold_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with_sentinel_chain(&pool).await;

    let (event, ctx) = crossing_event(TEST_TAG, 40.0, 25.0);
    let resolved = engine.resolve_event(&event, &ctx);

    assert!(
        sentinel_actions(&resolved).is_empty(),
        "the threshold was already crossed on an earlier hit; this one \
         must resolve nothing",
    );
}

/// A malformed `event_key` must take the whole chain out of the engine,
/// through the **real loader**, not just through `convert_trigger` in
/// isolation.
///
/// `:0` is the interesting shape: it parses as an integer, so only the
/// range check rejects it, and a chain seeded that way is otherwise
/// well-formed. `build_chains_from_rows` sees every trigger row fail to
/// convert, warns "All trigger rows failed to convert — skipping chain",
/// and drops the chain — so `load_single_chain_for_test` returns `None`
/// even though `content_chains` has a row. Without that path, a content
/// author would get a chain that looks wired in the DB and never fires.
///
/// This is the one end-to-end case the unit tests on `convert_trigger`
/// cannot reach: they prove the arm returns `None`, not that the loader
/// then discards the chain rather than registering it triggerless.
#[tokio::test]
async fn a_chain_with_an_unusable_percentage_is_dropped_at_load() {
    let pool = require_db_or_skip!();

    cleanup_sentinel_chain(&pool).await;
    seed_sentinel_chain_with_key(&pool, &format!("{TEST_TAG}:0")).await;
    let loaded = load_single_chain_for_test(&pool, TEST_CHAIN_ID).await;
    cleanup_sentinel_chain(&pool).await;

    let chain = loaded.expect("DB query for the sentinel chain must succeed");
    assert!(
        chain.is_none(),
        "a `:0` threshold can never fire (a 0% entity is dead and routes \
         to entity_dead_tag), so the loader must drop the chain rather \
         than register a dead one; got {:?}",
        chain.map(|c| c.id),
    );
}

/// The author-supplied one-shot guard: once the step has advanced, a
/// second crossing (a relog that respawns the duel NPC at full health,
/// then a re-fight) resolves nothing.
///
/// This is an **authoring-contract fixture, not an H04 regression
/// guard**: `step_status` evaluation is pre-existing engine code that no
/// change in this packet can break, and four other replay modules
/// already cover it. It earns its place because the stateless crossing
/// design has no one-shot of its own (`content_triggers.once` is dead),
/// so every chain on this trigger *must* carry a gate — and H21's
/// mission 1325 is the first one that will. Reverting the condition row
/// out of `seed_sentinel_chain_with_key` fails it, which is the point:
/// it pins the seed shape H21 copies.
#[tokio::test]
async fn a_recrossing_after_the_step_advanced_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with_sentinel_chain(&pool).await;

    let (_, mut ctx) = crossing_event(TEST_TAG, 60.0, 40.0);
    ctx.set_param(
        format!("mission_{TEST_MISSION}_step_{TEST_STEP}_status"),
        serde_json::json!("completed"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityHealthBelow,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);

    assert!(
        sentinel_actions(&resolved).is_empty(),
        "the step_status gate must block a re-crossing after the step \
         advanced — without it, a relog-and-refight re-runs the beat",
    );
}
