//! Castle return_q1 side story (mission 10320), CS-04c1c1: the chains
//! `castle_side_stories_chains.sql` seeds as 10130-10133.
//!
//! The Harset to Castle crossing (`stargate_crossed 'Castle'`, origin world 57)
//! accepts 10320 once; the Marsh click at step A (`interact_tag
//! 'Castle_ColMarsh'`) shows dialog 10721, unbinds Marsh and completes the
//! objective. Every event goes through the real dispatcher, so a resolve-only
//! check cannot pass a broken executor arm.
//!
//! Each test fails if its seed row is removed: without 10130 the crossing
//! accepts nothing, and without 10132 the click never reports.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::fire_stargate_crossed;
use super::super::fire_interact_tag;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// Colonel Marsh, the interaction target of the 10132 click.
const MARSH: u32 = 3;
const CASTLE: &str = "Castle";
const HARSET: &str = "Harset";
const MARSH_TAG: &str = "Castle_ColMarsh";
const MISSION: i32 = 10320;
const PREREQ: i32 = 708;
const STEP_A: i32 = 10372;
const OBJ_A: i32 = 10492;
/// The Marsh bind: dialog-set map 1000012 at slot 10.
const BIND_SET: i32 = 1000012;
const BIND_SLOT: i32 = 10;
const DIALOG: i32 = 10721;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the return_q1 side story, 10130-10133.
const RETURN_Q1_CHAINS: [i32; 4] = [10130, 10131, 10132, 10133];

// ── Harness ──────────────────────────────────────────────────────────────

