//! `stargate_dialed` / `stargate_crossed` chain-replay guards (CA10).
//!
//! Like [`super::grant_xp`], these seed their own chains: there are zero
//! `stargate_*` trigger rows in `db/resources/Content/Seed/` today, so a
//! sentinel chain is the only way to pin the loader arm before content
//! (mission 708, CA09) starts using it.
//!
//! What fails when the change is reverted:
//!
//! - Removing the `"stargate_dialed"` / `"stargate_crossed"` arms from
//!   `crates/content-engine/src/loader/trigger.rs` makes `convert_trigger`
//!   return `None`, the chain is rejected outright, and
//!   `load_single_chain_for_test` yields `None` — the `.expect` on the
//!   loaded chain fails.
//! - Removing the `Trigger::OnStargateDialed`/`OnStargateCrossed` arm
//!   from `triggers/matching.rs` breaks either the discriminant (the
//!   chain lands in the wrong index bucket and resolves zero actions) or
//!   the `destination_world` comparison (the wrong-world negative fires).
//!
//! Sentinel id range: `0x7000_6000` / `0x7000_6010`. Steps past every
//! reservation in `crates/services` (`…_0000`–`…_5000`). Cleanup deletes
//! the exact ids inserted, never a range.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

const DIALED_CHAIN_ID: i32 = 0x7000_6000;
const CROSSED_CHAIN_ID: i32 = 0x7000_6010;

/// The destination world the sentinel chains key on. A real
/// `resources.worlds.world` value so the fixture matches what
/// `fire_stargate_dialed` puts in `destination_world` at runtime.
const KEYED_WORLD: &str = "Harset";
/// A different real world — the negative case.
const OTHER_WORLD: &str = "Castle";

async fn seed_chain(pool: &PgPool, chain_id: i32, event_type: &str) {
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'stargate trigger chain-replay sentinel', 'space', NULL, true, 0)",
    )
    .bind(chain_id)
    .execute(pool)
    .await
    .expect("sentinel content_chains insert must succeed");

    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, $2, $3, 'player', false, 0)",
    )
    .bind(chain_id)
    .bind(event_type)
    .bind(KEYED_WORLD)
    .execute(pool)
    .await
    .expect("sentinel content_triggers insert must succeed");

    // `increment_counter` reads the counter name from `target_key`, not
    // from `params` — see `loader/action.rs`'s arm. A NULL `target_key`
    // makes `convert_action` return `None` and the row is dropped.
    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'increment_counter', NULL, $2, '{\"amount\": 1}'::jsonb, 0, 0)",
    )
    .bind(chain_id)
    .bind(format!("cimmeria_test_{event_type}"))
    .execute(pool)
    .await
    .expect("sentinel content_actions insert must succeed");
}

async fn cleanup_chain(pool: &PgPool, chain_id: i32) {
    for stmt in [
        "DELETE FROM resources.content_actions WHERE chain_id = $1",
        "DELETE FROM resources.content_triggers WHERE chain_id = $1",
        "DELETE FROM resources.content_chains WHERE chain_id = $1",
    ] {
        sqlx::query(stmt)
            .bind(chain_id)
            .execute(pool)
            .await
            .expect("sentinel cleanup must succeed");
    }
}

fn event(trigger_type: TriggerType, destination_world: &str) -> (TriggerEvent, ExecutionContext) {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "destination_world".to_string(),
        serde_json::json!(destination_world),
    );
    let ev = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    (ev, ctx)
}

