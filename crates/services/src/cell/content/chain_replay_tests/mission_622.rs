//! Mission 622 — Arm Yourself! Pin the **sequenced loot-split** +
//! equip-from-inventory flow. The loot is split across the two stasis-room
//! corpses and ordered Frost → Guard, with an intermediate step so the
//! sequence survives a relog:
//! - chain 1003 (Frost body, dialog 3995) grants ONLY the letter (3730) to the
//!   mission inventory, binds the Guard's dialog set (5230), and advances
//!   2113 → 80623 ("Search the NID Guard's body"). No pistol.
//! - chain 1005 (Guard corpse, dialog 3996) grants the pistol (55) to the
//!   backpack (container 1) and advances 80623 → 80622 ("Equip the pistol").
//! - chain 1004 fires on `item_equipped(55)` and is the only path that plays
//!   kismet sequence 10000 (opens the stasis-room door) + completes the mission.
//! - chains 1006/1007 re-bind Frost / the Guard on `player_loaded` at step
//!   2113 / 80623 respectively (interaction bindings aren't persisted).
//!
//! Sequencing: the Guard corpse's dialog (5230 → template 21) is NOT bound at
//! mission accept (chain 1001 binds only Frost's 5229), so the Guard is not
//! searchable until chain 1003 binds 5230 on the Frost dialog_open. That
//! binding gate lives at the interaction layer (`available_interactions` in
//! cell/interactions/dispatch.rs), which the chain-replay harness here does
//! not model — these tests pin the chain *conditions* and *actions*: the
//! per-step gates (1003 on 2113, 1005 on 80623, 1004 on 80622), the advances,
//! the re-loot guards (gates flip false on advance), and that the
//! `add_dialog_set` unlocks live on the right chains (1003/1006/1007, not 1001).
//!
//! Steps 80623 + 80622 are Cimmeria-introduced; their matching `<Steps>` rows
//! are shipped to the client via two `mission_overrides` patches on `_622` in
//! `CookedDataMissions.pak` (XML order 2113 → 80623 → 80622).

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 1003 (Frost) grants ONLY the letter, then advances to the
/// intermediate Guard-search step 80623 (NOT the equip step 80622, and NOT
/// straight to completion). The pistol comes from the Guard corpse (chain
/// 1005). Frost owning the 2113 → 80623 advance is what self-gates it (the
/// step-2113 gate flips false) and opens chain 1005's 80623 gate.
#[tokio::test]
async fn chain_1003_grants_letter_and_advances_to_guard_search_step() {
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
        "chain 1003 (Frost) must NOT grant the pistol; the pistol comes from the \
         Guard corpse (chain 1005). Actions: {:?}",
        chain.actions,
    );

    // Advances to 80623 (the Guard-search step), and ONLY to 80623 — never
    // straight to the equip step 80622, which would skip the Guard search.
    let advances_to_80623 = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::AdvanceStep {
                mission_id: 622,
                step_id: 80623
            }
        )
    });
    assert!(
        advances_to_80623,
        "chain 1003 (Frost) must advance mission 622 to the intermediate \
         Guard-search step 80623. Actions: {:?}",
        chain.actions,
    );
    let advances_to_80622 = chain.actions.iter().any(|a| {
        matches!(
            a,
            Action::AdvanceStep {
                mission_id: 622,
                step_id: 80622
            }
        )
    });
    assert!(
        !advances_to_80622,
        "chain 1003 (Frost) must NOT advance straight to the equip step 80622 — \
         that would skip the Guard search. Actions: {:?}",
        chain.actions,
    );
}

/// Frost's gate is `step_status 2113 = active`: it fires on the opening step
/// and self-limits once it advances to 80623 (a re-click can't re-grant the
/// letter or re-advance). Pins the gate + the resolved 80623 advance together.
#[tokio::test]
async fn chain_1003_fires_on_step_2113_and_advances_to_80623() {
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
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let advances_to_80623 = resolved.actions.iter().any(|(id, a)| {
        *id == 1003
            && matches!(
                a,
                Action::AdvanceStep {
                    mission_id: 622,
                    step_id: 80623
                }
            )
    });
    assert!(
        advances_to_80623,
        "chain 1003 must fire on step 2113 and advance to 80623; got {:?}",
        resolved.actions,
    );
}

