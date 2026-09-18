//! Step 2416 — "Retrieve the DHD Control Crystal." Chains 1346-1351.
//!
//! Five possible sources: three NID officers at Checkpoint Bravo
//! (optional objective 2798) or Warden Muelbach in the bunker above it
//! (optional objective 2799). Each grants item 2790 explicitly — the
//! crystal is not loot, and D-CA08 makes the STEP, not a `HasItem`
//! check, the proof of possession.
//!
//! The load-bearing property is that the crystal can be granted exactly
//! once. The guard is the step gate itself: every source chain advances
//! out of 2416 in the same action list it grants in, and `advance_step`
//! mutates `current_step_id` synchronously in memory
//! (`missions/progression.rs:76`) before returning. A second kill —
//! including re-killing an officer the respawn tick revived, which
//! reuses the same entity and tag — is a separate `fire_entity_death`
//! with a fresh context that sees step 2417.
//!
//! [`crystal_is_granted_only_once_across_two_officer_deaths`] proves
//! that end to end rather than by assertion: it executes the first
//! death's actions against a real `SpaceManager`, re-derives the context
//! from the mutated entity with the production
//! `populate_mission_context`, and re-resolves.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;
use cimmeria_entity::missions::{MissionInstance, MissionObjective, MISSION_ACTIVE, STATUS_ACTIVE};
use tokio::sync::mpsc;

use super::super::super::engine_loader::load_single_chain_for_test;
use super::super::super::executor::execute_actions;
use super::super::super::mission_context::populate_mission_context;
use super::{
    actions_of, assert_no_deferred_actions, engine_for, fire, make_castle_space_mgr, step_ctx,
    SOURCES, TAURI,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

/// DHD Control Crystal (`items.sql:11323`, `container_sets {2}`).
const CONTROL_CRYSTAL: i32 = 2790;
/// The Muelbach identifying drop (`items.sql:11846`).
const MUELBACH_ITEM: i32 = 2136;

const PLAYER_EID: u32 = 7181;
const PLAYER_ID: i32 = 7182;

/// Stage a player who is mid-708 on step 2416, with the step's real
/// objective set: required 2797 plus the two optional source objectives.
/// Reproducing the optional flags matters — `complete_objective`'s
/// auto-complete check filters on `!optional`
/// (`missions/progression.rs:176-180`), so a fixture that marked 2798
/// required would complete the whole mission and hide the bug this
/// guard exists for.
fn stage_player_on_step_2416(mgr: &mut SpaceManager) {
    mgr.create_entity(PLAYER_EID, "Castle", [800.0, 55.0, 515.0], [0.0; 3])
        .expect("Castle startup space must accept the player entity");
    let p = mgr
        .get_entity_mut(PLAYER_EID)
        .expect("player entity must exist immediately after create_entity");
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(TAURI);
    p.missions.add_mission(MissionInstance::new(
        708,
        2416,
        vec![
            MissionObjective {
                objective_id: 2797,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            },
            MissionObjective {
                objective_id: 2798,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: true,
            },
            MissionObjective {
                objective_id: 2799,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: true,
            },
        ],
    ));
    mgr.connect_entity(PLAYER_EID);
}

fn death_ctx_from_entity(mgr: &SpaceManager, tag: &str) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    let entity = mgr
        .get_entity(PLAYER_EID)
        .expect("player entity must exist");
    populate_mission_context(entity, &mut ctx);
    if let Some(archetype_id) = entity.archetype_id {
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
    }
    ctx
}

fn drain_grants(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u32, i32, i32, i32, bool)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id,
            count,
            notify_gm,
            ..
        } = msg
        {
            out.push((entity_id, player_id, item_id, count, notify_gm));
        }
    }
    out
}

