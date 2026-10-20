//! Mission 1360 — Frost's Letter (C04,
//! docs/analysis/castle-cellblock-rebuild/work-packets.md#c04).
//!
//! D-CB03 (answered: accept). `ArmYourself.py` grants the letter (item
//! 3730) but never touched a mission for it — this is new content the spec
//! calls optional. Chain 1121 is a sibling to chain 1003 (identical
//! trigger + step gate: `dialog_open(3995)` while mission 622's step 2113
//! is active) that accepts mission 1360 alongside chain 1003's letter
//! grant, rather than folding the accept into chain 1003 itself (which
//! would force an unrelated `mission_status 1360` condition onto Frost's
//! item-grant/Guard-unlock/step-advance actions).
//!
//! - Chain 1121 (`dialog_open(3995)`, `step_status 622/2113 eq active` AND
//!   `mission_status 1360 eq not_active`): accepts mission 1360.
//!
//! CS-08a1 adds owner-approved reconstruction chain 1366: Castle-origin
//! crossing to Harset advances 4037 -> 4038 without changing inventory.
//! This module pins seeded resolution, real crossing dispatch and executor
//! traffic. Services pins the actual Cellblock grant and persistence through
//! fresh Castle/Harset hydration. Recipient completion belongs to HA-1360a/b.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Fire `dialog_open(3995)` against chain 1121 with the given
/// `step_2113_status` (mission 622's opening step) and `mission_1360_status`,
/// and assert whether it resolves an `AcceptMission { mission_id: 1360 }`.
async fn assert_frost_dialog_resolves(
    step_2113_status: &str,
    mission_1360_status: &str,
    should_fire: bool,
) {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1121)
        .await
        .expect("DB query for chain 1121 must succeed")
        .expect("chain 1121 must exist in seeded content_chains");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(3995));
    ctx.set_param(
        "mission_622_step_2113_status".to_string(),
        serde_json::json!(step_2113_status),
    );
    ctx.set_param(
        "mission_1360_status".to_string(),
        serde_json::json!(mission_1360_status),
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let accepts_1360 = resolved
        .actions
        .iter()
        .filter(|(id, a)| *id == 1121 && matches!(a, Action::AcceptMission { mission_id: 1360 }))
        .count();

    if should_fire {
        assert_eq!(
            accepts_1360, 1,
            "chain 1121 must resolve exactly one AcceptMission(1360) with \
             step_2113_status={step_2113_status:?}, mission_1360_status={mission_1360_status:?}; \
             got {accepts_1360} matches in {:?}",
            resolved.actions,
        );
    } else {
        assert_eq!(
            accepts_1360, 0,
            "chain 1121 must NOT resolve AcceptMission(1360) with \
             step_2113_status={step_2113_status:?}, mission_1360_status={mission_1360_status:?}; \
             got {accepts_1360} matches in {:?}",
            resolved.actions,
        );
    }
}

/// Positive: first Frost interaction (step 2113 active, mission 1360 never
/// accepted) — chain 1121 accepts 1360 exactly once. Pins T03's "1360
/// active after loot" acceptance criterion at the chain-resolution level.
#[tokio::test]
async fn live_db_chain_1121_accepts_1360_on_first_frost_loot() {
    assert_frost_dialog_resolves("active", "not_active", true).await;
}

/// Negative (re-loot guard): once step 2113 has advanced to 80623 (Frost
/// already searched — chain 1003 owns that advance), a second Frost
/// interaction must NOT re-accept 1360. Mirrors chain 1003's own
/// `chain_1003_does_not_fire_after_advancing_to_80623` guard — both chains
/// share the same step gate, so a second Frost dialog_open resolves neither.
#[tokio::test]
async fn live_db_chain_1121_does_not_refire_after_advancing_to_80623() {
    assert_frost_dialog_resolves("completed", "not_active", false).await;
}

/// Negative (mission offer guard): even if the step gate were somehow
/// still open, a second Frost interaction while 1360 is already ACTIVE
/// must not re-accept it. `cell::missions::lifecycle::accept_mission`'s
/// server-side offer guard would refuse this authoritatively regardless,
/// but the chain-level `mission_status 1360 eq not_active` condition
/// (content-chains.instructions.md "Mission grants must gate on
/// not_active") is the first line of defense pinned here.
#[tokio::test]
async fn live_db_chain_1121_does_not_refire_once_1360_already_active() {
    assert_frost_dialog_resolves("active", "active", false).await;
}