/// Re-loot guard: chain 1003 must NOT fire once it has advanced past step 2113
/// (to 80623). Re-clicking Frost can't re-grant the letter or re-advance.
#[tokio::test]
async fn chain_1003_does_not_fire_after_advancing_to_80623() {
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
    // Frost already searched: 2113 completed, now on 80623.
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_622_step_80623_status".to_string(),
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
        "chain 1003 must NOT re-fire after advancing to 80623 — the step-2113 \
         gate is the re-loot guard. Got actions: {:?}",
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

/// Chain 1005's gate is `step_status 80623 = active` — the Guard is searchable
/// only AFTER Frost advanced us to the intermediate step. Positive case: it
/// fires (grants the pistol, advances to 80622) when 80623 is active.
#[tokio::test]
async fn chain_1005_fires_on_step_80623() {
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
        "mission_622_step_80623_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let advances_to_equip = resolved.actions.iter().any(|(id, a)| {
        *id == 1005
            && matches!(
                a,
                Action::AdvanceStep {
                    mission_id: 622,
                    step_id: 80622
                }
            )
    });
    assert!(
        advances_to_equip,
        "chain 1005 must fire on step 80623 and advance to the equip step 80622; \
         got {:?}",
        resolved.actions,
    );
}

/// Order guard: chain 1005 must NOT fire while the player is still on the
/// OPENING step 2113 (i.e. Frost not yet searched, so step 80623 isn't active).
/// The Guard's dialog isn't even bound at that point, but gating defensively on
/// 80623 (not 2113) means that even a spoofed dialog_open can't grant the
/// pistol before Frost is searched.
#[tokio::test]
async fn chain_1005_does_not_fire_before_frost_searched() {
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
    // Still on the opening step — Frost not yet searched.
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!("active"),
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
        "chain 1005 must NOT fire on step 2113 (before Frost is searched) — the \
         gate is step 80623. Got: {:?}",
        resolved.actions,
    );
}

