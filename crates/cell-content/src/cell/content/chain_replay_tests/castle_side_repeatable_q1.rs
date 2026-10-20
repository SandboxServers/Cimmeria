//! Castle repeatable_q1 Supply Run (mission 10330), CS-05b1c1: the chains
//! `castle_repeatable_chains.sql` seeds as 10230-10237.
//!
//! The offer binds the supply officer (1000016 at slot 424) once 704 is
//! completed and 10330 is not active. The officer click (10234) removes the
//! offer, accepts, and shows dialog 10729. Every click goes through the real
//! dispatcher (`fire_interact_tag`, `fire_player_loaded`), so a resolve-only
//! check cannot pass a broken executor arm.
//!
//! Each test fails if its seed row is removed: without 10234 the click
//! accepts nothing, and without 10231 the login binds no offer.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
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

pub(super) const ENTITY: u32 = 1;
pub(super) const PLAYER: i32 = 42;
/// The supply officer NPC, the interaction target of the 10234 click.
pub(super) const OFFICER: u32 = 3;
pub(super) const CASTLE: &str = "Castle";
pub(super) const OFFICER_TAG: &str = "Castle_HubSupplyOfficer";
pub(super) const MISSION: i32 = 10330;
pub(super) const PREREQ: i32 = 704;
pub(super) const STEP_A: i32 = 10384;
pub(super) const STEP_B: i32 = 10385;
pub(super) const OBJ_A: i32 = 10504;
pub(super) const OBJ_B: i32 = 10505;
/// The offer: dialog-set map 1000016 bound at slot 424.
pub(super) const OFFER_SET: i32 = 1000016;
pub(super) const TURN_IN_SET: i32 = 1000017;
pub(super) const SLOT: i32 = 424;
const OFFER_DIALOG: i32 = 10729;
pub(super) const TURN_IN_DIALOG: i32 = 10730;
/// Wire index of the client method onDialogDisplay.
pub(super) const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the Supply Run, 10230-10237.
pub(super) const ALL_CHAINS: [i32; 8] = [10230, 10231, 10232, 10233, 10234, 10235, 10236, 10237];

// ── Harness ──────────────────────────────────────────────────────────────

pub(super) async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
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

/// Castle with player `ENTITY` holding `missions` and the officer actor, with
/// the dialog-set and monologue caches loaded from the seed.
pub(super) async fn staged(pool: &PgPool, missions: Vec<MissionInstance>) -> SpaceManager {
    staged_in(pool, CASTLE, missions).await
}

/// As [`staged`], with the player and the officer in `world` (`Castle` is
/// world 8, `CombatSim` is world 1).
pub(super) async fn staged_in(
    pool: &PgPool,
    world: &str,
    missions: Vec<MissionInstance>,
) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /><Space WorldName="CombatSim" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<Spaces><Space WorldName="Castle" /><Space WorldName="CombatSim" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(OFFICER, world, [3.0; 3], [0.0; 3])
        .unwrap();
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

