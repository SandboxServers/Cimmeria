//! The shared harness of the return_q1 lifecycle tests: the sealed ids, the
//! staged cell and the dispatcher wrappers. Copied from the c1 module rather
//! than shared, so a change to one file cannot move the other's assertions.

use std::collections::BTreeSet;

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::super::engine_loader::load_chain_expansions_for_test;
use super::super::super::event_dispatch::fire_stargate_crossed;
use super::super::super::mission_context::{populate_mission_context, populate_world_context};
use super::super::super::{fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};

pub(super) const ENTITY: u32 = 1;
pub(super) const PLAYER: i32 = 42;
/// Colonel Marsh, the interaction target of the 10132 click.
pub(super) const MARSH: u32 = 3;
pub(super) const CASTLE: &str = "Castle";
pub(super) const HARSET: &str = "Harset";
/// World names of the other worlds Marsh's bind must never reach.
pub(super) const CELLBLOCK: &str = "Castle_CellBlock";
pub(super) const CMD_CENTER: &str = "Harset_CmdCenter";
pub(super) const DAKARA: &str = "Dakara_E1";
pub(super) const MARSH_TAG: &str = "Castle_ColMarsh";
pub(super) const MISSION: i32 = 10320;
pub(super) const PREREQ: i32 = 708;
pub(super) const STEP_A: i32 = 10372;
pub(super) const OBJ_A: i32 = 10492;
/// The Marsh bind: dialog-set map 1000012 at slot 10.
pub(super) const BIND_SET: i32 = 1000012;
pub(super) const BIND_SLOT: i32 = 10;
pub(super) const DIALOG: i32 = 10721;
/// Chain 10130, the only chain of the side story that accepts 10320.
pub(super) const ACCEPT_CHAIN: i64 = 10130;
/// Every chain of the return_q1 side story, 10130-10133.
pub(super) const RETURN_Q1_CHAINS: [i32; 4] = [10130, 10131, 10132, 10133];

/// An engine holding every trigger expansion of `ids`. A chain with two
/// trigger rows expands to two chains, so the single-chain loader would hide
/// a second trigger from the tests that add one.
pub(super) async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &id in ids {
        let expansions = load_chain_expansions_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"));
        assert!(
            !expansions.is_empty(),
            "chain {id} must exist in seeded content_chains"
        );
        for chain in expansions {
            engine.register_chain(chain);
        }
    }
    engine
}

/// Player `ENTITY` in `world` holding `missions`, with Marsh in the same
/// world and the mission, objective, dialog-set and monologue caches loaded.
pub(super) async fn stage(
    pool: &PgPool,
    world: &str,
    missions: Vec<MissionInstance>,
) -> SpaceManager {
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
pub(super) fn completed_708() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 708 at step 2417, the step whose Marsh click chain 1352 owns.
pub(super) fn open_708() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 2417, Vec::new());
    mission.status = MISSION_ACTIVE;
    mission.current_step_id = Some(2417);
    mission
}

/// 10320 at step A with its objective active, as the crossing accept leaves it.
pub(super) fn return_mission() -> MissionInstance {
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

/// 10320 completed: the state after the Marsh report.
pub(super) fn finished_return() -> MissionInstance {
    let mut mission = MissionInstance::new(MISSION, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

pub(super) fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
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

/// The Marsh bind as a live cell entity holds it after a click or a relog.
pub(super) fn bind_marsh(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(ENTITY)
        .unwrap()
        .available_interactions
        .entry(BIND_SLOT)
        .or_default()
        .push((BIND_SET, None, 0));
}

/// (10320 status, 10320 step, Marsh bind on slot 10): what a state check reads.
pub(super) fn snapshot(mgr: &SpaceManager) -> (Option<i8>, Option<i32>, Vec<i32>) {
    (
        mission_of(mgr).map(|m| m.status),
        mission_of(mgr).and_then(|m| m.current_step_id),
        bound_at(mgr, BIND_SLOT),
    )
}

/// Fire `player_loaded` with the world name `claimed`. The staged world and
/// the claimed name are separate on purpose: `player_loaded 'Castle'` filters
/// on the name, so only a login that claims Castle reaches the chain's own
/// `world` condition, which is then the one guard under test.
pub(super) async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager, claimed: &str) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, claimed, engine, &tx, mgr).await;
}

/// Cross from `mgr`'s world to Castle, collecting the cell-to-base traffic.
pub(super) async fn cross_to_castle(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_stargate_crossed(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// Click Marsh; returns whether a chain handled the click and the traffic.
pub(super) async fn click_marsh(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, MARSH_TAG, MARSH, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Position of the 10320 `MissionUpdate` whose step matches `step`.
pub(super) fn update_at(out: &[CellToBaseMsg], step: Option<i32>) -> Option<usize> {
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

/// The chain ids the engine resolves for one event, built as the dispatchers
/// build the context: the world, the mission state, the archetype and the one
/// key param. Resolution sees the conditions themselves, so it shows a gate
/// that the executor's own state check would hide.
pub(super) fn resolve(
    engine: &ChainEngine,
    mgr: &SpaceManager,
    trigger_type: TriggerType,
    key: Option<(&str, serde_json::Value)>,
) -> BTreeSet<i64> {
    let mut ctx = ExecutionContext::new().with_source(EntityId(ENTITY as i32));
    if let Some((name, value)) = key {
        ctx.set_param(name.to_string(), value);
    }
    populate_world_context(ENTITY, mgr, &mut ctx);
    let entity = mgr.get_entity(ENTITY).unwrap();
    populate_mission_context(entity, &mut ctx);
    if let Some(archetype) = entity.archetype_id {
        ctx.set_param("archetype".to_string(), serde_json::json!(archetype));
    }
    let event = TriggerEvent {
        trigger_type,
        source_entity: Some(EntityId(ENTITY as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine
        .resolve_event(&event, &ctx)
        .actions
        .iter()
        .map(|(id, _)| *id)
        .collect()
}