/// Every officer chain resolves the same three actions in the same
/// order, and only for its own tag.
#[tokio::test]
async fn each_crystal_source_grants_2790_completes_its_objective_and_advances() {
    let pool = require_db_or_skip!();

    for (chain_id, tag, objective_id) in SOURCES {
        let engine = engine_for(&pool, chain_id).await;

        let mut ctx = step_ctx(2416, TAURI);
        ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));

        let resolved = fire(&engine, TriggerType::EntityDeath, &ctx);
        assert_no_deferred_actions(&resolved, chain_id as i64);
        let actions = actions_of(&resolved, chain_id as i64);

        assert!(
            matches!(
                actions.first(),
                Some(Action::GrantItem {
                    item_id: CONTROL_CRYSTAL,
                    count: 1,
                    container_id: None,
                })
            ),
            "chain {chain_id} must grant the Control Crystal first, with no \
             explicit container so `items.container_sets` decides; got {actions:?}",
        );
        assert_eq!(
            actions
                .iter()
                .filter(|a| matches!(
                    a,
                    Action::GrantItem {
                        item_id: CONTROL_CRYSTAL,
                        ..
                    }
                ))
                .count(),
            1,
            "chain {chain_id} must grant exactly one crystal per firing; got {actions:?}",
        );
        assert_eq!(
            actions
                .iter()
                .filter(|a| matches!(
                    a,
                    Action::CompleteObjective { mission_id: 708, objective_id: o } if *o == objective_id
                ))
                .count(),
            1,
            "chain {chain_id} must complete optional objective {objective_id}; got {actions:?}",
        );
        assert!(
            matches!(
                actions.last(),
                Some(Action::AdvanceStep {
                    mission_id: 708,
                    step_id: 2417
                })
            ),
            "chain {chain_id} must advance to 2417 LAST — the advance is the \
             single-grant guard and must follow the grant it closes; got {actions:?}",
        );

        // Cross-tag negative: this chain must not answer another source's
        // death, or a single kill would grant several crystals.
        for (_, other_tag, _) in SOURCES {
            if other_tag == tag {
                continue;
            }
            let mut other = step_ctx(2416, TAURI);
            other.set_param("entity_tag".to_string(), serde_json::json!(other_tag));
            assert!(
                actions_of(
                    &fire(&engine, TriggerType::EntityDeath, &other),
                    chain_id as i64
                )
                .is_empty(),
                "chain {chain_id} (tag {tag}) must not answer a death of {other_tag}",
            );
        }
    }
}

/// Muelbach additionally drops her identifying item. RECONSTRUCTION —
/// see chain 1349's seed comment — but pinned so it can't silently
/// disappear in a seed edit.
#[tokio::test]
async fn chain_1349_also_grants_the_muelbach_item() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1349).await;

    let mut ctx = step_ctx(2416, TAURI);
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Castle_Muelbach"),
    );

    let resolved = fire(&engine, TriggerType::EntityDeath, &ctx);
    let actions = actions_of(&resolved, 1349);
    assert_eq!(
        actions.len(),
        4,
        "chain 1349 must resolve four actions (crystal, 2136, objective, \
         advance); got {actions:?}",
    );
    assert_eq!(
        actions
            .iter()
            .filter(|a| matches!(
                a,
                Action::GrantItem {
                    item_id: MUELBACH_ITEM,
                    count: 1,
                    ..
                }
            ))
            .count(),
        1,
        "chain 1349 must grant item 2136 alongside the crystal; got {actions:?}",
    );
}

/// No source chain may fire before the player reaches step 2416.
/// Otherwise a player who cleared Checkpoint Bravo on the way in would
/// bank the crystal, and the step would advance the moment 708 reached
/// 2416 — or worse, `advance_step` would yank them out of 2415.
#[tokio::test]
async fn no_crystal_source_fires_outside_step_2416() {
    let pool = require_db_or_skip!();

    for (chain_id, tag, _) in SOURCES {
        let engine = engine_for(&pool, chain_id).await;
        for step in [2415, 2417, 2418, 4462, 4469] {
            let mut ctx = step_ctx(step, TAURI);
            ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
            assert!(
                actions_of(
                    &fire(&engine, TriggerType::EntityDeath, &ctx),
                    chain_id as i64
                )
                .is_empty(),
                "chain {chain_id} must not resolve while the player is on step {step}",
            );
        }

        // And not at all when 708 has never been accepted.
        let mut never = ExecutionContext::new();
        never.set_param("entity_tag".to_string(), serde_json::json!(tag));
        never.set_param(
            "mission_708_status".to_string(),
            serde_json::json!("not_active"),
        );
        assert!(
            actions_of(
                &fire(&engine, TriggerType::EntityDeath, &never),
                chain_id as i64
            )
            .is_empty(),
            "chain {chain_id} must not resolve before mission 708 is accepted",
        );

        // The case the `mission_status 708 eq active` row actually
        // defends, and the only one that can distinguish it from the step
        // gate: step 2416 reported active while the mission itself is
        // not. The step gate passes here, so if this resolved, the
        // mission row would be dead weight rather than the "cheap second
        // line" the seed calls it. `populate_mission_context` always
        // writes both keys together, so the pairing cannot occur in
        // production — a fixture is the only way to pin the row.
        let mut orphan_step = ExecutionContext::new();
        orphan_step.set_param("entity_tag".to_string(), serde_json::json!(tag));
        orphan_step.set_param(
            "mission_708_step_2416_status".to_string(),
            serde_json::json!("active"),
        );
        orphan_step.set_param(
            "mission_708_status".to_string(),
            serde_json::json!("not_active"),
        );
        orphan_step.set_param("archetype".to_string(), serde_json::json!(TAURI));
        assert!(
            actions_of(
                &fire(&engine, TriggerType::EntityDeath, &orphan_step),
                chain_id as i64
            )
            .is_empty(),
            "chain {chain_id} must not resolve on an orphaned active step 2416 \
             with mission 708 inactive — deleting the `mission_status 708 eq \
             active` condition row is what makes this resolve",
        );
    }
}

