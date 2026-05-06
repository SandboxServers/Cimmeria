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

    let remove_vial_actions = chain
        .actions
        .iter()
        .filter(
            |a| matches!(a, Action::RemoveItem { item_id, count } if *item_id == 19 && *count == 1),
        )
        .count();
    // Pin `== 1` rather than `>= 1` so a future seed change that
    // accidentally duplicates the remove action (causing a stack-of-2
    // vials to vanish in one use) fails this guard.
    assert_eq!(
        remove_vial_actions, 1,
        "chain 1034 must include exactly one `RemoveItem {{ item_id: 19, count: 1 }}` \
         so the ambernol vial is consumed on use; got {remove_vial_actions} \
         matching actions. Full action list: {:?}",
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

// ────────────────────────────────────────────────────────────────────
// Mission 638 (Speak to Prisoner 329) — archetype-gated dialog routing
// ────────────────────────────────────────────────────────────────────
//
// Chains 1011 (non-Jaffa) and 1012 (Jaffa) had their `add_dialog_set`
// IDs swapped relative to the archetype condition (issue #216), so a
// Tau'ri / Human player saw the Jaffa "My symbiote will cure me"
// dialog and vice versa.
//
// The shared helper [`assert_region_enter_resolves_dialog_set`] is the
// reusable shape for any future archetype-gated dialog regression
// guard: pass a chain id, an archetype value, and the dialog_set_map id
// you expect the chain to add. Every other archetype-routed dialog
// pair in the seed (1056/1057, future siblings) can be regression-
// guarded by adding two calls.

/// Load `chain_id`, fire a `RegionEnter('Castle_Cellblock.Region2')`
/// event with the supplied `archetype` (and `mission_638_status =
/// 'not_active'` so the mission-gate condition passes), and assert
/// that the resolved actions contain exactly one `AddDialogSet` whose
/// `dialog_set_id` matches `expected_dialog_set_map`.
///
/// `expected_dialog_set_map` is the `dialog_set_maps.dialog_set_map_id`
/// (i.e., the `target_id` column on the `add_dialog_set` action row),
/// not the `dialog_set_id` it groups under. The Rust field is named
/// `dialog_set_id` for historical reasons; the value stored is the
/// per-archetype map row.
async fn assert_region_enter_resolves_dialog_set(
    chain_id: i32,
    archetype: i32,
    expected_dialog_set_map: i32,
) {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let actual: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(_, action)| match action {
            Action::AddDialogSet { dialog_set_id, .. } => Some(*dialog_set_id),
            _ => None,
        })
        .collect();

    assert_eq!(
        actual,
        vec![expected_dialog_set_map],
        "chain {chain_id} with archetype={archetype} must add exactly \
         dialog_set_map {expected_dialog_set_map}; got {actual:?}"
    );
}

/// Load `chain_id` and fire `RegionEnter('Castle_Cellblock.Region2')`
/// with the supplied archetype. Assert that NO `AddDialogSet` actions
/// resolve — used as the negative pin (the wrong archetype must not
/// match the chain's gate).
async fn assert_region_enter_does_not_resolve(chain_id: i32, archetype: i32) {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "region_key".to_string(),
        serde_json::json!("Castle_Cellblock.Region2"),
    );
    ctx.set_param(
        "mission_638_status".to_string(),
        serde_json::json!("not_active"),
    );
    ctx.set_param("archetype".to_string(), serde_json::json!(archetype));

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let dialog_set_actions: Vec<i32> = resolved
        .actions
        .iter()
        .filter_map(|(_, action)| match action {
            Action::AddDialogSet { dialog_set_id, .. } => Some(*dialog_set_id),
            _ => None,
        })
        .collect();

    assert!(
        dialog_set_actions.is_empty(),
        "chain {chain_id} must NOT match for archetype={archetype} — \
         the archetype condition is what gates branch selection. \
         Got AddDialogSet ids: {dialog_set_actions:?}"
    );
}

/// Tau'ri / Human player (archetype Soldier=1) entering Region2 must get
/// the Human "Free Prisoner 329" dialog set (dialog_set_map 2794 →
/// dialog 2300), not the Jaffa one. Pinned shape: chain 1011's
/// archetype-neq-8 condition routes here.
#[tokio::test]
async fn chain_1011_routes_non_jaffa_to_human_dialog_set() {
    const ARCHETYPE_SOLDIER: i32 = 1;
    const HUMAN_FREE_PRISONER_DIALOG_SET_MAP: i32 = 2794;
    assert_region_enter_resolves_dialog_set(
        1011,
        ARCHETYPE_SOLDIER,
        HUMAN_FREE_PRISONER_DIALOG_SET_MAP,
    )
    .await;
}

/// Jaffa player (archetype 8) entering Region2 must get the Jaffa
/// "Free Prisoner 329" dialog set (dialog_set_map 5866 → dialog 5021,
/// the symbiote dialog), not the Human one. Pinned shape: chain 1012's
/// archetype-eq-8 condition routes here.
#[tokio::test]
async fn chain_1012_routes_jaffa_to_jaffa_dialog_set() {
    const ARCHETYPE_JAFFA: i32 = 8;
    const JAFFA_FREE_PRISONER_DIALOG_SET_MAP: i32 = 5866;
    assert_region_enter_resolves_dialog_set(
        1012,
        ARCHETYPE_JAFFA,
        JAFFA_FREE_PRISONER_DIALOG_SET_MAP,
    )
    .await;
}

/// Negative pin: chain 1011 (non-Jaffa branch) must NOT match when the
/// player is Jaffa. Without the archetype condition both chains would
/// fire and the prisoner would offer both dialog sets at once.
#[tokio::test]
async fn chain_1011_does_not_match_jaffa_archetype() {
    const ARCHETYPE_JAFFA: i32 = 8;
    assert_region_enter_does_not_resolve(1011, ARCHETYPE_JAFFA).await;
}

/// Negative pin: chain 1012 (Jaffa branch) must NOT match when the
/// player is non-Jaffa. Mirror of the above, on the other side of the
/// archetype gate.
#[tokio::test]
async fn chain_1012_does_not_match_non_jaffa_archetype() {
    const ARCHETYPE_SOLDIER: i32 = 1;
    assert_region_enter_does_not_resolve(1012, ARCHETYPE_SOLDIER).await;
}