/// Re-loot guard: chain 1005 must NOT fire once it has advanced past 80623 (to
/// the equip step 80622). Re-searching the Guard can't re-grant the pistol.
#[tokio::test]
async fn chain_1005_does_not_fire_after_advancing_to_80622() {
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
    // Guard already searched: 80623 completed, now on the equip step 80622.
    ctx.set_param(
        "mission_622_step_80623_status".to_string(),
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
    let chain_1005_fired = resolved.actions.iter().any(|(id, _)| *id == 1005);
    assert!(
        !chain_1005_fired,
        "chain 1005 must NOT re-fire after advancing to 80622 — the step-80623 \
         gate is the re-loot guard. Got: {:?}",
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

/// Sequencing guard: searching Frost is what unlocks the Guard corpse.
/// Chain 1003 (Frost dialog_open) must bind the Guard's dialog set
/// (5230 → template 21); chain 1001 (mission accept) must NOT. If the bind
/// moved back to 1001, both corpses would be searchable at once and the
/// Frost → Guard ordering the player asked for would be gone.
#[tokio::test]
async fn frost_loot_unlocks_guard_and_accept_does_not_bind_guard() {
    let pool = require_db_or_skip!();

    let chain_1001 = load_single_chain_for_test(&pool, 1001)
        .await
        .expect("DB query for chain 1001 must succeed")
        .expect("chain 1001 must exist in seeded content_chains");
    let chain_1003 = load_single_chain_for_test(&pool, 1003)
        .await
        .expect("DB query for chain 1003 must succeed")
        .expect("chain 1003 must exist in seeded content_chains");

    let binds_guard = |c: &cimmeria_content_engine::chain::Chain| {
        c.actions.iter().any(|a| {
            matches!(
                a,
                Action::AddDialogSet {
                    dialog_set_id: 5230,
                    slot: 21,
                    ..
                }
            )
        })
    };

    assert!(
        binds_guard(&chain_1003),
        "chain 1003 (Frost loot) must bind the Guard's dialog set (5230 → \
         template 21) so searching Frost unlocks the Guard. Actions: {:?}",
        chain_1003.actions,
    );
    assert!(
        !binds_guard(&chain_1001),
        "chain 1001 (mission accept) must NOT bind the Guard's dialog set — \
         that would make the Guard searchable before Frost, defeating the \
         Frost → Guard sequencing. Actions: {:?}",
        chain_1001.actions,
    );

    // Frost's own dialog set (5229 → template 14) must STILL be bound at
    // accept — only the Guard's bind moved.
    let binds_frost_at_accept = chain_1001.actions.iter().any(|a| {
        matches!(
            a,
            Action::AddDialogSet {
                dialog_set_id: 5229,
                slot: 14,
                ..
            }
        )
    });
    assert!(
        binds_frost_at_accept,
        "chain 1001 must still bind Frost's dialog set (5229 → template 14) at \
         mission accept. Actions: {:?}",
        chain_1001.actions,
    );
}

/// Relog safety: the corpse dialog bindings (`available_interactions`) are not
/// persisted across a relog/restart, so login-restore chains must re-apply
/// them. Chain 1006 re-binds Frost (5229) while step 2113 is active; chain 1007
/// re-binds the Guard (5230) while the intermediate step 80623 is active. Both
/// fire on `player_loaded`. Without 1007, the intermediate step would buy
/// nothing — a player who searched Frost then logged out would lose the Guard
/// binding and the pistol would be unobtainable.
#[tokio::test]
async fn login_restore_chains_rebind_corpses_per_step() {
    let pool = require_db_or_skip!();

    let chain_1006 = load_single_chain_for_test(&pool, 1006)
        .await
        .expect("DB query for chain 1006 must succeed")
        .expect("chain 1006 (Frost login restore) must exist in seeded content_chains");
    let chain_1007 = load_single_chain_for_test(&pool, 1007)
        .await
        .expect("DB query for chain 1007 must succeed")
        .expect("chain 1007 (Guard login restore) must exist in seeded content_chains");

    // Chain 1006 must re-bind Frost (5229 → template 14).
    assert!(
        chain_1006.actions.iter().any(|a| matches!(
            a,
            Action::AddDialogSet {
                dialog_set_id: 5229,
                slot: 14,
                ..
            }
        )),
        "chain 1006 must re-bind Frost's dialog set (5229 → template 14) on \
         login. Actions: {:?}",
        chain_1006.actions,
    );

    // Chain 1007 must re-bind the Guard (5230 → template 21).
    assert!(
        chain_1007.actions.iter().any(|a| matches!(
            a,
            Action::AddDialogSet {
                dialog_set_id: 5230,
                slot: 21,
                ..
            }
        )),
        "chain 1007 must re-bind the Guard's dialog set (5230 → template 21) on \
         login. Actions: {:?}",
        chain_1007.actions,
    );
}

/// The login-restore chains must actually FIRE on `player_loaded` at their
/// respective steps — pins the trigger + step gate together so a future edit
/// that loosens the gate (or changes the trigger) trips here. Chain 1007 in
/// particular is the whole point of the intermediate step: it must fire while
/// step 80623 is active.
#[tokio::test]
async fn chain_1007_fires_on_login_at_step_80623() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1007)
        .await
        .expect("DB query for chain 1007 must succeed")
        .expect("chain 1007 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    // OnPlayerLoaded matches on the `world_name` param (loaded from the
    // trigger's event_key); the chain's key is 'Castle_CellBlock'.
    ctx.set_param(
        "world_name".to_string(),
        serde_json::json!("Castle_CellBlock"),
    );
    ctx.set_param(
        "mission_622_status".to_string(),
        serde_json::json!("active"),
    );
    ctx.set_param(
        "mission_622_step_80623_status".to_string(),
        serde_json::json!("active"),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let rebinds_guard = resolved.actions.iter().any(|(id, a)| {
        *id == 1007
            && matches!(
                a,
                Action::AddDialogSet {
                    dialog_set_id: 5230,
                    slot: 21,
                    ..
                }
            )
    });
    assert!(
        rebinds_guard,
        "chain 1007 must re-bind the Guard (5230) on player_loaded at step \
         80623; got {:?}",
        resolved.actions,
    );
}
