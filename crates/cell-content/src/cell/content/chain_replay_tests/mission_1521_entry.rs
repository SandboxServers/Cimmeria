//! Mission 1521 (Castle symbiote tank, Option B) entry chains, CS-03c1:
//! `castle_symbiote_chains.sql` chains 1369 (Infirmary offer, archetype 8
//! only), 1370 (advance 4455 to 4456 on accept) and the tank starter 1374,
//! plus the tank's template and spawn rows.
//!
//! The offer test goes through the real `fire_enter_region` dispatcher, so
//! the executor runs the accept and its `mission_accepted` replay for real;
//! a resolve-only test cannot see 1370. The tank click goes through the real
//! `fire_interact_tag`. Each test names the seed row it pins.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_enter_region, fire_interact_tag};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{RegionData, SpaceManager, REGION_FLAG_CLIENT_HINTED};
use crate::cell::spawner::{
    load_item_containers, load_mission_defs, load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

pub(super) const ENTITY: u32 = 1;
pub(super) const TANK_ENTITY: u32 = 2;
pub(super) const PLAYER: i32 = 42;
pub(super) const CASTLE: &str = "Castle";
const INFIRMARY: &str = "Castle.Infirmary";
pub(super) const MISSION: i32 = 1521;
pub(super) const STEP_FIRST: i32 = 4455;
pub(super) const STEP_TANK: i32 = 4456;
pub(super) const TANK_TAG: &str = "Castle_SymbioteTank";

// ── Harness ──────────────────────────────────────────────────────────────

/// Register the named chains, panicking if any is missing from the seed.
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

/// A character in `world` with the given archetype (`None` is an entity
/// whose archetype was never stamped) and missions already on the record.
pub(super) async fn staged(
    pool: &PgPool,
    world: &str,
    archetype: Option<i32>,
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
    mgr.item_containers = load_item_containers(pool)
        .await
        .expect("item placement cache");
    // The Infirmary volume, registered in Castle only and containing the
    // origin where the player stands. The accept's step-activation replay
    // re-fires `enter_region` only for client-hinted volumes that contain
    // the server-known position, so without it the replay cannot run.
    mgr.regions.insert(
        1,
        RegionData {
            runtime_id: 1,
            db_set_id: 2051,
            tag: INFIRMARY.to_string(),
            world_name: CASTLE.to_string(),
            height: 4.0,
            radius: 10.0,
            flags: REGION_FLAG_CLIENT_HINTED,
            points: vec![
                [-10.0, 0.0, -10.0],
                [-10.0, 0.0, 10.0],
                [10.0, 0.0, 10.0],
                [10.0, 4.0, -10.0],
            ],
        },
    );
    mgr.create_entity(ENTITY, world, [0.0; 3], [0.0; 3])
        .unwrap();
    let entity = mgr.get_entity_mut(ENTITY).unwrap();
    entity.is_player = true;
    entity.player_id = Some(PLAYER);
    entity.archetype_id = archetype;
    for mission in missions {
        entity.missions.add_mission(mission);
    }
    mgr
}

/// A mission record at `step`, with `objectives` active.
pub(super) fn mission_at(id: i32, step: Option<i32>, objectives: &[i32]) -> MissionInstance {
    let objectives = objectives
        .iter()
        .map(|&objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect();
    let mut mission = MissionInstance::new(id, step.unwrap_or(0), objectives);
    mission.current_step_id = step;
    mission
}

/// Mission 704 (the prerequisite) completed, which is the normal state
/// for a character who can take 1520.
pub(super) fn completed_704() -> MissionInstance {
    let mut mission = mission_at(704, None, &[]);
    mission.status = MISSION_COMPLETED;
    mission
}

/// Fire the real `enter_region` dispatcher for the Infirmary and drain the
/// cell-to-base traffic it produced.
async fn enter(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, INFIRMARY, engine, &tx, mgr).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

pub(super) fn mission_of(mgr: &SpaceManager, id: i32) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY).unwrap().missions.get_mission(id)
}

// ── Offer: chain 1369 ────────────────────────────────────────────────────