/// Negative: once mission 1360 is COMPLETED, chain 1121 must not re-fire
/// either — same offer-guard shape as the ACTIVE case above, covering the
/// other non-`not_active` status the mission lifecycle can be in.
#[tokio::test]
async fn live_db_chain_1121_does_not_refire_once_1360_completed() {
    assert_frost_dialog_resolves("active", "completed", false).await;
}

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_FAILED,
    MISSION_NOT_ACTIVE, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::event_dispatch::{fire_dialog_open, fire_stargate_crossed};
use super::super::executor::execute_actions;
use super::super::mission_context::{populate_mission_context, populate_world_context};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_item_containers, load_mission_defs, load_step_objectives, load_world_rows,
};

const LETTER_ENTITY: u32 = 1;
const LETTER_PLAYER: i32 = 42;

async fn letter_engine(pool: &PgPool, ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &id in ids {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .expect("load seeded chain");
        if let Some(chain) = chain {
            engine.register_chain(chain);
        } else {
            assert_eq!(id, 1366, "Cellblock and 708 prerequisite seeds must exist");
            // Revert proof must exercise the stranded 4037 state, rather
            // than fail before the real crossing dispatcher runs.
        }
    }
    engine
}

async fn letter_space(pool: &PgPool, world: &str) -> SpaceManager {
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
    mgr.item_containers = load_item_containers(pool)
        .await
        .expect("item placement cache");
    mgr.create_entity(LETTER_ENTITY, world, [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(LETTER_ENTITY).unwrap().player_id = Some(LETTER_PLAYER);
    mgr
}

fn put_letter_mission(mgr: &mut SpaceManager, status: i8, step: Option<i32>) {
    let mut mission = MissionInstance::new(
        1360,
        step.unwrap_or(0),
        vec![MissionObjective {
            objective_id: 4650,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }],
    );
    mission.status = status;
    mission.current_step_id = step;
    mgr.get_entity_mut(LETTER_ENTITY)
        .unwrap()
        .missions
        .add_mission(mission);
}

fn assert_delivery(mgr: &SpaceManager) {
    let mission = mgr
        .get_entity(LETTER_ENTITY)
        .unwrap()
        .missions
        .get_mission(1360)
        .unwrap();
    assert_eq!(mission.status, MISSION_ACTIVE);
    assert_eq!(mission.current_step_id, Some(4038));
    assert_eq!(mission.completed_steps, vec![4037]);
    assert_eq!(mission.completed_objectives, vec![4650]);
    assert_eq!(mission.active_objectives.len(), 1);
    assert_eq!(mission.active_objectives[0].objective_id, 4651);
    assert_eq!(mission.active_objectives[0].status, STATUS_ACTIVE);
}

fn assert_delivery_messages(messages: &[CellToBaseMsg]) {
    let updates: Vec<_> = messages
        .iter()
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::MissionUpdate {
                    mission_id: 1360,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        updates.len(),
        1,
        "one actual post-action persist: {messages:?}"
    );
    assert!(matches!(updates[0], CellToBaseMsg::MissionUpdate {
        player_id: LETTER_PLAYER, status: MISSION_ACTIVE, current_step_id: Some(4038),
        completed_step_ids, completed_objective_ids, active_objective_ids,
        failed_objective_ids, repeats: 0, ..
    } if completed_step_ids == &[4037] && completed_objective_ids == &[4650]
        && active_objective_ids == &[4651] && failed_objective_ids.is_empty()));
    // A closed traffic allowlist catches any inventory, reward, dialog,
    // teleport or binding action accidentally added to the bridge.
    assert!(
        messages.iter().all(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )),
        "unexpected crossing mutation: {messages:?}"
    );
}

#[tokio::test]
async fn live_db_castle_carry_in_metadata_and_frozen_bridge_shape() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, bool, i32, bool, i32, bool)> = sqlx::query_as(
        "SELECT s.step_id, s.index, s.step_enabled, o.objective_id, o.is_enabled, \
         t.task_id, t.is_enabled FROM resources.mission_steps s \
         JOIN resources.mission_objectives o ON o.step_id=s.step_id \
         JOIN resources.mission_tasks t ON t.objective_id=o.objective_id \
         WHERE s.mission_id=1360 ORDER BY s.index, o.objective_id, t.task_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            (4037, 0, true, 4650, true, 5693, true),
            (4038, 1, true, 4651, true, 5694, true)
        ]
    );
    let header: (String, i32, bool, i32) = sqlx::query_as(
        "SELECT scope_type, scope_id, enabled, priority FROM resources.content_chains WHERE chain_id=1366",
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(header, ("mission".into(), 1360, true, 0));
    let triggers: Vec<(String, String, String, bool, i32)> = sqlx::query_as(
        "SELECT event_type,event_key,scope,once,sort_order FROM resources.content_triggers WHERE chain_id=1366",
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(
        triggers,
        vec![(
            "stargate_crossed".into(),
            "Harset".into(),
            "player".into(),
            false,
            0
        )]
    );
    type ConditionRow = (
        String,
        Option<i32>,
        Option<String>,
        String,
        Option<String>,
        i32,
    );
    let conditions: Vec<ConditionRow> = sqlx::query_as(
        "SELECT condition_type,target_id,target_key,operator,value,sort_order \
         FROM resources.content_conditions WHERE chain_id=1366 ORDER BY sort_order",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        conditions,
        vec![
            ("world".into(), Some(8), None, "eq".into(), None, 0),
            (
                "mission_status".into(),
                Some(1360),
                None,
                "eq".into(),
                Some("active".into()),
                1
            ),
            (
                "step_status".into(),
                Some(1360),
                Some("4037".into()),
                "eq".into(),
                Some("active".into()),
                2
            ),
        ]
    );
    let chain = load_single_chain_for_test(&pool, 1366)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(chain.actions.len(), 1);
    assert!(matches!(
        chain.actions[0],
        Action::AdvanceStep {
            mission_id: 1360,
            step_id: 4038
        }
    ));
    assert_eq!(chain.action_delays, vec![0]);
    type ActionRow = (String, i32, String, serde_json::Value, i32, i32);
    let actions: Vec<ActionRow> = sqlx::query_as(
        "SELECT action_type,target_id,target_key,params,delay_ms,sort_order \
         FROM resources.content_actions WHERE chain_id=1366 ORDER BY sort_order",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        actions,
        vec![(
            "advance_step".into(),
            1360,
            "4038".into(),
            serde_json::json!({}),
            0,
            0
        )]
    );
}

#[tokio::test]
async fn live_db_mission_1360_seeded_frost_accept_then_real_castle_crossing() {
    let pool = require_db_or_skip!();
    let engine = letter_engine(&pool, &[1003, 1121, 1366]).await;
    let mut mgr = letter_space(&pool, "Castle").await;
    mgr.get_entity_mut(LETTER_ENTITY)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(622, 2113, vec![]));
    let (tx, mut rx) = mpsc::channel(128);
    fire_dialog_open(LETTER_ENTITY, LETTER_PLAYER, 3995, &engine, &tx, &mut mgr).await;
    let mut granted = Vec::new();
    while let Ok(m) = rx.try_recv() {
        granted.push(m);
    }
    assert_eq!(
        granted
            .iter()
            .filter(|m| matches!(
                m,
                CellToBaseMsg::GrantItem {
                    item_id: 3730,
                    container_id: 2,
                    count: 1,
                    ..
                }
            ))
            .count(),
        1,
        "seeded Frost action must resolve to mission bag 2: {granted:?}"
    );
    assert_eq!(
        mgr.get_entity(LETTER_ENTITY)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap()
            .current_step_id,
        Some(4037)
    );
    fire_stargate_crossed(
        LETTER_ENTITY,
        LETTER_PLAYER,
        "Harset",
        &engine,
        &tx,
        &mut mgr,
    )
    .await;
    assert_delivery(&mgr);
    let mut crossed = Vec::new();
    while let Ok(m) = rx.try_recv() {
        crossed.push(m);
    }
    assert_delivery_messages(&crossed);
    fire_stargate_crossed(
        LETTER_ENTITY,
        LETTER_PLAYER,
        "Harset",
        &engine,
        &tx,
        &mut mgr,
    )
    .await;
    assert!(
        rx.try_recv().is_err(),
        "repeat crossing must emit no mutation"
    );
}

