//! Mission 1520 (Castle Infirmary) entry and relog chains, CS-02c1:
//! `castle_1520_chains.sql` chains 1367-1368 (the region offer and the
//! objective credit), 1380 (the Ogilvie bind on accept), 1381-1383 (the
//! relog restores).
//!
//! The entry tests go through the real `fire_enter_region` dispatcher, so
//! the executor runs the accept and its `enter_region` replay for real. A
//! resolve-only test cannot see the replay. The relog tests go through the
//! real `fire_player_loaded`. A bind is a per-player `add_dialog_set` that
//! lands in `entity.available_interactions[slot]` (the template id), so the
//! tests read the bind back from there, with the `dialog_set_maps` cache
//! loaded from the seed.
//!
//! Each test names the seed row it pins. Removing that row makes the chain
//! load fail, or the assertion fail.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_enter_region, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{RegionData, SpaceManager, REGION_FLAG_CLIENT_HINTED};
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_step_objectives,
    load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const CASTLE: &str = "Castle";
const INFIRMARY: &str = "Castle.Infirmary";
const MISSION: i32 = 1520;
const STEP_OFFER: i32 = 4452;
const STEP_DISPENSER: i32 = 4453;
const STEP_DONE: i32 = 4454;
const OBJ_CREDIT: i32 = 5171;
const OBJ_HIDDEN: i32 = 5172;

// ── Harness ──────────────────────────────────────────────────────────────

/// Register the named chains, panicking if any is missing from the seed.
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

/// A character in `world` with the given archetype (`None` is an entity
/// whose archetype was never stamped) and missions already on the record.
async fn staged(
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
    // Without this cache `add_dialog_set` is a logged miss and binds nothing.
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
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
fn mission_at(id: i32, step: Option<i32>, objectives: &[i32]) -> MissionInstance {
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
fn completed_704() -> MissionInstance {
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

fn mission_of(mgr: &SpaceManager, id: i32) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY).unwrap().missions.get_mission(id)
}

/// Log in through the real `fire_player_loaded` and drain what it sent.
async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
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

/// Every slot that holds a bind, so a test can say "nothing else is bound".
fn all_binds(mgr: &SpaceManager) -> Vec<(i32, Vec<i32>)> {
    let mut slots: Vec<i32> = mgr
        .get_entity(ENTITY)
        .unwrap()
        .available_interactions
        .keys()
        .copied()
        .collect();
    slots.sort_unstable();
    slots.into_iter().map(|s| (s, bound_at(mgr, s))).collect()
}

// ── Entry offer: chain 1367 ──────────────────────────────────────────────

/// **Offer, archetypes 1, 2, 3, 4 and 6.** With 704 completed and 1520 never
/// taken, entering the Infirmary accepts 1520 at its first step. Archetype 5
/// is the one excluded archetype inside the 1-6 band.
#[tokio::test]
async fn live_db_1520_enter_region_offers_mission_to_archetypes_1_2_3_4_6() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1367, 1368, 1380]).await;
    for archetype in [1, 2, 3, 4, 6] {
        let mut mgr = staged(&pool, CASTLE, Some(archetype), vec![completed_704()]).await;
        enter(&engine, &mut mgr).await;
        let mission = mission_of(&mgr, MISSION).unwrap_or_else(|| {
            panic!("archetype {archetype} must be offered mission 1520 on entry")
        });
        assert_eq!(mission.status, MISSION_ACTIVE, "archetype {archetype}");
        assert_eq!(
            mission.current_step_id,
            Some(STEP_OFFER),
            "archetype {archetype}: an offer starts at 4452 and must not advance"
        );
    }
}

/// **No offer, archetypes 5, 7 and 8, and an unpopulated archetype.** The
/// 1-6 band with `neq 5` is the gate; 7 and 8 sit outside it, and an
/// entity with no archetype fails the `gte 1` condition.
#[tokio::test]
async fn live_db_1520_enter_region_refuses_archetypes_5_7_8_and_unpopulated() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1367, 1368, 1380]).await;
    for archetype in [Some(5), Some(7), Some(8), None] {
        let mut mgr = staged(&pool, CASTLE, archetype, vec![completed_704()]).await;
        enter(&engine, &mut mgr).await;
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "archetype {archetype:?} must not be offered mission 1520"
        );
    }
}

