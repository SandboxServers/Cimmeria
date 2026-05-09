//! Mission 622 — Arm Yourself! Pin the equip-from-inventory flow:
//! chain 1003 grants the pistol to the backpack (container 1) and advances
//! mission 622 to step 80622 ("Equip the pistol"); chain 1004 fires on
//! `item_equipped(55)` and is the only path that plays kismet sequence
//! 10000 (which opens the stasis-room door) and completes the mission.
//!
//! The bug shape this guards: previously the pistol was added directly to
//! the bandolier (container 3) and the mission auto-completed, which
//! bypassed the bandolier ammo / appearance / fire-animation paths and
//! caused issues #211 and #212. Step 80622 is a Cimmeria-introduced step
//! whose matching `<Steps>` row is shipped to the client via a mission_
//! overrides patch on `_622` in `CookedDataMissions.pak`; the per-key
//! `InvalidKeys` channel in `onVersionInfo` makes that override visible
//! to the UI without re-shipping the whole PAK.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1003 must grant the pistol (item 55) to container 1 (backpack)
/// and advance to step 80622. It must NOT play sequence 10000 or
/// complete the mission — those happen in chain 1004 after equip.
#[tokio::test]
async fn chain_1003_grants_pistol_to_backpack_and_advances_to_equip_step() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1003)
        .await
        .expect("DB query for chain 1003 must succeed")
        .expect("chain 1003 must exist in seeded content_chains");

    // Pistol → container 1 (backpack), exactly once.
    let pistol_to_backpack = chain
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::GrantItem {
                    item_id: 55,
                    container_id: Some(1),
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        pistol_to_backpack, 1,
        "chain 1003 must grant pistol (item 55) to container 1 (backpack) \
         exactly once — auto-equipping to container 3 (bandolier) bypasses \
         the bandolier ammo path and was the cause of #211/#212. Actions: {:?}",
        chain.actions,
    );

    // Letter (3730) → container 0 (mission inventory), exactly once.
    let letter_to_mission = chain
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::GrantItem {
                    item_id: 3730,
                    container_id: Some(0),
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        letter_to_mission, 1,
        "chain 1003 must grant Frost's letter (3730) to container 0; got {} matches",
        letter_to_mission,
    );

    // Advance to the equip step (80622) — this is what makes the new
    // mission text render on the UI.
    let advances_to_equip_step = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::AdvanceStep {
                mission_id: 622,
                step_id: 80622
            }
        )
    });
    assert!(
        advances_to_equip_step,
        "chain 1003 must advance mission 622 to step 80622. Actions: {:?}",
        chain.actions,
    );

    // No play_sequence 10000 — that's chain 1004's job, gated on equip.
    let plays_door_sequence = chain
        .actions
        .iter()
        .any(|a| matches!(a, Action::PlaySequence { sequence_id: 10000 }));
    assert!(
        !plays_door_sequence,
        "chain 1003 must NOT play sequence 10000; it's deferred to chain 1004",
    );

    // Mission completion is deferred to chain 1004 too.
    let completes_mission = chain
        .actions
        .iter()
        .any(|a| matches!(a, Action::CompleteMission { mission_id: 622 }));
    assert!(
        !completes_mission,
        "chain 1003 must NOT complete mission 622; deferred to chain 1004",
    );
}

/// Chain 1003 must NOT fire when step 2113 has been advanced past — the
/// gate is what blocks duplicate-grant when the player re-opens Frost's
/// loot dialog. Regression guard for the bug we observed in the live UI
/// where a permissive `mission_status = active` gate let the chain fire
/// repeatedly across re-interactions.
#[tokio::test]
async fn chain_1003_does_not_fire_after_step_2113_advances() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1003)
        .await
        .expect("DB query for chain 1003 must succeed")
        .expect("chain 1003 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(3995));
    ctx.set_param(
        "mission_622_status".to_string(),
        serde_json::json!("active"),
    );
    // Step 2113 has already completed — chain 1003 advanced us past it on
    // a prior dialog open. A second dialog open must NOT match.
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_622_step_80622_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1003_fired = resolved.actions.iter().any(|(id, _)| *id == 1003);
    assert!(
        !chain_1003_fired,
        "chain 1003 must NOT fire after step 2113 has been advanced past — \
         re-opening Frost's loot dialog would otherwise grant duplicate items. \
         Got actions: {:?}",
        resolved.actions,
    );
}

/// Chain 1004 fires on `item_equipped(55)` while step 80622 is active.
/// Plays the door kismet sequence and completes the mission.
#[tokio::test]
async fn chain_1004_fires_on_pistol_equip_at_step_80622() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1004)
        .await
        .expect("DB query for chain 1004 must succeed")
        .expect("chain 1004 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("item_id".to_string(), serde_json::json!(55));
    ctx.set_param(
        "mission_622_step_80622_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemEquipped,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    let plays_sequence = resolved
        .actions
        .iter()
        .any(|(id, a)| *id == 1004 && matches!(a, Action::PlaySequence { sequence_id: 10000 }));
    assert!(
        plays_sequence,
        "chain 1004 must play sequence 10000 (stasis-room door) on equip; got {:?}",
        resolved.actions,
    );

    let completes = resolved
        .actions
        .iter()
        .any(|(id, a)| *id == 1004 && matches!(a, Action::CompleteMission { mission_id: 622 }));
    assert!(
        completes,
        "chain 1004 must complete mission 622 on equip; got {:?}",
        resolved.actions,
    );
}

/// Chain 1004 must NOT fire when step 80622 isn't active. Guards against
/// an early-equip path: if the player somehow obtains item 55 without
/// looting Frost (so step 80622 was never reached), equipping must not
/// short-circuit the door + mission completion.
#[tokio::test]
async fn chain_1004_does_not_fire_without_step_80622_active() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1004)
        .await
        .expect("DB query for chain 1004 must succeed")
        .expect("chain 1004 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("item_id".to_string(), serde_json::json!(55));
    // step 80622 not active — still on step 2113 (player hasn't looted yet).
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemEquipped,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1004_fired = resolved.actions.iter().any(|(id, _)| *id == 1004);
    assert!(
        !chain_1004_fired,
        "chain 1004 must NOT fire before step 80622 is reached — \
         a pre-loot equip would otherwise complete the mission without \
         visiting Frost's body. Got actions: {:?}",
        resolved.actions,
    );
}