#[tokio::test]
async fn live_db_mission_1360_crossing_guards_and_all_archetypes() {
    let pool = require_db_or_skip!();
    let engine = letter_engine(&pool, &[1366]).await;
    let negatives = [
        ("Harset", "Harset", MISSION_ACTIVE, Some(4037)),
        ("Harset_CmdCenter", "Harset", MISSION_ACTIVE, Some(4037)),
        ("Castle", "Abydos", MISSION_ACTIVE, Some(4037)),
        ("Castle", "Harset_CmdCenter", MISSION_ACTIVE, Some(4037)),
        ("Castle", "Harset", MISSION_NOT_ACTIVE, Some(4037)),
        ("Castle", "Harset", MISSION_COMPLETED, Some(4037)),
        ("Castle", "Harset", MISSION_FAILED, Some(4037)),
        ("Castle", "Harset", MISSION_ACTIVE, None),
        ("Castle", "Harset", MISSION_ACTIVE, Some(4038)),
    ];
    for (world, destination, status, step) in negatives {
        let mut mgr = letter_space(&pool, world).await;
        put_letter_mission(&mut mgr, status, step);
        let (tx, mut rx) = mpsc::channel(64);
        fire_stargate_crossed(
            LETTER_ENTITY,
            LETTER_PLAYER,
            destination,
            &engine,
            &tx,
            &mut mgr,
        )
        .await;
        assert!(
            rx.try_recv().is_err(),
            "must refuse {world}/{destination}/{status}/{step:?}"
        );
        let mission = mgr
            .get_entity(LETTER_ENTITY)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap();
        assert_eq!((mission.status, mission.current_step_id), (status, step));
    }
    for archetype in 1..=8 {
        let mut mgr = letter_space(&pool, "Castle").await;
        put_letter_mission(&mut mgr, MISSION_ACTIVE, Some(4037));
        mgr.get_entity_mut(LETTER_ENTITY).unwrap().archetype_id = Some(archetype);
        let (tx, mut rx) = mpsc::channel(64);
        fire_stargate_crossed(
            LETTER_ENTITY,
            LETTER_PLAYER,
            "Harset",
            &engine,
            &tx,
            &mut mgr,
        )
        .await;
        assert_delivery(&mgr);
        let mut messages = Vec::new();
        while let Ok(m) = rx.try_recv() {
            messages.push(m);
        }
        assert_delivery_messages(&messages);
    }
}

