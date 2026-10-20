//! The shared harness of the defense_q3 lifecycle tests: the sealed ids, the
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
use super::super::super::mission_context::{populate_mission_context, populate_world_context};
use super::super::super::{fire_entity_death, fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};

pub(super) const ENTITY: u32 = 1;
pub(super) const PLAYER: i32 = 42;
/// The second player of the two-player credit test.
pub(super) const ENTITY2: u32 = 5;
pub(super) const PLAYER2: i32 = 43;
/// The giver NPC, the interaction target of the 10123 click.
pub(super) const GIVER: u32 = 2;
pub(super) const CASTLE: &str = "Castle";
/// `resources.worlds` row 1; any world other than 8 refuses the giver.
pub(super) const COMBAT_SIM: &str = "CombatSim";
pub(super) const GIVER_TAG: &str = "Castle_DefenseQ3_Giver";
pub(super) const MISSION: i32 = 10312;
pub(super) const PREREQ: i32 = 10310;
pub(super) const STEP: i32 = 10366;
/// The four objectives, one per Bravo officer and the Muelbach kill.
pub(super) const OBJECTIVES: [i32; 4] = [10486, 10487, 10488, 10489];
/// The kill tags, paired index for index with `OBJECTIVES`.
pub(super) const KILL_TAGS: [&str; 4] = [
    "Castle_BravoOfficer1",
    "Castle_BravoOfficer2",
    "Castle_BravoOfficer3",
    "Castle_Muelbach",
];
/// The giver offer: dialog-set map 1000010 bound at slot 425.
pub(super) const OFFER_SET: i32 = 1000010;
pub(super) const OFFER_SLOT: i32 = 425;
pub(super) const OFFER_DIALOG: i32 = 10717;
pub(super) const DONE_DIALOG: i32 = 10718;
/// Every chain of the defense_q3 side story, 10120-10129.
pub(super) const ALL_CHAINS: [i32; 10] = [
    10120, 10121, 10122, 10123, 10124, 10125, 10126, 10127, 10128, 10129,
];

// ── Harness ──────────────────────────────────────────────────────────────

/// An engine holding every trigger expansion of `ids`. A chain with two
/// trigger rows expands to two chains, so the single-chain loader would hide
/// a second trigger from the disjointness test.
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

/// The giver and player `ENTITY` in `world`, the player holding `missions`,
/// with the production caches loaded from the seed.
pub(super) async fn staged(
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
    mgr.create_entity(GIVER, world, [3.0; 3], [0.0; 3]).unwrap();
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

/// 10310 not yet completed: the prerequisite is held but still running.
pub(super) fn open_prereq() -> MissionInstance {
    MissionInstance::new(PREREQ, 0, Vec::new())
}

/// 10310 completed: the prerequisite the offer gates on.
pub(super) fn completed_prereq() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10312 at its single step with every objective in `done` completed and
/// the rest active, as the accept and the kills would leave it.
pub(super) fn active_mission(done: &[i32]) -> MissionInstance {
    let active = OBJECTIVES
        .iter()
        .filter(|id| !done.contains(id))
        .map(|&objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect();
    let mut mission = MissionInstance::new(MISSION, STEP, active);
    mission.status = MISSION_ACTIVE;
    mission.current_step_id = Some(STEP);
    mission.completed_objectives = done.to_vec();
    mission
}

/// 10312 completed: the story is over, and no chain may bind or move it.
pub(super) fn finished_mission() -> MissionInstance {
    let mut mission = active_mission(&OBJECTIVES);
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

/// Everything the story can change on the player, as one comparable value:
/// 10312's status, step and completed objectives, the offer binding and the
/// dialogs on screen.
pub(super) fn snapshot(
    mgr: &SpaceManager,
) -> (Option<i8>, Option<i32>, Vec<i32>, Vec<i32>, Vec<i32>) {
    (
        mission_of(mgr).map(|m| m.status),
        mission_of(mgr).and_then(|m| m.current_step_id),
        mission_of(mgr)
            .map(|m| m.completed_objectives.clone())
            .unwrap_or_default(),
        bound_at(mgr, OFFER_SLOT),
        mgr.get_entity(ENTITY).unwrap().offered_dialogs(),
    )
}

pub(super) fn any_mission_traffic(out: &[CellToBaseMsg]) -> bool {
    out.iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                ..
            }
        )
    })
}

pub(super) async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// Click the giver; returns whether a chain handled the click and everything
/// sent to the cell-to-base channel.
pub(super) async fn click_giver(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, GIVER_TAG, GIVER, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Kill the NPC tagged `tag` as `ENTITY`; returns everything sent.
pub(super) async fn kill(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
    tag: &str,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_entity_death(ENTITY, PLAYER, tag, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// The chain ids the engine resolves for one event, built as its dispatcher
/// builds the context: the world, the mission state, the archetype and the
/// one key param. Resolution sees the conditions themselves, so it can show
/// a gate that the executor's own state check would hide.
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
    populate_mission_context(mgr.get_entity(ENTITY).unwrap(), &mut ctx);
    ctx.set_param("archetype".to_string(), serde_json::json!(1));
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

/// The events the story listens on, with the one param each dispatcher sets.
pub(super) fn event_keys() -> Vec<(TriggerType, Option<(&'static str, serde_json::Value)>)> {
    let mut keys = vec![
        (
            TriggerType::MissionCompleted,
            Some(("mission_id", PREREQ.into())),
        ),
        (TriggerType::PlayerLoaded, None),
        (
            TriggerType::MissionAbandoned,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::InteractTag,
            Some(("entity_tag", GIVER_TAG.into())),
        ),
        (
            TriggerType::MissionAccepted,
            Some(("mission_id", MISSION.into())),
        ),
    ];
    keys.extend(KILL_TAGS.iter().map(|tag| {
        (
            TriggerType::EntityDeath,
            Some(("entity_tag", (*tag).into())),
        )
    }));
    keys
}
