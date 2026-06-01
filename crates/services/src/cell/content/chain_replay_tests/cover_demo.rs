//! Cover-system demo chain (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
//! fires on `OnPlayerEnteredCover` (any cover_set_id) gated on mission
//! 639 step 2145 active, and increments the `cover_demo_entered`
//! counter on the player entity.
//!
//! Load-bearing for: proves the DB → loader → trigger match → executor
//! pipeline works for the new `OnPlayerEnteredCover` trigger family.
//! Without this guard a future loader/dispatcher refactor could
//! silently break the entire cover→content chain authoring path.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Single source of truth for the demo chain id. Keep in sync with the
/// `1035` literal in `db/resources/Content/Seed/castle_cellblock_chains.sql`
/// (mission 639 range 1031–1040). Typed as `i64` to match `Chain.id`; the
/// loader takes `i32`, so the call site casts at that boundary.
const COVER_DEMO_CHAIN_ID: i64 = 1035;

#[tokio::test]
async fn cover_demo_fires_when_step_2145_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, COVER_DEMO_CHAIN_ID as i32)
        .await
        .expect("DB query for cover-demo chain must succeed")
        .expect("cover-demo chain must exist and load");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(42));
    ctx.set_param(
        "mission_639_step_2145_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerEnteredCover,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == COVER_DEMO_CHAIN_ID)
        .collect();
    assert_eq!(
        chain_actions.len(),
        1,
        "cover-demo chain must resolve exactly one action when step 2145 is active; \
         got {chain_actions:?}",
    );
    assert!(
        matches!(
            chain_actions[0].1,
            Action::IncrementCounter {
                ref counter_name,
                amount: 1,
            } if counter_name == "cover_demo_entered"
        ),
        "cover-demo chain's single action must be IncrementCounter(cover_demo_entered, 1); \
         got {:?}",
        chain_actions[0].1
    );
}

#[tokio::test]
async fn cover_demo_does_not_fire_when_step_2145_inactive() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, COVER_DEMO_CHAIN_ID as i32)
        .await
        .expect("DB query for cover-demo chain must succeed")
        .expect("cover-demo chain must exist and load");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(42));
    ctx.set_param(
        "mission_639_step_2145_status".to_string(),
        serde_json::json!("not_active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerEnteredCover,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_actions: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == COVER_DEMO_CHAIN_ID)
        .collect();
    assert!(
        chain_actions.is_empty(),
        "cover-demo chain must NOT fire when step 2145 is inactive; \
         got {chain_actions:?}",
    );
}