/// The actions a seeded chain carries, read straight off the loaded chain.
async fn chain_actions_of(pool: &PgPool, id: i32) -> Vec<Action> {
    load_single_chain_for_test(pool, id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"))
        .actions
}

/// **Offer, archetype 8 only.** With 704 completed and 1521 never taken,
/// entering the Infirmary as a Jaffa accepts 1521, and the `mission_accepted`
/// replay of 1370 moves it 4455 to 4456. The credit for 5175 comes from
/// `advance_step`; 1370 never carries `complete_objective`.
#[tokio::test]
async fn live_db_1521_enter_region_offers_mission_to_archetype_8_and_advances_to_4456() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1369, 1370]).await;
    let mut mgr = staged(&pool, CASTLE, Some(8), vec![completed_704()]).await;
    enter(&engine, &mut mgr).await;

    let mission = mission_of(&mgr, MISSION).expect("archetype 8 must be offered mission 1521");
    assert_eq!(mission.status, MISSION_ACTIVE);
    assert_eq!(
        mission.current_step_id,
        Some(STEP_TANK),
        "1370 must advance 4455 to 4456 on accept"
    );
    assert!(
        mission.completed_objectives.contains(&5175),
        "advance_step must credit 5175 itself; got {:?}",
        mission.completed_objectives
    );
    assert_no_complete_objective(&pool).await;
}

/// No 1521 chain may carry `complete_objective`: every credit comes from
/// `advance_step`/`complete_mission`, and an explicit one double-credits.
async fn assert_no_complete_objective(pool: &PgPool) {
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_actions \
         WHERE chain_id IN (1369, 1370, 1374, 1375, 1376, 1377) \
           AND action_type::text = 'complete_objective'",
    )
    .fetch_one(pool)
    .await
    .expect("count complete_objective rows");
    assert_eq!(count, 0, "no 1521 chain may carry complete_objective");
    for id in [1369, 1370, 1374, 1375, 1376, 1377] {
        assert!(
            !chain_actions_of(pool, id)
                .await
                .iter()
                .any(|a| matches!(a, Action::CompleteObjective { .. })),
            "chain {id} must never carry complete_objective"
        );
    }
}

/// **No offer to archetypes 1-7, an unpopulated archetype, the wrong world,
/// a missing or unfinished 704, or 1521 already taken or finished.**
#[tokio::test]
async fn live_db_1521_enter_region_refuses_other_archetypes_and_gates() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1369, 1370]).await;
    for archetype in [
        Some(1),
        Some(2),
        Some(3),
        Some(4),
        Some(5),
        Some(6),
        Some(7),
        None,
    ] {
        let mut mgr = staged(&pool, CASTLE, archetype, vec![completed_704()]).await;
        enter(&engine, &mut mgr).await;
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "archetype {archetype:?} must not be offered mission 1521"
        );
    }

    let mut no_704 = staged(&pool, CASTLE, Some(8), vec![]).await;
    enter(&engine, &mut no_704).await;
    assert!(mission_of(&no_704, MISSION).is_none(), "704 never taken");

    let mut active_704 = staged(&pool, CASTLE, Some(8), vec![mission_at(704, Some(0), &[])]).await;
    enter(&engine, &mut active_704).await;
    assert!(
        mission_of(&active_704, MISSION).is_none(),
        "704 not completed"
    );

    let mut wrong_world = staged(&pool, "Castle_CellBlock", Some(8), vec![completed_704()]).await;
    enter(&engine, &mut wrong_world).await;
    assert!(mission_of(&wrong_world, MISSION).is_none(), "wrong world");

    // Active at the tank step: a re-entry must not re-accept or move it.
    let mut active = staged(
        &pool,
        CASTLE,
        Some(8),
        vec![completed_704(), mission_at(MISSION, Some(STEP_TANK), &[])],
    )
    .await;
    enter(&engine, &mut active).await;
    assert_eq!(
        mission_of(&active, MISSION).and_then(|m| m.current_step_id),
        Some(STEP_TANK),
        "re-entry must leave an active 1521 on 4456"
    );

    let mut done = staged(
        &pool,
        CASTLE,
        Some(8),
        vec![completed_704(), {
            let mut m = mission_at(MISSION, None, &[]);
            m.status = MISSION_COMPLETED;
            m
        }],
    )
    .await;
    enter(&engine, &mut done).await;
    assert_eq!(
        mission_of(&done, MISSION).map(|m| m.status),
        Some(MISSION_COMPLETED),
        "a completed 1521 is not re-offered"
    );
}