async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &id in ids {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

/// Player `ENTITY` in `world` holding `missions`, with Marsh in the same
/// world and the mission, objective, dialog-set and monologue caches loaded.
async fn stage(pool: &PgPool, world: &str, missions: Vec<MissionInstance>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&format!(
        r#"<Spaces><Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#
    ))
    .unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(MARSH, world, [3.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(ENTITY, world, [0.0; 3], [0.0; 3])
        .unwrap();
    let entity = mgr.get_entity_mut(ENTITY).unwrap();
    entity.is_player = true;
    entity.player_id = Some(PLAYER);
    entity.archetype_id = Some(1);
    for mission in missions {
        entity.missions.add_mission(mission);
    }
    mgr
}

/// 708 completed: the prerequisite the crossing accept gates on.
fn completed_708() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10320 at step A with its objective active, as the crossing accept leaves it.
fn return_mission() -> MissionInstance {
    let objectives = vec![MissionObjective {
        objective_id: OBJ_A,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }];
    let mut mission = MissionInstance::new(MISSION, STEP_A, objectives);
    mission.current_step_id = Some(STEP_A);
    mission
}

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// The dialog-set-map ids bound on the player at `slot`, ascending.
fn bound_at(mgr: &SpaceManager, slot: i32) -> Vec<i32> {
    let mut ids: Vec<i32> = mgr
        .get_entity(ENTITY)
        .unwrap()
        .available_interactions
        .get(&slot)
        .map(|v| v.iter().map(|&(id, _, _)| id).collect())
        .unwrap_or_default();
    ids.sort_unstable();
    ids
}

/// Cross from `mgr`'s world to Castle, collecting the cell-to-base traffic.
async fn cross_to_castle(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_stargate_crossed(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// Click Marsh; returns whether a chain handled the click and the traffic.
async fn click_marsh(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, MARSH_TAG, MARSH, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Position of the 10320 `MissionUpdate` whose step matches `step`.
fn update_at(out: &[CellToBaseMsg], step: Option<i32>) -> Option<usize> {
    out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                current_step_id,
                ..
            } if *current_step_id == step
        )
    })
}

// ── Seed shape ───────────────────────────────────────────────────────────

/// Debug text of a `world = id` condition.
fn world_is(world_id: i32) -> String {
    format!(
        "{:?}",
        Condition::World {
            operator: ComparisonOp::Eq,
            world_id
        }
    )
}

/// Debug text of a `mission_status` condition.
fn mission_is(mission_id: i32, expected_status: MissionStatusValue) -> String {
    format!(
        "{:?}",
        Condition::MissionStatus {
            mission_id,
            operator: ComparisonOp::Eq,
            expected_status
        }
    )
}

/// Debug text of a `step_status = active` condition on 10320.
fn step_active(step_id: i32) -> String {
    format!(
        "{:?}",
        Condition::StepStatus {
            mission_id: MISSION,
            step_id,
            operator: ComparisonOp::Eq,
            expected_status: StepStatusValue::Active
        }
    )
}

fn marsh_bind() -> Action {
    Action::AddDialogSet {
        dialog_set_id: BIND_SET,
        slot: BIND_SLOT,
        mission_id: None,
    }
}

fn marsh_unbind() -> Action {
    Action::RemoveDialogSet {
        dialog_set_id: BIND_SET,
        slot: BIND_SLOT,
    }
}

/// **Chains 10130-10133 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list, action list in sort order, plus enabled, priority 0, not
/// `once` and no delays. Revert proof: delete 10130 and the load fails; drop
/// its `mission_status 708` condition or move `complete_objective` ahead of
/// `display_dialog` in 10132 and the comparison fails.
#[tokio::test]
async fn live_db_castle_side_return_q1_chains_load() {
    let pool = require_db_or_skip!();
    let active_at_a = || {
        vec![
            world_is(8),
            mission_is(MISSION, MissionStatusValue::Active),
            step_active(STEP_A),
        ]
    };
    // (chain, trigger Debug, conditions in order, actions in order)
    let sealed: [(i32, &str, Vec<String>, Vec<Action>); 4] = [
        (
            10130,
            "OnStargateCrossed { destination_world: Some(\"Castle\") }",
            vec![
                world_is(57),
                mission_is(PREREQ, MissionStatusValue::Completed),
                mission_is(MISSION, MissionStatusValue::NotActive),
            ],
            vec![Action::AcceptMission {
                mission_id: MISSION,
            }],
        ),
        (
            10131,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            active_at_a(),
            vec![marsh_bind()],
        ),
        (
            10132,
            "OnInteractTag { entity_tag: \"Castle_ColMarsh\" }",
            active_at_a(),
            vec![
                Action::DisplayDialog { dialog_id: DIALOG },
                marsh_unbind(),
                Action::CompleteObjective {
                    mission_id: MISSION,
                    objective_id: OBJ_A,
                },
            ],
        ),
        (
            10133,
            "OnMissionAbandoned { mission_id: 10320 }",
            vec![
                world_is(8),
                mission_is(MISSION, MissionStatusValue::NotActive),
            ],
            vec![marsh_unbind()],
        ),
    ];
    for (id, trigger, conditions, actions) in sealed {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        assert!(chain.enabled, "chain {id} must be enabled");
        assert_eq!(chain.priority, 0, "chain {id} priority");
        assert!(!chain.once, "chain {id} must not be once");
        assert_eq!(
            format!("{:?}", chain.trigger),
            trigger,
            "chain {id} trigger"
        );
        let got: Vec<String> = chain.conditions.iter().map(|c| format!("{c:?}")).collect();
        assert_eq!(got, conditions, "chain {id} conditions in sort order");
        assert_eq!(chain.actions, actions, "chain {id} actions in sort order");
        assert!(
            chain.action_delays.iter().all(|&d| d == 0),
            "chain {id} must have no delays"
        );
    }
}

// ── Chains ───────────────────────────────────────────────────────────────

/// **The crossing accepts 10320 exactly once, from world 57 with 708 done.**
/// The first crossing persists the accept at step A and sends no dialog; a
/// second crossing fires nothing. Revert proof: delete chain 10130 and the
/// first crossing persists nothing. Dropping the `not_active` guard does not
/// fail this test, because the executor refuses a re-accept of a running
/// mission with no traffic; `live_db_castle_side_return_q1_crossing_gates` pins that
/// guard through its completed-10320 case.
#[tokio::test]
async fn live_db_castle_side_return_q1_crossing_accepts_once() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;
    let mut mgr = stage(&pool, HARSET, vec![completed_708()]).await;

    let first = cross_to_castle(&engine, &mut mgr).await;

    assert!(
        update_at(&first, Some(STEP_A)).is_some(),
        "the crossing must persist 10320 at step A; got {first:?}"
    );
    assert!(
        !first.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall { method_index, .. }
                if *method_index == ON_DIALOG_DISPLAY
        )),
        "the crossing must show no dialog"
    );
    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_ACTIVE,
        "the crossing accept must leave 10320 active"
    );

    let second = cross_to_castle(&engine, &mut mgr).await;

    assert!(
        second.is_empty(),
        "a second crossing must fire nothing; got {second:?}"
    );
}

