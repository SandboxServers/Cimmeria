//! Mission 641 (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
//! `interact_tag('Preparation_ColMarsh')` shows briefing dialog 4001 to
//! non-sci players, gated by `mission_status(641) = 'not_active'`.
//!
//! The regression guard: the original seed gated on
//! `step_status(641, 2121) = 'not_active'`, which is also true *after*
//! the player advances past step 2121 (e.g. picks up the P90). That made
//! chain 1051 re-fire the briefing after pickup, chain 1053 re-accept
//! the mission on dialog choice, and the player loop back to step 2121.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Test 1: with mission 641 not yet accepted, the chain matches
/// (briefing is appropriate).
#[tokio::test]
async fn chain_1051_fires_when_mission_641_not_accepted() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1051)
        .await
        .expect("DB query for chain 1051 must succeed")
        .expect("chain 1051 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Preparation_ColMarsh"),
    );
    ctx.set_param(
        "mission_641_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(5)); // non-sci

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    use cimmeria_content_engine::actions::Action;
    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1051_actions: Vec<_> = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1051)
        .collect();
    // Chain 1051 resolves to exactly one action (`display_dialog 4001`) per
    // the seed. Pinning `== 1` rather than `> 0` AND pinning the action
    // variant so a future seed change that swaps the action while keeping
    // the count at 1 (e.g. accidentally turning the briefing dialog into a
    // mission accept) also fails this guard.
    assert_eq!(
        chain_1051_actions.len(),
        1,
        "chain 1051 must resolve exactly one action when mission 641 \
         hasn't been accepted; got {chain_1051_actions:?}",
    );
    assert!(
        matches!(
            chain_1051_actions[0].1,
            Action::DisplayDialog { dialog_id: 4001 }
        ),
        "chain 1051's action must be `DisplayDialog {{ dialog_id: 4001 }}`; \
         got {:?}",
        chain_1051_actions[0].1,
    );
}

/// Test 2: once mission 641 is active, chain 1051 must NOT fire — that
/// prevents the briefing dialog from re-showing and triggering the
/// re-accept loop. This is the bug-shape regression guard.
#[tokio::test]
async fn chain_1051_does_not_fire_when_mission_641_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1051)
        .await
        .expect("DB query for chain 1051 must succeed")
        .expect("chain 1051 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Preparation_ColMarsh"),
    );
    ctx.set_param(
        "mission_641_status".to_string(),
        serde_json::json!("active"),
    );
    // Step 2121 has been advanced past — the OLD condition on
    // `step_status(641, 2121) = 'not_active'` would still match this
    // shape. Including it here proves the new condition genuinely uses
    // mission_status, not step_status.
    ctx.set_param(
        "mission_641_step_2121_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(5));

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1051_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1051)
        .count();
    assert_eq!(
        chain_1051_actions, 0,
        "chain 1051 must NOT fire while mission 641 is active — the old \
         step_status(641, 2121) condition would have re-fired the \
         briefing after pickup and looped the quest. Got \
         {chain_1051_actions} actions.",
    );
}