/// 704 completed: the prerequisite the offer gates on.
pub(super) fn completed_704() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10330 at `step` with `objectives` active, as the accept would leave it.
pub(super) fn supply_mission(step: i32, objectives: &[i32]) -> MissionInstance {
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

pub(super) fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// The dialog-set-map ids bound on the player at `slot`, ascending.
pub(super) fn bound_at(mgr: &SpaceManager, slot: i32) -> Vec<i32> {
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

pub(super) fn offered(mgr: &SpaceManager) -> Vec<i32> {
    mgr.get_entity(ENTITY).unwrap().offered_dialogs()
}

pub(super) async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// Click the officer; returns whether a chain handled the click and
/// everything sent to the cell-to-base channel.
pub(super) async fn click_officer(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, OFFICER_TAG, OFFICER, engine, &tx, mgr).await;
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

/// Debug text of a `mission_status = expected` condition.
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

/// Debug text of a `mission_status neq active` condition (10330 may repeat).
fn mission_not_active(mission_id: i32) -> String {
    format!(
        "{:?}",
        Condition::MissionStatus {
            mission_id,
            operator: ComparisonOp::Neq,
            expected_status: MissionStatusValue::Active
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

fn add_set(dialog_set_id: i32) -> Action {
    Action::AddDialogSet {
        dialog_set_id,
        slot: SLOT,
        mission_id: None,
    }
}

fn remove_set(dialog_set_id: i32) -> Action {
    Action::RemoveDialogSet {
        dialog_set_id,
        slot: SLOT,
    }
}

/// **Chains 10230-10237 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list, action list in sort order, plus enabled, priority 0, not
/// `once` and no delays. Revert proof: drop 10234's `mission_status 10330
/// neq active` condition, or swap its accept and dialog actions; the
/// comparison fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_chains_load() {
    let pool = require_db_or_skip!();
    let offer_gate = || vec![world_8(), mission_is(PREREQ, MissionStatusValue::Completed)];
    let offer_gate_open = || {
        vec![
            world_8(),
            mission_is(PREREQ, MissionStatusValue::Completed),
            mission_not_active(MISSION),
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
    let sealed: [(i32, &str, Vec<String>, Vec<Action>); 8] = [
        (
            10230,
            "OnMissionCompleted { mission_id: 704 }",
            offer_gate_open(),
            vec![add_set(OFFER_SET)],
        ),
        (
            10231,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            offer_gate_open(),
            vec![add_set(OFFER_SET)],
        ),
        (
            10232,
            "OnMissionAbandoned { mission_id: 10330 }",
            offer_gate(),
            vec![remove_set(TURN_IN_SET), add_set(OFFER_SET)],
        ),
        (
            10233,
            "OnMissionCompleted { mission_id: 10330 }",
            offer_gate(),
            vec![add_set(OFFER_SET)],
        ),
        (
            10234,
            "OnInteractTag { entity_tag: \"Castle_HubSupplyOfficer\" }",
            offer_gate_open(),
            vec![
                remove_set(OFFER_SET),
                Action::AcceptMission {
                    mission_id: MISSION,
                },
                Action::DisplayDialog {
                    dialog_id: OFFER_DIALOG,
                },
            ],
        ),
        (
            10235,
            "OnRegionEnter { region_key: \"Castle.Infirmary\" }",
            vec![
                world_8(),
                mission_is(MISSION, MissionStatusValue::Active),
                step_active(STEP_A),
            ],
            vec![
                Action::AdvanceStep {
                    mission_id: MISSION,
                    step_id: STEP_B,
                },
                add_set(TURN_IN_SET),
            ],
        ),
        (
            10236,
            "OnInteractTag { entity_tag: \"Castle_HubSupplyOfficer\" }",
            active_at(STEP_B),
            vec![
                remove_set(TURN_IN_SET),
                Action::DisplayDialog {
                    dialog_id: TURN_IN_DIALOG,
                },
                Action::CompleteMission {
                    mission_id: MISSION,
                },
            ],
        ),
        (
            10237,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            active_at(STEP_B),
            vec![add_set(TURN_IN_SET)],
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

/// **The officer click accepts, unbinds the offer and shows 10729, in that
/// order.** 704 done and 10330 not active: the login binds the offer (10231),
/// the click runs 10234. The traffic must show the accept before the dialog,
/// and the state must show A active, the offer unbound and 10729 offered. A
/// resolve-only check would not see the executor arms.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_accept_then_dialog_order() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_704()]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, SLOT),
        vec![OFFER_SET],
        "the offer must be bound after login"
    );

    let (handled, out) = click_officer(&engine, &mut mgr).await;

    assert!(handled, "chain 10234 must handle the click");
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
    let dialog = out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
                if *entity_id == ENTITY
                    && *method_index == ON_DIALOG_DISPLAY
                    && args.len() >= 8
                    && i32::from_le_bytes(args[4..8].try_into().unwrap()) == OFFER_DIALOG
        )
    });
    let (accept, dialog) = (
        accept.expect("the accept must persist 10330 at step A"),
        dialog.expect("the chain must send onDialogDisplay for dialog 10729"),
    );
    assert!(
        accept < dialog,
        "traffic order must be accept, then dialog; got {accept}, {dialog}"
    );
    assert_eq!(
        mgr.get_entity(ENTITY)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .and_then(|m| m.current_step_id),
        Some(STEP_A),
        "the accept must start at step A"
    );
    assert!(
        bound_at(&mgr, SLOT).is_empty(),
        "the offer must be unbound after accept; got {:?}",
        bound_at(&mgr, SLOT)
    );
    assert_eq!(
        offered(&mgr),
        vec![OFFER_DIALOG],
        "dialog 10729 must be shown"
    );
}

/// **A repeat click after accept fires nothing.** The mission is already
/// active at A, so 10234's `neq active` gate fails and 10236 needs step B:
/// no chain handles the click and there is no mission or dialog traffic.
/// Revert proof: drop 10234's `mission_status 10330 neq active` condition and
/// the click accepts again.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_repeat_click_noop() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(
        &pool,
        vec![completed_704(), supply_mission(STEP_A, &[OBJ_A])],
    )
    .await;

    let (handled, out) = click_officer(&engine, &mut mgr).await;

    assert!(
        !handled,
        "no chain may handle a click with 10330 active at step A"
    );
    assert!(
        !out.iter().any(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )),
        "a click with 10330 active must send no mission or dialog traffic"
    );
}
