//! Live-DB chain-replay regression guards.
//!
//! Each test loads a specific seeded `content_*` chain from the database
//! through the same `build_chains_from_rows` pipeline that the cell service
//! uses at startup, registers it in a fresh `ChainEngine`, and fires
//! synthetic events through `resolve_event` to assert the chain matches
//! (or doesn't) under specific `ExecutionContext` shapes.
//!
//! Loading from DB rather than hand-constructing the chain in Rust is
//! deliberate: the whole point is to catch silent drift in the SQL seed
//! (e.g. someone removes a `mission_status` condition that was added to fix
//! a bug). A pure-Rust replica would let that drift pass.
//!
//! Skip cleanly when DATABASE_URL is unset.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 3026 (`db/resources/Content/Seed/sgc_w1_chains.sql`):
/// `dialog_choice('5365')` accepts mission 1562, gated by
/// `mission_status(1562) = 'not_active'`. That condition is the
/// regression guard: without it, every dialog re-display re-accepts
/// the mission and resets progress.
///
/// Test 1: with `mission_1562_status = 'not_active'`, the chain matches
/// and the engine resolves its actions. Pins the happy-path acceptance.
#[tokio::test]
async fn chain_3026_fires_dialog_5365_when_mission_1562_is_not_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3026)
        .await
        .expect("DB query for chain 3026 must succeed")
        .expect("chain 3026 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    // The dialog_id key matches the trigger's filter. The mission
    // status param is what the chain's MissionStatus condition reads
    // (key shape is `mission_{id}_status` per
    // `crates/content-engine/src/conditions.rs`).
    ctx.set_param("dialog_id".to_string(), serde_json::json!(5365));
    ctx.set_param(
        "mission_1562_status".to_string(),
        serde_json::json!("not_active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_3026_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 3026)
        .count();
    assert!(
        chain_3026_actions > 0,
        "chain 3026 must resolve at least one action when \
         mission_1562_status = 'not_active'; resolver returned {} actions \
         total ({chain_3026_actions} from chain 3026)",
        resolved.actions.len(),
    );
}

/// Test 2: with `mission_1562_status = 'active'`, the chain's
/// `mission_status` condition fails and the engine does NOT resolve any
/// actions for chain 3026. This is the regression guard proper: if the
/// condition is removed from the seed, this test fails.
///
/// We don't assert that the resolver returns *zero* actions overall —
/// other chains may match `dialog_id=5365`, and the goal is specifically
/// to pin chain 3026's behavior, not the whole content pipeline.
#[tokio::test]
async fn chain_3026_does_not_fire_when_mission_1562_is_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3026)
        .await
        .expect("DB query for chain 3026 must succeed")
        .expect("chain 3026 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(5365));
    ctx.set_param(
        "mission_1562_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_3026_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 3026)
        .count();
    assert_eq!(
        chain_3026_actions, 0,
        "chain 3026 must NOT resolve any actions when \
         mission_1562_status = 'active' (the not_active condition is \
         the regression guard against re-accepting on dialog re-display); \
         got {chain_3026_actions} actions from chain 3026",
    );
}

/// Test 3: `mission_1562_status = 'completed'` also blocks chain 3026.
/// Same shape as the active-state test but pins the third leaf of the
/// MissionStatusValue enum so a future "operator changed from eq to !="
/// regression on the seed surfaces here.
#[tokio::test]
async fn chain_3026_does_not_fire_when_mission_1562_is_completed() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 3026)
        .await
        .expect("DB query for chain 3026 must succeed")
        .expect("chain 3026 must exist in seeded content_chains and successfully load/convert");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(5365));
    ctx.set_param(
        "mission_1562_status".to_string(),
        serde_json::json!("completed"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_3026_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 3026)
        .count();
    assert_eq!(
        chain_3026_actions, 0,
        "chain 3026 must NOT resolve any actions when \
         mission_1562_status = 'completed'; got {chain_3026_actions}",
    );
}