/// **No offer, the wrong world, a missing or unfinished 704, or 1520 already
/// taken.** Each case must leave the record exactly as it was. The 1520
/// active case also shows that 1367 is disjoint from 1368: a re-entry does
/// not re-accept the mission.
#[tokio::test]
async fn live_db_1520_enter_region_refuses_wrong_world_and_704_gates() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1367, 1368, 1380]).await;

    let mut no_704 = staged(&pool, CASTLE, Some(1), vec![]).await;
    enter(&engine, &mut no_704).await;
    assert!(
        mission_of(&no_704, MISSION).is_none(),
        "704 never taken: 1520 must not be offered"
    );

    let mut active_704 = staged(&pool, CASTLE, Some(1), vec![mission_at(704, Some(0), &[])]).await;
    enter(&engine, &mut active_704).await;
    assert!(
        mission_of(&active_704, MISSION).is_none(),
        "704 active, not completed: 1520 must not be offered"
    );

    let mut wrong_world = staged(&pool, "Castle_CellBlock", Some(1), vec![completed_704()]).await;
    enter(&engine, &mut wrong_world).await;
    assert!(
        mission_of(&wrong_world, MISSION).is_none(),
        "the Infirmary region in another world must not offer 1520"
    );

    let mut completed_1520 = staged(
        &pool,
        CASTLE,
        Some(1),
        vec![completed_704(), {
            let mut m = mission_at(MISSION, None, &[]);
            m.status = MISSION_COMPLETED;
            m
        }],
    )
    .await;
    enter(&engine, &mut completed_1520).await;
    assert_eq!(
        mission_of(&completed_1520, MISSION).map(|m| m.status),
        Some(MISSION_COMPLETED),
        "1520 completed: re-entry must not re-offer it"
    );
}

// ── Credit and replay: chain 1368 ────────────────────────────────────────

/// **Objective 5171 is credited once, and entry never advances.** Re-entry
/// after the credit must not duplicate it. Point 10 of the worknote: 4452
/// holds two required objectives, so 5171 alone does not finish the mission.
#[tokio::test]
async fn live_db_1520_reentry_credits_5171_once_and_stays_on_4452() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1367, 1368, 1380]).await;
    let mut mgr = staged(
        &pool,
        CASTLE,
        Some(1),
        vec![
            completed_704(),
            mission_at(MISSION, Some(STEP_OFFER), &[OBJ_CREDIT, OBJ_HIDDEN]),
        ],
    )
    .await;

    enter(&engine, &mut mgr).await;
    enter(&engine, &mut mgr).await;

    let mission = mission_of(&mgr, MISSION).expect("1520 stays on the record");
    assert_eq!(
        mission
            .completed_objectives
            .iter()
            .filter(|&&o| o == OBJ_CREDIT)
            .count(),
        1,
        "5171 must be credited exactly once across two entries; got {:?}",
        mission.completed_objectives,
    );
    assert_eq!(
        mission.status, MISSION_ACTIVE,
        "entry must not complete 1520"
    );
    assert_eq!(
        mission.current_step_id,
        Some(STEP_OFFER),
        "entry must not advance past 4452"
    );
}