/// THE guard. Kill an officer, push the whole action list through
/// `execute_actions` against a real `SpaceManager`, then re-derive the
/// context from the mutated player with the production
/// `populate_mission_context` and kill a second officer. Exactly one
/// `CellToBaseMsg::GrantItem` for the crystal may reach base.
///
/// What fails when the change is reverted:
/// - deleting the `step_status 708 2416 eq active` condition row from
///   chains 1346-1349 → the second death resolves and a second
///   `GrantItem` appears;
/// - deleting the `advance_step` action row → the step never leaves
///   2416 and the second death resolves;
/// - giving `advance_step` a non-zero `delay_ms` → `execute_actions`
///   queues it instead of running it, the in-memory step stays 2416, and
///   the second death resolves.
///
/// Also pins the executor arm: a `GrantItem` that fell through
/// `execute_one`'s `other =>` catch-all would produce zero messages and
/// fail the first assertion.
#[tokio::test]
async fn crystal_is_granted_only_once_across_two_officer_deaths() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    for (chain_id, _, _) in SOURCES {
        let chain = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }

    let mut mgr = make_castle_space_mgr();
    stage_player_on_step_2416(&mut mgr);
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();

    // First kill: officer 1.
    let ctx = death_ctx_from_entity(&mgr, "Castle_BravoOfficer1");
    let first = fire(&engine, TriggerType::EntityDeath, &ctx);
    assert!(
        !first.actions.is_empty(),
        "the first officer death must resolve chain 1346 — a fixture that \
         resolves nothing would make the single-grant assertion vacuous",
    );
    execute_actions(first, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    let after_first = mgr
        .get_entity(PLAYER_EID)
        .and_then(|e| e.missions.get_mission(708))
        .cloned()
        .expect("mission 708 must still be tracked on the player");

    // The step must have moved in memory, synchronously.
    assert_eq!(
        after_first.current_step_id,
        Some(2417),
        "advance_step must have moved the player to 2417 inside execute_actions; \
         a deferred (delay_ms > 0) advance would leave this at 2416 and reopen \
         the crystal faucet",
    );
    // ...and the mission must still be running. Step 2416's ONLY required
    // objective is 2797; 2798/2799 are optional
    // (`mission_objectives.sql:6625/6629`). Flip 2797 to optional and
    // `complete_objective 2798` satisfies `all_required_complete`
    // (`missions/progression.rs:176-183`), ending 708 at step 2416 — and
    // the `current_step_id` assertion above would STILL pass, because
    // `advance_step` runs afterwards in the same action list and sets the
    // step regardless. Same guard as
    // `report.rs::assert_report_advances_without_completing`, one step
    // earlier.
    assert_eq!(
        after_first.status, MISSION_ACTIVE,
        "mission 708 must still be ACTIVE after the crystal grant — a \
         `completed` here means required objective 2797 was flipped to optional \
         and `complete_objective`'s all-required check now fires on the optional \
         source objective",
    );

    // Second kill: a different officer, context re-derived from the
    // entity exactly as `fire_entity_death` would.
    let ctx = death_ctx_from_entity(&mgr, "Castle_BravoOfficer2");
    let second = fire(&engine, TriggerType::EntityDeath, &ctx);
    assert!(
        second.actions.is_empty(),
        "a second officer death after the step advanced must resolve NOTHING; \
         got {:?}",
        second.actions,
    );
    execute_actions(second, PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &exec_engine).await;

    // Same for re-killing the SAME officer after a respawn: the respawn
    // tick reuses the entity and its tag, so the only thing standing
    // between the player and a second crystal is the step gate.
    let ctx = death_ctx_from_entity(&mgr, "Castle_BravoOfficer1");
    let respawned = fire(&engine, TriggerType::EntityDeath, &ctx);
    assert!(
        respawned.actions.is_empty(),
        "re-killing the same respawned officer must resolve nothing; got {:?}",
        respawned.actions,
    );
    execute_actions(
        respawned,
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &exec_engine,
    )
    .await;

    let grants = drain_grants(&mut rx);
    let crystals: Vec<_> = grants
        .iter()
        .filter(|(_, _, item_id, _, _)| *item_id == CONTROL_CRYSTAL)
        .collect();
    assert_eq!(
        crystals.len(),
        1,
        "exactly one Control Crystal may reach base across three deaths; got {grants:?}",
    );
    assert_eq!(
        *crystals[0],
        (PLAYER_EID, PLAYER_ID, CONTROL_CRYSTAL, 1, false),
        "the grant must be addressed to the killer with count 1 and \
         notify_gm = false (a chain grant is gameplay, not a GM action)",
    );
}

// Chains 1350/1351 (the report cue) live in [`super::report_cue`].