/// **The crossing accepts nothing from world 8, without 708, or once 10320
/// is already running or done.** Revert proof: flip 10130's `world 57` to
/// `world 8`; the Harset-origin case then fails. Drop the `not_active` row and
/// the completed case fails (the executor already refuses the running case).
#[tokio::test]
async fn live_db_castle_side_return_q1_crossing_gates() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;

    let mut from_castle = stage(&pool, CASTLE, vec![completed_708()]).await;
    assert!(
        cross_to_castle(&engine, &mut from_castle).await.is_empty(),
        "a crossing from world 8 must not accept 10320"
    );

    let mut without_708 = stage(&pool, HARSET, Vec::new()).await;
    assert!(
        cross_to_castle(&engine, &mut without_708).await.is_empty(),
        "without 708 completed the crossing must not accept 10320"
    );

    let mut running = stage(&pool, HARSET, vec![completed_708(), return_mission()]).await;
    assert!(
        cross_to_castle(&engine, &mut running).await.is_empty(),
        "an active 10320 must not be re-accepted by a crossing"
    );

    let mut finished_return = MissionInstance::new(MISSION, 0, Vec::new());
    finished_return.status = MISSION_COMPLETED;
    finished_return.current_step_id = None;
    let mut done = stage(&pool, HARSET, vec![completed_708(), finished_return]).await;
    assert!(
        cross_to_castle(&engine, &mut done).await.is_empty(),
        "a completed 10320 must not be re-accepted by a crossing"
    );
}

/// **The Marsh click at step A reports, unbinds, then completes, in that
/// order.** Dialog 10721 is shown, the bind on slot 10 is gone and 10320's
/// objective completes after the dialog traffic. Revert proof: move
/// `complete_objective` ahead of `display_dialog` in 10132; the order
/// assertion fails.
#[tokio::test]
async fn live_db_castle_side_return_q1_marsh_click_reports_and_completes() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &RETURN_Q1_CHAINS).await;
    let mut mgr = stage(&pool, CASTLE, vec![completed_708(), return_mission()]).await;
    mgr.get_entity_mut(ENTITY)
        .unwrap()
        .available_interactions
        .entry(BIND_SLOT)
        .or_default()
        .push((BIND_SET, None, 0));
    assert_eq!(bound_at(&mgr, BIND_SLOT), vec![BIND_SET]);

    let (handled, out) = click_marsh(&engine, &mut mgr).await;

    assert!(handled, "chain 10132 must handle the Marsh click");
    let dialog = out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
                if *entity_id == ENTITY
                    && *method_index == ON_DIALOG_DISPLAY
                    && args.len() >= 8
                    && i32::from_le_bytes(args[4..8].try_into().unwrap()) == DIALOG
        )
    });
    let complete = out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                ..
            }
        )
    });
    let (dialog, complete) = (
        dialog.expect("the click must send onDialogDisplay for dialog 10721"),
        complete.expect("the click must persist the objective completion of 10320"),
    );
    assert!(
        dialog < complete,
        "traffic order must be dialog, then completion; got {dialog}, {complete}"
    );
    assert!(
        bound_at(&mgr, BIND_SLOT).is_empty(),
        "the Marsh bind must be removed after the click"
    );
    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_COMPLETED,
        "10320 must be completed after the Marsh report"
    );
}
