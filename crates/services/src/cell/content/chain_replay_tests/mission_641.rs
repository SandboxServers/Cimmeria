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

/// Chain 1055 (locker pickup) must grant the P90 to the backpack
/// (container 1), advance to step 80641 ("Equip the P90"), and clear the
/// locker highlight. Must NOT set the Marsh "talk to me" marker yet —
/// that's deferred to chain 1066 after equip.
///
/// Bug shape: auto-equipping to container 3 bypassed the bandolier ammo
/// path (issues #211/#212). Step 80641 is a Cimmeria-introduced step
/// shipped to the client via mission_overrides patch on `_641` in
/// `CookedDataMissions.pak` and the per-key `InvalidKeys` channel.
#[tokio::test]
async fn chain_1055_grants_p90_to_backpack_and_advances_to_equip_step() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1055)
        .await
        .expect("DB query for chain 1055 must succeed")
        .expect("chain 1055 must exist in seeded content_chains");

    let p90_to_backpack = chain
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::GrantItem {
                    item_id: 21,
                    container_id: Some(1),
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        p90_to_backpack, 1,
        "chain 1055 must grant P90 (item 21) to container 1 (backpack) \
         exactly once. Actions: {:?}",
        chain.actions,
    );

    // Advance to the equip step (80641) — this is what makes the new
    // "Equip the P90" mission text render on the UI.
    let advances_to_equip_step = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::AdvanceStep {
                mission_id: 641,
                step_id: 80641
            }
        )
    });
    assert!(
        advances_to_equip_step,
        "chain 1055 must advance mission 641 to step 80641. Actions: {:?}",
        chain.actions,
    );

    // No SetInteractionType on Preparation_ColMarsh — Marsh stays
    // non-talkable until equip (chain 1066 sets the marker then).
    let marsh_marker_set = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::SetInteractionType { entity_tag, operation, .. }
            if entity_tag == "Preparation_ColMarsh" && operation == "|"
        )
    });
    assert!(
        !marsh_marker_set,
        "chain 1055 must NOT set the Marsh talk-to-me marker — deferred to \
         chain 1066 on item_equipped. Actions: {:?}",
        chain.actions,
    );
}

/// Chain 1055 must NOT fire a second time once step 2121 has been
/// advanced past — `step_status 641 2121 = active` is what scopes the
/// chain to the first interact. Regression guard for duplicate-grant on
/// re-interact.
#[tokio::test]
async fn chain_1055_does_not_re_fire_after_step_2121_advances() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1055)
        .await
        .expect("DB query for chain 1055 must succeed")
        .expect("chain 1055 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Preparation_SMG1A"),
    );
    ctx.set_param(
        "mission_641_step_2121_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_641_step_80641_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1055_fired = resolved.actions.iter().any(|(id, _)| *id == 1055);
    assert!(
        !chain_1055_fired,
        "chain 1055 must NOT fire after step 2121 has been advanced past — \
         re-interacting with the locker would otherwise grant a second P90. \
         Got actions: {:?}",
        resolved.actions,
    );
}

/// Chain 1066 fires on `item_equipped(21)` while step 80641 is active,
/// advances to step 3563 ("Speak to Col. Marsh"), and sets Marsh's
/// mission-available marker.
#[tokio::test]
async fn chain_1066_advances_to_marsh_step_and_sets_marker_on_equip() {
    use cimmeria_content_engine::actions::Action;

    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1066)
        .await
        .expect("DB query for chain 1066 must succeed")
        .expect("chain 1066 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("item_id".to_string(), serde_json::json!(21));
    ctx.set_param(
        "mission_641_step_80641_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemEquipped,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let advances_to_marsh_step = resolved.actions.iter().any(|(id, a)| {
        *id == 1066
            && matches!(
                a,
                Action::AdvanceStep {
                    mission_id: 641,
                    step_id: 3563
                }
            )
    });
    assert!(
        advances_to_marsh_step,
        "chain 1066 must advance mission 641 to step 3563 on equip; got {:?}",
        resolved.actions,
    );

    let sets_marsh_marker = resolved.actions.iter().any(|(id, a)| {
        *id == 1066
            && matches!(
                a,
                Action::SetInteractionType { entity_tag, operation, .. }
                if entity_tag == "Preparation_ColMarsh" && operation == "|"
            )
    });
    assert!(
        sets_marsh_marker,
        "chain 1066 must set Preparation_ColMarsh marker on equip; got {:?}",
        resolved.actions,
    );
}

/// Chain 1066 must NOT fire when step 80641 isn't active — guards against
/// an early-equip path skipping the locker pickup.
#[tokio::test]
async fn chain_1066_does_not_fire_without_step_80641_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1066)
        .await
        .expect("DB query for chain 1066 must succeed")
        .expect("chain 1066 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("item_id".to_string(), serde_json::json!(21));
    // Player is still on step 2121 (locker not yet looted) — chain 1066
    // requires step 80641 active and must not fire.
    ctx.set_param(
        "mission_641_step_2121_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemEquipped,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1066_fired = resolved.actions.iter().any(|(id, _)| *id == 1066);
    assert!(
        !chain_1066_fired,
        "chain 1066 must NOT fire before step 80641 is reached. \
         Got actions: {:?}",
        resolved.actions,
    );
}
