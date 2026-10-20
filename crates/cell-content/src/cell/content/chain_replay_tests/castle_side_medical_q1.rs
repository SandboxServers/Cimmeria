//! Castle Q1 medical side story (mission 10300), CS-04a1c1: the chains
//! `castle_side_stories_chains.sql` seeds as 10000-10006.
//!
//! The offer binds the medic (1000001 at slot 177) once 704 is completed.
//! The medic click (10003) accepts; the nested `mission_accepted` (10004)
//! advances A to B, unbinds the medic and shows dialog 10700. Every click
//! goes through the real dispatcher (`fire_interact_tag`, `fire_player_loaded`),
//! so a resolve-only check cannot pass a broken executor arm.
//!
//! Each test fails if its seed row is removed: without 10003 the click
//! accepts nothing, and without 10004 the step never reaches 10341.

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
use super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// The medic NPC, the interaction target of the 10003 click.
const MEDIC: u32 = 3;
const CASTLE: &str = "Castle";
const MEDIC_TAG: &str = "Castle_Pop_Infirmary_Medic1";
const MISSION: i32 = 10300;
const PREREQ: i32 = 704;
const STEP_A: i32 = 10340;
const STEP_B: i32 = 10341;
const OBJ_A: i32 = 10460;
const OBJ_B: i32 = 10461;
/// The medic offer: dialog-set map 1000001 bound at slot 177.
const OFFER_SET: i32 = 1000001;
const OFFER_SLOT: i32 = 177;
const DIALOG: i32 = 10700;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the medical side story, 10000-10006 (offer, relog and
/// abandon rebinds, accept, nested advance, courtyard completion, heal).
const ALL_CHAINS: [i32; 7] = [10000, 10001, 10002, 10003, 10004, 10005, 10006];

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

/// Castle with player `ENTITY` holding `missions` and the medic actor, with
/// the dialog-set and monologue caches loaded from the seed.
async fn staged(pool: &PgPool, missions: Vec<MissionInstance>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<Spaces><Space WorldName="Castle" /></Spaces>"#)
        .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(MEDIC, CASTLE, [3.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(ENTITY, CASTLE, [0.0; 3], [0.0; 3])
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

/// 704 completed: the prerequisite the offer gates on.
fn completed_704() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10300 at `step` with `objectives` active, as the accept would leave it.
fn medic_mission(step: i32, objectives: &[i32]) -> MissionInstance {
    let objectives = objectives
        .iter()
        .map(|&objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect();
    let mut mission = MissionInstance::new(MISSION, step, objectives);
    mission.current_step_id = Some(step);
    mission
}

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
}

fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr).and_then(|m| m.current_step_id)
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

fn offered(mgr: &SpaceManager) -> Vec<i32> {
    mgr.get_entity(ENTITY).unwrap().offered_dialogs()
}

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// Click the medic; returns whether a chain handled the click (false falls
/// through to the default dialog path) and everything sent to the
/// cell-to-base channel.
async fn click_medic(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, MEDIC_TAG, MEDIC, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

// ── Seed shape ───────────────────────────────────────────────────────────

/// Debug text of a `world = 8` condition.
fn world_8() -> String {
    format!(
        "{:?}",
        Condition::World {
            operator: ComparisonOp::Eq,
            world_id: 8
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

/// Debug text of a `step_status = active` condition.
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

fn offer_bind() -> Action {
    Action::AddDialogSet {
        dialog_set_id: OFFER_SET,
        slot: OFFER_SLOT,
        mission_id: None,
    }
}

/// **Chains 10000-10006 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list (type, ids, operator, status), action list (type, ids,
/// slot, step) in sort order, plus enabled, priority 0, not `once` and no
/// delays. Revert proof: delete 10004 and the load fails; drop 10003's
/// `mission_status 704` condition or swap 10004's action order and the
/// comparison fails.
#[tokio::test]
async fn live_db_castle_side_medical_q1_chains_load() {
    let pool = require_db_or_skip!();
    let offer_gate = || {
        vec![
            world_8(),
            mission_is(PREREQ, MissionStatusValue::Completed),
            mission_is(MISSION, MissionStatusValue::NotActive),
        ]
    };
    let active_at = |step| {
        vec![
            world_8(),
            mission_is(MISSION, MissionStatusValue::Active),
            step_active(step),
        ]
    };
    // (chain, trigger Debug, conditions in order, actions in order)
    let sealed: [(i32, &str, Vec<String>, Vec<Action>); 7] = [
        (
            10000,
            "OnMissionCompleted { mission_id: 704 }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10001,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10002,
            "OnMissionAbandoned { mission_id: 10300 }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10003,
            "OnInteractTag { entity_tag: \"Castle_Pop_Infirmary_Medic1\" }",
            offer_gate(),
            vec![Action::AcceptMission {
                mission_id: MISSION,
            }],
        ),
        (
            10004,
            "OnMissionAccepted { mission_id: 10300 }",
            vec![world_8(), step_active(STEP_A)],
            vec![
                Action::AdvanceStep {
                    mission_id: MISSION,
                    step_id: STEP_B,
                },
                Action::RemoveDialogSet {
                    dialog_set_id: OFFER_SET,
                    slot: OFFER_SLOT,
                },
                Action::DisplayDialog { dialog_id: DIALOG },
            ],
        ),
        (
            10005,
            "OnRegionEnter { region_key: \"Castle.CastleFrontCourtyard\" }",
            active_at(STEP_B),
            vec![Action::CompleteObjective {
                mission_id: MISSION,
                objective_id: OBJ_B,
            }],
        ),
        (
            10006,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            active_at(STEP_A),
            vec![Action::AdvanceStep {
                mission_id: MISSION,
                step_id: STEP_B,
            }],
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

// ── The accept ───────────────────────────────────────────────────────────

/// **The medic click accepts, advances A to B, unbinds and shows 10700, in
/// that order.** 704 done and 10300 not_active: the login binds the offer,
/// the click runs 10003 (accept), whose `mission_accepted` runs 10004. The
/// traffic must show the accept, then the step-B update, then the dialog;
/// the state must show B active, the offer unbound and 10700 offered. A
/// resolve-only check would not see the executor arms.
#[tokio::test]
async fn live_db_castle_side_medical_q1_accept_advances_and_shows_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_704()]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, OFFER_SLOT),
        vec![OFFER_SET],
        "the offer must be bound before the click"
    );

    let (handled, out) = click_medic(&engine, &mut mgr).await;

    assert!(handled, "chain 10003 must handle the click");
    let accept = out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                current_step_id: Some(STEP_A),
                ..
            }
        )
    });
    let advance = out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                current_step_id: Some(STEP_B),
                ..
            }
        )
    });
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
    let (accept, advance, dialog) = (
        accept.expect("the accept must persist 10300 at step A"),
        advance.expect("the nested advance must persist step B"),
        dialog.expect("the nested chain must send onDialogDisplay for dialog 10700"),
    );
    assert!(
        accept < advance && advance < dialog,
        "traffic order must be accept, advance to B, dialog; got {accept}, {advance}, {dialog}",
    );
    assert_eq!(step_of(&mgr), Some(STEP_B), "A must advance to B");
    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_ACTIVE,
        "the accept must leave the mission active"
    );
    assert!(
        bound_at(&mgr, OFFER_SLOT).is_empty(),
        "the medic offer must be unbound after accept; got {:?}",
        bound_at(&mgr, OFFER_SLOT)
    );
    assert_eq!(offered(&mgr), vec![DIALOG], "dialog 10700 must be shown");
}

/// **A repeat click after accept fires nothing.** The mission is already
/// active at A, so 10003's `not_active` gate fails: no chain handles the
/// click, and there is no accept, step change, dialog or mission traffic.
/// Revert proof: drop 10003's `mission_status 10300` condition and the click
/// is handled by the chain again.
#[tokio::test]
async fn live_db_castle_side_medical_q1_repeat_click_noop() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(
        &pool,
        vec![completed_704(), medic_mission(STEP_A, &[OBJ_A])],
    )
    .await;

    let (handled, out) = click_medic(&engine, &mut mgr).await;

    assert!(
        !handled,
        "no chain may handle a click with 10300 active (10003 is gated on not_active)"
    );
    assert!(
        !out.iter().any(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )),
        "a click with 10300 active must send no mission or dialog traffic"
    );
    assert_eq!(step_of(&mgr), Some(STEP_A), "the step must stay at A");
    assert!(offered(&mgr).is_empty(), "no dialog may be shown");
    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_ACTIVE,
        "the mission must stay active"
    );
}