/// Load a seeded chain and assert it matches its own world and rejects
/// another. Shared body — the two triggers differ only in `event_type`
/// and the discriminant they must land under.
async fn assert_world_keyed_trigger(
    pool: &PgPool,
    chain_id: i32,
    event_type: &str,
    trigger_type: TriggerType,
    other_trigger_type: TriggerType,
) {
    cleanup_chain(pool, chain_id).await;
    seed_chain(pool, chain_id, event_type).await;

    let loaded = load_single_chain_for_test(pool, chain_id).await;
    cleanup_chain(pool, chain_id).await;

    let chain = loaded
        .expect("DB query for the sentinel chain must succeed")
        .expect(
            "sentinel chain must load — a None here means convert_trigger \
             has no arm for this event_type and rejected the row",
        );
    assert_eq!(
        chain.actions.len(),
        1,
        "the sentinel's single action row must survive the loader"
    );

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let (ev, ctx) = event(trigger_type.clone(), KEYED_WORLD);
    assert_eq!(
        engine.resolve_event(&ev, &ctx).actions.len(),
        1,
        "{event_type} keyed on {KEYED_WORLD} must match an event for \
         {KEYED_WORLD}"
    );

    let (ev, ctx) = event(trigger_type, OTHER_WORLD);
    assert!(
        engine.resolve_event(&ev, &ctx).actions.is_empty(),
        "{event_type} keyed on {KEYED_WORLD} must NOT match a gate to \
         {OTHER_WORLD} — a collapsed wildcard would fire on every gate"
    );

    // Cross-trigger negative: dialling is not crossing. Without distinct
    // `TriggerType` discriminants both chains would answer both events,
    // and mission 708 would complete its "step through" objective the
    // moment the player touched the DHD.
    let (ev, ctx) = event(other_trigger_type, KEYED_WORLD);
    assert!(
        engine.resolve_event(&ev, &ctx).actions.is_empty(),
        "a {event_type} chain must not answer the sibling stargate event"
    );
}

#[tokio::test]
async fn stargate_dialed_chain_matches_its_world_only() {
    let pool = require_db_or_skip!();
    assert_world_keyed_trigger(
        &pool,
        DIALED_CHAIN_ID,
        "stargate_dialed",
        TriggerType::StargateDialed,
        TriggerType::StargateCrossed,
    )
    .await;
}

#[tokio::test]
async fn stargate_crossed_chain_matches_its_world_only() {
    let pool = require_db_or_skip!();
    assert_world_keyed_trigger(
        &pool,
        CROSSED_CHAIN_ID,
        "stargate_crossed",
        TriggerType::StargateCrossed,
        TriggerType::StargateDialed,
    )
    .await;
}

/// A NULL `event_key` is the documented wildcard: fires for any gate
/// destination. Pin it so a future "reject NULL like the integer-keyed
/// triggers do" change can't silently disable wildcard chains.
#[tokio::test]
async fn stargate_dialed_with_null_event_key_is_a_wildcard() {
    let pool = require_db_or_skip!();
    let chain_id = DIALED_CHAIN_ID + 1;

    cleanup_chain(&pool, chain_id).await;
    sqlx::query(
        "INSERT INTO resources.content_chains \
         (chain_id, description, scope_type, scope_id, enabled, priority) \
         VALUES ($1, 'stargate wildcard sentinel', 'space', NULL, true, 0)",
    )
    .bind(chain_id)
    .execute(&pool)
    .await
    .expect("sentinel content_chains insert must succeed");
    sqlx::query(
        "INSERT INTO resources.content_triggers \
         (chain_id, event_type, event_key, scope, once, sort_order) \
         VALUES ($1, 'stargate_dialed', NULL, 'player', false, 0)",
    )
    .bind(chain_id)
    .execute(&pool)
    .await
    .expect("sentinel content_triggers insert must succeed");
    sqlx::query(
        "INSERT INTO resources.content_actions \
         (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) \
         VALUES ($1, 'increment_counter', NULL, 'cimmeria_test_sg_wildcard', \
                 '{\"amount\": 1}'::jsonb, 0, 0)",
    )
    .bind(chain_id)
    .execute(&pool)
    .await
    .expect("sentinel content_actions insert must succeed");

    let loaded = load_single_chain_for_test(&pool, chain_id).await;
    cleanup_chain(&pool, chain_id).await;

    let chain = loaded
        .expect("DB query must succeed")
        .expect("a NULL event_key must still produce a loadable chain");
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    for world in [KEYED_WORLD, OTHER_WORLD] {
        let (ev, ctx) = event(TriggerType::StargateDialed, world);
        assert_eq!(
            engine.resolve_event(&ev, &ctx).actions.len(),
            1,
            "wildcard stargate_dialed must fire for {world}"
        );
    }
}