/// 1370 is a direct step gate: it carries the world row and the 4455 gate.
#[tokio::test]
async fn live_db_1521_accept_advance_is_gated_on_step_4455() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1370)
        .await
        .expect("query 1370")
        .expect("chain 1370 must be seeded");
    use cimmeria_content_engine::conditions::{Condition, StepStatusValue};
    assert!(
        chain
            .conditions
            .iter()
            .any(|c| matches!(c, Condition::World { world_id: 8, .. })),
        "1370 must require world 8; got {:?}",
        chain.conditions
    );
    assert!(
        chain.conditions.iter().any(|c| matches!(
            c,
            Condition::StepStatus {
                mission_id: MISSION,
                step_id: STEP_FIRST,
                expected_status: StepStatusValue::Active,
                ..
            }
        )),
        "1370 must require step 4455 active; got {:?}",
        chain.conditions
    );
    // The raw rows too, so the operator (which the loaded enum may fold)
    // cannot flip to neq unnoticed.
    let rows: Vec<(String, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT condition_type::text, target_key, operator::text, value \
         FROM resources.content_conditions \
         WHERE chain_id = 1370 AND condition_type::text = 'step_status'",
    )
    .fetch_all(&pool)
    .await
    .expect("query 1370 step_status rows");
    assert_eq!(
        rows,
        vec![(
            "step_status".to_string(),
            Some("4455".to_string()),
            "eq".to_string(),
            Some("active".to_string())
        )],
        "1370 gate row must be exactly step_status 1521/4455 eq active"
    );
    assert_no_complete_objective(&pool).await;
}

// ── Tank starter: chain 1374 ─────────────────────────────────────────────

/// Click the tank as a Jaffa at `step`: whether a chain handled it, and how
/// many `StartMinigame` sends it produced.
async fn click_tank(engine: &ChainEngine, pool: &PgPool, step: i32) -> (bool, usize) {
    click_tank_in(engine, pool, CASTLE, step).await
}

async fn click_tank_in(
    engine: &ChainEngine,
    pool: &PgPool,
    world: &str,
    step: i32,
) -> (bool, usize) {
    let mut mgr = staged(
        pool,
        world,
        Some(8),
        vec![completed_704(), mission_at(MISSION, Some(step), &[])],
    )
    .await;
    let (tx, mut rx) = mpsc::channel(64);
    let handled =
        fire_interact_tag(ENTITY, PLAYER, TANK_TAG, TANK_ENTITY, engine, &tx, &mut mgr).await;
    let mut starts = 0;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::StartMinigame { .. }) {
            starts += 1;
        }
    }
    (handled, starts)
}

/// **1374 starts Livewire at 4456 and is a no-op at 4455, 4457 and 4458.**
#[tokio::test]
async fn live_db_1521_tank_click_starts_livewire_only_at_4456() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1374]).await;
    assert_eq!(click_tank(&engine, &pool, STEP_TANK).await, (true, 1));
    for step in [STEP_FIRST, 4457, 4458] {
        assert_eq!(
            click_tank(&engine, &pool, step).await,
            (false, 0),
            "a tank click at step {step} must do nothing"
        );
    }
    // Pins 1374's world row: the right step in the wrong world is a no-op.
    assert_eq!(
        click_tank_in(&engine, &pool, "Castle_CellBlock", STEP_TANK).await,
        (false, 0),
        "a tank click in another world must do nothing"
    );
}

// ── Tank rows: template 413 and spawn 13802 ──────────────────────────────

/// **Template 413 is the static-256 prop and spawn 13802 is the only
/// `Castle_SymbioteTank`, 4.0 u or more from the dispenser 13801.**
#[tokio::test]
async fn live_db_1521_tank_template_and_spawn_rows() {
    let pool = require_db_or_skip!();
    let (interaction, mesh): (i64, Option<String>) = sqlx::query_as(
        "SELECT interaction_type, static_mesh FROM resources.entity_templates \
         WHERE template_id = 413",
    )
    .fetch_one(&pool)
    .await
    .expect("template 413 must be seeded");
    assert_eq!(interaction, 256, "static INT_MINIGAME_LIVEWIRE");
    assert_eq!(mesh.as_deref(), Some("CA-Props.CA-SymbioteChamber00"));

    let tanks: Vec<(i32, i32, f32, f32)> = sqlx::query_as(
        "SELECT spawn_id, template_id, x, z FROM resources.spawnlist \
         WHERE world_id = 8 AND tag = 'Castle_SymbioteTank'",
    )
    .fetch_all(&pool)
    .await
    .expect("query tank spawns");
    assert_eq!(tanks.len(), 1, "exactly one Castle_SymbioteTank: {tanks:?}");
    assert_eq!((tanks[0].0, tanks[0].1), (13802, 413));

    let (dx, dz): (f32, f32) =
        sqlx::query_as("SELECT x, z FROM resources.spawnlist WHERE spawn_id = 13801")
            .fetch_one(&pool)
            .await
            .expect("dispenser 13801 must be seeded");
    let gap = ((tanks[0].2 - dx).powi(2) + (tanks[0].3 - dz).powi(2)).sqrt();
    assert!(
        gap >= 4.0 - 1e-3,
        "tank is {gap} u from the dispenser; need >= 4.0"
    );
}
