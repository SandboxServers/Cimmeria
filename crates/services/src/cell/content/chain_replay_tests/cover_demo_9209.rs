//! Chain 9209 (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
//! the cover-system demo chain — fires on `OnPlayerEnteredCover` (any
//! cover_set_id) gated on mission 639 step 2145 active, and increments
//! the `cover_demo_entered` counter on the player entity.
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

/// Test 1: with mission 639 step 2145 active, entering ANY cover set
/// resolves the chain to its `IncrementCounter("cover_demo_entered", 1)`
/// action. Pinning the exact action variant + value catches drift if
/// the demo seed gets reworked into a different observable side effect.
#[tokio::test]
async fn chain_9209_fires_when_step_2145_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 9209)
        .await
        .expect("DB query for chain 9209 must succeed")
        .expect("chain 9209 must exist and load — the cover-system demo");

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
        .filter(|(id, _)| *id == 9209)
        .collect();
    assert_eq!(
        chain_actions.len(),
        1,
        "chain 9209 must resolve exactly one action when step 2145 is active; \
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
        "chain 9209's single action must be IncrementCounter(cover_demo_entered, 1); \
         got {:?}",
        chain_actions[0].1
    );
}

/// Test 2: when step 2145 is NOT active (e.g. before the player picks
/// up the Ambernol vial), the chain must NOT fire. Catches a regression
/// where a future loader change collapses the step-status condition
/// into a wildcard.
#[tokio::test]
async fn chain_9209_does_not_fire_when_step_2145_inactive() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 9209)
        .await
        .expect("DB query for chain 9209 must succeed")
        .expect("chain 9209 must exist and load");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(42));
    // Mission 639 step 2145 not yet active — earlier in the quest flow.
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
        .filter(|(id, _)| *id == 9209)
        .collect();
    assert!(
        chain_actions.is_empty(),
        "chain 9209 must NOT fire when step 2145 is inactive; \
         got {chain_actions:?}",
    );
}
