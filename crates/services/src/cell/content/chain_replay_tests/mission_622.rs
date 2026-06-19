//! Mission 622 — Arm Yourself! Pin the **loot-split** + equip-from-inventory
//! flow:
//! - chain 1003 (Frost body, dialog 3995) grants ONLY the letter (3730) to the
//!   mission inventory; it does not grant the pistol and does not advance.
//! - chain 1005 (Guard corpse, dialog 3996) grants the pistol (55) to the
//!   backpack (container 1) and owns the 2113 → 80622 advance.
//! - chain 1004 fires on `item_equipped(55)` and is the only path that plays
//!   kismet sequence 10000 (opens the stasis-room door) + completes the mission.
//!
//! The loot is split across the two stasis-room corpses; the order the player
//! searches them in must not matter (see the order-independence tests below).
//! Frost's letter chain is gated on `mission_status = active` (not step 2113)
//! so it stays reachable after the Guard advances the step; its re-loot guard
//! is the body's interaction-bit clear, since `once` is a no-op in the engine.
//!
//! Step 80622 is a Cimmeria-introduced step whose matching `<Steps>` row is
//! shipped to the client via a `mission_overrides` patch on `_622` in
//! `CookedDataMissions.pak`; unchanged by the loot split.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1003 (Frost) must grant ONLY the letter — no pistol, no advance, no
/// completion. The pistol + advance moved to chain 1005 (the Guard corpse).
#[tokio::test]
async fn chain_1003_grants_letter_only_no_pistol_no_advance() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1003)
        .await
        .expect("DB query for chain 1003 must succeed")
        .expect("chain 1003 must exist in seeded content_chains");

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
        "chain 1003 must grant Frost's letter (3730) to container 0 exactly once; \
         actions: {:?}",
        chain.actions,
    );

    // No pistol — that's the Guard corpse (chain 1005) now.
    let grants_pistol = chain
        .actions
        .iter()
        .any(|a| matches!(a, Action::GrantItem { item_id: 55, .. }));
    assert!(
        !grants_pistol,
        "chain 1003 (Frost) must NOT grant the pistol after the loot split; \
         the pistol comes from the Guard corpse (chain 1005). Actions: {:?}",
        chain.actions,
    );

    // No advance — Frost's chain must not own the step transition, or looting
    // Frost first would skip past the search step before the pistol is found.
    let advances = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::AdvanceStep {
                mission_id: 622,
                ..
            }
        )
    });
    assert!(
        !advances,
        "chain 1003 (Frost) must NOT advance mission 622; the Guard chain (1005) \
         owns the 2113 → 80622 advance. Actions: {:?}",
        chain.actions,
    );
}

/// Order-independence: chain 1003 (Frost letter) must STILL fire when the
/// player loots the Guard first (which advances 2113 → 80622). The letter gate
/// is `mission_status = active`, not step 2113, so the prior advance does not
/// lock Frost out. (This inverts the pre-split guard, which keyed on 2113.)
#[tokio::test]
async fn chain_1003_letter_still_fires_after_guard_advanced_the_step() {
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
    // Guard was looted first: 2113 completed, now on 80622. Frost's letter
    // must remain grantable.
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
        chain_1003_fired,
        "chain 1003 (Frost letter) must still fire after the Guard advanced the \
         step — the letter gate is mission-active, not step 2113, so loot order \
         doesn't matter. Got actions: {:?}",
        resolved.actions,
    );
}

/// Chain 1005 (Guard corpse, dialog 3996) grants the pistol (55) to the
/// backpack and advances to the equip step. No letter, no completion.
#[tokio::test]
async fn chain_1005_grants_pistol_and_advances_to_equip_step() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1005)
        .await
        .expect("DB query for chain 1005 must succeed")
        .expect("chain 1005 must exist in seeded content_chains");

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
        "chain 1005 must grant the pistol (55) to container 1 (backpack) exactly \
         once; actions: {:?}",
        chain.actions,
    );

    let advances = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::AdvanceStep {
                mission_id: 622,
                step_id: 80622
            }
        )
    });
    assert!(
        advances,
        "chain 1005 must advance mission 622 to step 80622 (it owns the advance). \
         Actions: {:?}",
        chain.actions,
    );

    // No letter, no completion — those belong to 1003 / 1004.
    assert!(
        !chain
            .actions
            .iter()
            .any(|a| matches!(a, Action::GrantItem { item_id: 3730, .. })),
        "chain 1005 must NOT grant the letter (that's Frost / chain 1003)",
    );
    assert!(
        !chain
            .actions
            .iter()
            .any(|a| matches!(a, Action::CompleteMission { mission_id: 622 })),
        "chain 1005 must NOT complete the mission (that's chain 1004 on equip)",
    );
}

/// Re-loot guard: chain 1005 (pistol) must NOT fire once step 2113 has been
/// advanced past — re-searching the Guard corpse can't re-grant the pistol or
/// re-advance. The step-gate flip is this chain's guard.
#[tokio::test]
async fn chain_1005_does_not_fire_after_step_2113_advances() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1005)
        .await
        .expect("DB query for chain 1005 must succeed")
        .expect("chain 1005 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(3996));
    ctx.set_param(
        "mission_622_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!("completed"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1005_fired = resolved.actions.iter().any(|(id, _)| *id == 1005);
    assert!(
        !chain_1005_fired,
        "chain 1005 must NOT fire after step 2113 advances — re-searching the \
         Guard would otherwise re-grant the pistol / re-advance. Got: {:?}",
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