#[tokio::test]
async fn live_db_mission_1360_and_708_pre_action_context_both_orders() {
    let pool = require_db_or_skip!();
    let engine = letter_engine(&pool, &[1360, 1366]).await;
    for completed_708 in [false, true] {
        for bridge_first in [false, true] {
            let mut mgr = letter_space(&pool, "Castle").await;
            put_letter_mission(&mut mgr, MISSION_ACTIVE, Some(4037));
            let mut exit = MissionInstance::new(708, 4469, vec![]);
            if completed_708 {
                exit.complete();
            }
            mgr.get_entity_mut(LETTER_ENTITY)
                .unwrap()
                .missions
                .add_mission(exit);
            let mut ctx = ExecutionContext::new();
            populate_world_context(LETTER_ENTITY, &mgr, &mut ctx);
            populate_mission_context(mgr.get_entity(LETTER_ENTITY).unwrap(), &mut ctx);
            ctx.set_param("destination_world".into(), serde_json::json!("Harset"));
            let event = TriggerEvent {
                trigger_type: TriggerType::StargateCrossed,
                source_entity: None,
                target_entity: None,
                params: ctx.params.clone(),
            };
            let mut resolved = engine.resolve_event(&event, &ctx);
            assert_eq!(resolved.actions.len(), if completed_708 { 1 } else { 2 });
            assert!(resolved.actions.iter().any(|(id, action)| *id == 1366
                && matches!(
                    action,
                    Action::AdvanceStep {
                        mission_id: 1360,
                        step_id: 4038
                    }
                )));
            // The seed resolves both from one snapshot. Test either permitted
            // order without modifying the chains or rebuilding context midway.
            resolved
                .actions
                .sort_by_key(|(id, _)| if bridge_first { -*id } else { *id });
            let (tx, mut rx) = mpsc::channel(64);
            execute_actions(
                resolved,
                LETTER_ENTITY,
                LETTER_PLAYER,
                &tx,
                &mut mgr,
                &engine,
            )
            .await;
            assert_delivery(&mgr);
            assert_eq!(
                mgr.get_entity(LETTER_ENTITY)
                    .unwrap()
                    .missions
                    .get_mission(708)
                    .unwrap()
                    .status,
                MISSION_COMPLETED
            );
            let mut messages = Vec::new();
            while let Ok(m) = rx.try_recv() {
                messages.push(m);
            }
            assert_delivery_messages(&messages);
            assert_eq!(
                messages
                    .iter()
                    .filter(|m| matches!(
                        m,
                        CellToBaseMsg::MissionUpdate {
                            mission_id: 708,
                            ..
                        }
                    ))
                    .count(),
                usize::from(!completed_708)
            );
        }
    }
}
