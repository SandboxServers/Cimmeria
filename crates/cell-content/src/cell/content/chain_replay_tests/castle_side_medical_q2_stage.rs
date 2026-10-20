//! Shared fixtures for the CS-04a2c2 Q2 (mission 10301) lifecycle and
//! refusal replays in `castle_side_medical_q2_lifecycle.rs`: the staged
//! Castle, the Q1 x Q2 mission-state matrix and the event-resolution probe.
//! Copied from `castle_side_medical_q2.rs` (c1), whose helpers are private.

use std::collections::BTreeSet;

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::fire_player_loaded;
use super::super::mission_context::{populate_mission_context, populate_world_context};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
pub(super) const ENTITY: u32 = 1;
pub(super) const PLAYER: i32 = 42;
/// The prisoner and the medic NPCs, staged in Castle for the matrix.
pub(super) const PRISONER: u32 = 2;
pub(super) const MEDIC: u32 = 3;
pub(super) const CASTLE: &str = "Castle";
/// A second world, outside world 8, where every Q2 chain must refuse.
pub(super) const COMBAT_SIM: &str = "CombatSim";
pub(super) const COURTYARD: &str = "Castle.CastleFrontCourtyard";
pub(super) const PRISONER_TAG: &str = "Castle_Pop_Prisoner5";
pub(super) const MEDIC_TAG: &str = "Castle_Pop_Infirmary_Medic1";
pub(super) const MISSION: i32 = 10301;
/// Q1 medical story (10300) and its prerequisite (704).
pub(super) const Q1: i32 = 10300;
pub(super) const PREREQ: i32 = 704;
pub(super) const Q1_STEP_A: i32 = 10340;
pub(super) const Q1_STEP_B: i32 = 10341;
pub(super) const Q1_OBJ_A: i32 = 10460;
pub(super) const Q1_OBJ_B: i32 = 10461;
pub(super) const STEP_A: i32 = 10342;
pub(super) const STEP_B: i32 = 10343;
pub(super) const OBJ_A: i32 = 10462;
pub(super) const OBJ_B: i32 = 10463;
/// Q1's offer map, bound at slot 177 while 10300 is not started.
pub(super) const Q1_SET: i32 = 1000001;
/// The prisoner offer: dialog-set map 1000002 bound at slot 417.
pub(super) const OFFER_SET: i32 = 1000002;
pub(super) const OFFER_SLOT: i32 = 417;
/// The medic turn-in: dialog-set map 1000003 bound at slot 177.
pub(super) const TURN_IN_SET: i32 = 1000003;
pub(super) const TURN_IN_SLOT: i32 = 177;
pub(super) const ACCEPT_DIALOG: i32 = 10701;
pub(super) const TURN_IN_DIALOG: i32 = 10702;
/// The Q1 chains, 10000-10006.
pub(super) const Q1_CHAINS: [i32; 7] = [10000, 10001, 10002, 10003, 10004, 10005, 10006];
/// The Q2 chains, 10010-10017.
pub(super) const Q2_CHAINS: [i32; 8] = [10010, 10011, 10012, 10013, 10014, 10015, 10016, 10017];

/// One event the dispatchers raise, with the one param that keys it.
pub(super) type EventKey = (TriggerType, Option<(&'static str, serde_json::Value)>);

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

/// Player `ENTITY` in `world` (Castle or CombatSim) holding `missions`, with
/// the prisoner and the medic actors and the caches loaded from the seed.
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
    mgr.create_entity(PRISONER, CASTLE, [3.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(MEDIC, CASTLE, [3.0; 3], [0.0; 3])
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

pub(super) async fn staged(pool: &PgPool, missions: Vec<MissionInstance>) -> SpaceManager {
    staged_in(pool, CASTLE, missions).await
}

/// A mission that is completed: the state a finished story leaves behind.
pub(super) fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// A mission active at `step` with `objectives` active, as an accept or an
/// advance would leave it.
pub(super) fn active_at(mission_id: i32, step: i32, objectives: &[i32]) -> MissionInstance {
    let objectives = objectives
        .iter()
        .map(|&objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect();
    let mut mission = MissionInstance::new(mission_id, step, objectives);
    mission.current_step_id = Some(step);
    mission
}

pub(super) fn mission_of(mgr: &SpaceManager, mission_id: i32) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(mission_id)
}

pub(super) fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr, MISSION).and_then(|m| m.current_step_id)
}

pub(super) fn drain<T>(rx: &mut mpsc::Receiver<T>) -> Vec<T> {
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

/// The player logs in to `world`, the real `player_loaded` dispatcher.
pub(super) async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager, world: &str) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, world, engine, &tx, mgr).await;
}

/// Every event the Q1 and Q2 chains listen on, each with its keying param.
pub(super) fn event_keys() -> Vec<EventKey> {
    vec![
        (TriggerType::PlayerLoaded, None),
        (
            TriggerType::InteractTag,
            Some(("entity_tag", MEDIC_TAG.into())),
        ),
        (
            TriggerType::InteractTag,
            Some(("entity_tag", PRISONER_TAG.into())),
        ),
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", PREREQ.into())),
        ),
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", Q1.into())),
        ),
        (
            TriggerType::MissionAccepted,
            Some(("mission_id", Q1.into())),
        ),
        (
            TriggerType::MissionAccepted,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::MissionAbandoned,
            Some(("mission_id", Q1.into())),
        ),
        (
            TriggerType::MissionAbandoned,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::RegionEnter,
            Some(("region_key", COURTYARD.into())),
        ),
    ]
}

/// The chain ids that resolve for `event` in the current state of `mgr`, built
/// as the dispatchers build the context (world, missions, archetype, key param).
pub(super) fn fired_by(
    engine: &ChainEngine,
    mgr: &SpaceManager,
    event: &EventKey,
) -> BTreeSet<i64> {
    let (trigger_type, key) = event;
    let mut ctx = ExecutionContext::new().with_source(EntityId(ENTITY as i32));
    if let Some((name, value)) = key {
        ctx.set_param(name.to_string(), value.clone());
    }
    populate_world_context(ENTITY, mgr, &mut ctx);
    let entity = mgr.get_entity(ENTITY).unwrap();
    populate_mission_context(entity, &mut ctx);
    ctx.set_param("archetype".to_string(), serde_json::json!(1));
    let event = TriggerEvent {
        trigger_type: trigger_type.clone(),
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

/// The Q1 state at `index`: 0 not started, 1 at A, 2 at B, 3 completed.
pub(super) fn q1_state(index: usize) -> Option<MissionInstance> {
    match index {
        0 => None,
        1 => Some(active_at(Q1, Q1_STEP_A, &[Q1_OBJ_A])),
        2 => Some(active_at(Q1, Q1_STEP_B, &[Q1_OBJ_B])),
        _ => Some(completed(Q1)),
    }
}

/// The Q2 state at `index`: 0 not accepted, 1 at A, 2 at B, 3 completed.
pub(super) fn q2_state(index: usize) -> Option<MissionInstance> {
    match index {
        0 => None,
        1 => Some(active_at(MISSION, STEP_A, &[OBJ_A])),
        2 => Some(active_at(MISSION, STEP_B, &[OBJ_B])),
        _ => Some(completed(MISSION)),
    }
}

/// One cell of the 10300 x 10301 matrix, with 704 completed under it.
pub(super) fn cell(q1: usize, q2: usize) -> Vec<MissionInstance> {
    let mut missions = vec![completed(PREREQ)];
    missions.extend(q1_state(q1));
    missions.extend(q2_state(q2));
    missions
}
