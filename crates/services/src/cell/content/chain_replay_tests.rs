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

/// Chain 1034 (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
/// `item_use 19` consumes the ambernol vial, completes mission 639, and
/// accepts mission 640. The `remove_item` action is the load-bearing piece
/// — without it, the player keeps the vial after using it (and any chain
/// gated on "no longer holds vial" stays stuck).
///
/// Regression guard for an actual production bug: the seed file was
/// updated to add the `remove_item` action, but a stale local DB without
/// a re-seed surfaced as "ambernol use no longer removes the vial". This
/// test would have failed in CI on the broken seed.
#[tokio::test]
async fn chain_1034_includes_remove_item_for_ambernol() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1034)
        .await
        .expect("DB query for chain 1034 must succeed")
        .expect("chain 1034 must exist in seeded content_chains");

    let removes_vial = chain.actions.iter().any(
        |a| matches!(a, Action::RemoveItem { item_id, count } if *item_id == 19 && *count == 1),
    );
    assert!(
        removes_vial,
        "chain 1034 must include `RemoveItem {{ item_id: 19, count: 1 }}` so the \
         ambernol vial is consumed on use; loaded {} actions: {:?}",
        chain.actions.len(),
        chain.actions,
    );
}

/// Chain 1051 (`db/resources/Content/Seed/castle_cellblock_chains.sql`):
/// `interact_tag('Preparation_ColMarsh')` shows briefing dialog 4001 to
/// non-sci players, gated by `mission_status(641) = 'not_active'`.
///
/// The regression guard: the original seed gated on
/// `step_status(641, 2121) = 'not_active'`, which is also true *after*
/// the player advances past step 2121 (e.g. picks up the P90). That made
/// chain 1051 re-fire the briefing after pickup, chain 1053 re-accept
/// the mission on dialog choice, and the player loop back to step 2121.
///
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

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1051_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1051)
        .count();
    // Chain 1051 resolves to exactly one action (`display_dialog 4001`) per
    // the seed. Pinning `== 1` rather than `> 0` so a future seed change
    // that accidentally adds an extra action — say, a stray `set_interaction_type`
    // that wasn't intended for the briefing path — fails this guard.
    assert_eq!(
        chain_1051_actions, 1,
        "chain 1051 must resolve exactly one action (display_dialog 4001) \
         when mission 641 hasn't been accepted; got {chain_1051_actions}",
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