/// **Accepting through the real dispatcher replays the Infirmary entry.**
/// The accept runs 1367, and the replayed `enter_region` credits 5171 on the
/// new record. The mission stays on 4452. Chain 1380 fires on the accept and
/// binds Ogilvie: dialog set 5827 at slot 179, and nothing else. An
/// archetype-7 control is refused by 1367, so no mission, no bind.
#[tokio::test]
async fn live_db_1520_accept_replays_entry_credits_5171_and_binds_ogilvie() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1367, 1368, 1380]).await;
    let mut mgr = staged(&pool, CASTLE, Some(2), vec![completed_704()]).await;

    enter(&engine, &mut mgr).await;

    let mission = mission_of(&mgr, MISSION).expect("1520 must be accepted on entry");
    assert_eq!(mission.current_step_id, Some(STEP_OFFER));
    assert!(
        mission.completed_objectives.contains(&OBJ_CREDIT),
        "the replayed entry must credit 5171; got {:?}",
        mission.completed_objectives,
    );
    assert_eq!(
        all_binds(&mgr),
        vec![(179, vec![5827])],
        "the accept must fire 1380 (Ogilvie 5827 at slot 179) and bind nothing else"
    );

    let mut control = staged(&pool, CASTLE, Some(7), vec![completed_704()]).await;
    enter(&engine, &mut control).await;
    assert!(
        all_binds(&control).is_empty(),
        "an archetype-7 player is not offered 1520 and must get no bind; got {:?}",
        all_binds(&control)
    );
}

// ── Relog restores: chains 1381-1383 ─────────────────────────────────────

/// **Relog restores each step's state, once.** The login goes through the
/// real `fire_player_loaded`. At 4452 it rebinds Ogilvie (1381: 5827 at slot
/// 179). At 4453 it rebinds the dispenser (1382: 5828 at slot 412). Each
/// login binds exactly its own slot, so the gates are disjoint. At 4454 the
/// radio call cannot be shown again, so the login completes 1520 (1383) and
/// binds nothing.
#[tokio::test]
async fn live_db_1520_relog_restores_bind_at_4452_and_4453_and_completes_at_4454() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1381, 1382, 1383]).await;

    for (step, expected) in [
        (STEP_OFFER, vec![(179, vec![5827])]),
        (STEP_DISPENSER, vec![(412, vec![5828])]),
    ] {
        let mut mgr = staged(
            &pool,
            CASTLE,
            Some(1),
            vec![completed_704(), mission_at(MISSION, Some(step), &[])],
        )
        .await;
        log_in(&engine, &mut mgr).await;
        assert_eq!(all_binds(&mgr), expected, "login at step {step}");
        assert_eq!(
            mission_of(&mgr, MISSION).map(|m| m.status),
            Some(MISSION_ACTIVE),
            "a login at step {step} must leave 1520 active"
        );
    }

    let mut done = staged(
        &pool,
        CASTLE,
        Some(1),
        vec![completed_704(), mission_at(MISSION, Some(STEP_DONE), &[])],
    )
    .await;
    log_in(&engine, &mut done).await;
    assert_eq!(
        mission_of(&done, MISSION).map(|m| m.status),
        Some(MISSION_COMPLETED),
        "1383 must complete 1520 on a login at 4454"
    );
    assert!(
        all_binds(&done).is_empty(),
        "a login at 4454 binds nothing; got {:?}",
        all_binds(&done)
    );
}

// ── Seed rows: template 412 and spawn 13801 ──────────────────────────────

/// **The dispenser is invisible to clicks until a per-player bind.** Template
/// 412 carries `interaction_type 0` and dynamic properties, and exactly one
/// `Castle_MedDispenser` stands in world 8: spawn 13801 on template 412. A
/// static `interaction_type` would let every player click it on every step.
#[tokio::test]
async fn live_db_1520_dispenser_template_and_spawn_rows() {
    let pool = require_db_or_skip!();
    let (interaction, dynamic, name): (i64, bool, String) = sqlx::query_as(
        "SELECT interaction_type, has_dynamic_properties, template_name \
         FROM resources.entity_templates WHERE template_id = 412",
    )
    .fetch_one(&pool)
    .await
    .expect("template 412 must be seeded");
    assert_eq!(
        interaction, 0,
        "no static interaction: binds carry it per player"
    );
    assert!(dynamic, "the per-player bind needs dynamic properties");
    assert_eq!(name, "Castle_MedDispenser");

    let spawns: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT spawn_id, template_id FROM resources.spawnlist \
         WHERE world_id = 8 AND tag = 'Castle_MedDispenser'",
    )
    .fetch_all(&pool)
    .await
    .expect("query dispenser spawns");
    assert_eq!(
        spawns,
        vec![(13801, 412)],
        "exactly one Castle_MedDispenser"
    );
}
