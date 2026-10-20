//! Castle Q1 medical side story (mission 10300), CS-04a1c2: the lifecycle
//! after the accept. The courtyard completes the mission only at step B
//! (10005). A login heals A to B (10006) or rebinds the offer while the
//! mission is not_active (10001). An abandon at A or B rebinds the offer
//! (10002). Completing 704 offers the medic (10000). Every event goes through the real dispatcher, so the executor
//! arms run; the accept and its nested advance are pinned by
//! [`super::castle_side_medical_q1`].
//!
//! The harness is copied from that module rather than shared, so a change
//! to one file cannot move the other's assertions.

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::fire_mission_completed;
use super::super::mission_context::{populate_mission_context, populate_world_context};
use super::super::{fire_enter_region, fire_player_loaded};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::missions::abandon_mission;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const CASTLE: &str = "Castle";
/// The courtyard region whose entry completes 10300 at step B (10005).
const COURTYARD: &str = "Castle.CastleFrontCourtyard";
const MISSION: i32 = 10300;
const PREREQ: i32 = 704;
const STEP_A: i32 = 10340;
const STEP_B: i32 = 10341;
const OBJ_A: i32 = 10460;
const OBJ_B: i32 = 10461;
/// The medic offer: dialog-set map 1000001 bound at slot 177.
const OFFER_SET: i32 = 1000001;
const OFFER_SLOT: i32 = 177;
/// Every chain of the medical side story, 10000-10006.
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

/// Castle with player `ENTITY` holding `missions`, with the caches loaded
/// from the seed.
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

/// 10300 completed: the story is over, and no chain may bind or move it.
fn completed_10300() -> MissionInstance {
    let mut mission = MissionInstance::new(MISSION, 0, Vec::new());
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

/// What a lifecycle event may change: the mission status, the step, and
/// whether objective B is credited.
fn snapshot(mgr: &SpaceManager) -> (Option<i8>, Option<i32>, bool) {
    (
        mission_of(mgr).map(|m| m.status),
        step_of(mgr),
        mission_of(mgr).is_some_and(|m| m.completed_objectives.contains(&OBJ_B)),
    )
}

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// The chain ids a courtyard entry resolves to, built the way
/// `fire_enter_region` builds its context, before any action runs.
fn courtyard_matches(engine: &ChainEngine, mgr: &SpaceManager) -> Vec<i64> {
    let mut ctx = ExecutionContext::new().with_source(EntityId(ENTITY as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(COURTYARD));
    populate_world_context(ENTITY, mgr, &mut ctx);
    populate_mission_context(mgr.get_entity(ENTITY).unwrap(), &mut ctx);
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
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

async fn enter_courtyard(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, COURTYARD, engine, &tx, mgr).await;
}

// ── The courtyard ────────────────────────────────────────────────────────

/// **The courtyard completes 10300 at B and nowhere else.** At B, the entry
/// fires 10005, whose `complete_objective 10461` credits B and completes the
/// mission. At A, not_active or completed, the gate fails and the state does
/// not move. Revert proof: remove 10005's `step_status` condition and the
/// entry at A completes the mission.
#[tokio::test]
async fn live_db_castle_side_medical_q1_courtyard_completes_only_at_b() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut at_b = staged(
        &pool,
        vec![completed_704(), medic_mission(STEP_B, &[OBJ_B])],
    )
    .await;
    assert_eq!(
        courtyard_matches(&engine, &at_b),
        vec![10005],
        "at B the entry must resolve to 10005 alone"
    );
    enter_courtyard(&engine, &mut at_b).await;
    let mission = mission_of(&at_b).expect("10300 must stay on the record after completion");
    assert_eq!(
        mission.status, MISSION_COMPLETED,
        "entering the courtyard at B must complete 10300"
    );
    assert!(
        mission.completed_objectives.contains(&OBJ_B),
        "complete_objective 10461 must credit B; got {:?}",
        mission.completed_objectives
    );

    let cases: [(&str, Vec<MissionInstance>); 3] = [
        (
            "at A",
            vec![completed_704(), medic_mission(STEP_A, &[OBJ_A])],
        ),
        ("not_active", vec![completed_704()]),
        ("completed", vec![completed_704(), completed_10300()]),
    ];
    for (label, missions) in cases {
        let mut mgr = staged(&pool, missions).await;
        let before = snapshot(&mgr);
        assert!(
            !courtyard_matches(&engine, &mgr).contains(&10005),
            "entering the courtyard {label} must not resolve 10005"
        );
        enter_courtyard(&engine, &mut mgr).await;
        assert_eq!(
            snapshot(&mgr),
            before,
            "entering the courtyard {label} must change nothing"
        );
    }
}

// ── The login ────────────────────────────────────────────────────────────

/// **Each login step has one owner.** With 704 done and 10300 not_active the
/// login rebinds the offer (10001) and nothing else moves. At A the login
/// heals to B (10006) and binds nothing, because the offer gate needs
/// not_active. At B nothing fires. Completed binds nothing. Revert proof:
/// drop 10006 and the A login stays at A.
#[tokio::test]
async fn live_db_castle_side_medical_q1_relog_each_step() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut not_active = staged(&pool, vec![completed_704()]).await;
    log_in(&engine, &mut not_active).await;
    assert_eq!(
        bound_at(&not_active, OFFER_SLOT),
        vec![OFFER_SET],
        "a not_active login must rebind the offer (10001)"
    );
    assert!(
        mission_of(&not_active).is_none(),
        "a not_active login must not create 10300"
    );

    let mut at_a = staged(
        &pool,
        vec![completed_704(), medic_mission(STEP_A, &[OBJ_A])],
    )
    .await;
    log_in(&engine, &mut at_a).await;
    assert_eq!(
        step_of(&at_a),
        Some(STEP_B),
        "a login at A must heal to B (10006)"
    );
    assert_eq!(
        mission_of(&at_a).map(|m| m.status),
        Some(MISSION_ACTIVE),
        "the heal must leave 10300 active"
    );
    assert!(
        bound_at(&at_a, OFFER_SLOT).is_empty(),
        "a login at A must not bind the offer"
    );
    assert!(offered(&at_a).is_empty(), "the heal must show no dialog");

    let mut at_b = staged(
        &pool,
        vec![completed_704(), medic_mission(STEP_B, &[OBJ_B])],
    )
    .await;
    let before = snapshot(&at_b);
    log_in(&engine, &mut at_b).await;
    assert_eq!(snapshot(&at_b), before, "a login at B must change nothing");
    assert!(
        bound_at(&at_b, OFFER_SLOT).is_empty(),
        "a login at B binds nothing"
    );

    let mut completed = staged(&pool, vec![completed_704(), completed_10300()]).await;
    log_in(&engine, &mut completed).await;
    assert!(
        bound_at(&completed, OFFER_SLOT).is_empty(),
        "a login after completion must bind nothing"
    );
    assert_eq!(
        mission_of(&completed).map(|m| m.status),
        Some(MISSION_COMPLETED),
        "a login after completion must leave 10300 completed"
    );
}

// ── The abandon ──────────────────────────────────────────────────────────

/// **An abandon at A or at B rebinds the offer.** The real abandon removes
/// 10300 first, then `mission_abandoned` fires 10002, which binds the offer
/// again at slot 177. Revert proof: drop 10002 and the offer stays unbound.
#[tokio::test]
async fn live_db_castle_side_medical_q1_abandon_rebinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for (label, step, objective) in [("at A", STEP_A, OBJ_A), ("at B", STEP_B, OBJ_B)] {
        let mut mgr = staged(
            &pool,
            vec![completed_704(), medic_mission(step, &[objective])],
        )
        .await;
        let (tx, _rx) = mpsc::channel(256);
        assert!(
            abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
            "{label}: the abandon must remove 10300"
        );
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert!(
            mission_of(&mgr).is_none(),
            "{label}: 10300 must be gone after the abandon"
        );
        assert_eq!(
            bound_at(&mgr, OFFER_SLOT),
            vec![OFFER_SET],
            "{label}: mission_abandoned must rebind the offer (10002)"
        );
    }
}

// ── The first offer ──────────────────────────────────────────────────────

/// **Completing 704 offers the medic (10000), and only then.** Review finding:
/// the brief's tests never fired `mission_completed 704`, so 10000's own
/// binding had no test. Completing 704 with 10300 not_active binds the offer;
/// completing another mission, or 704 while 10300 is already running, binds
/// nothing. Revert proof: change 10000's trigger key and the first case stays
/// unbound.
#[tokio::test]
async fn live_db_castle_side_medical_q1_704_completion_offers_the_medic() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut offer = staged(&pool, vec![completed_704()]).await;
    let (tx, _rx) = mpsc::channel(256);
    fire_mission_completed(ENTITY, PLAYER, PREREQ, &engine, &tx, &mut offer).await;
    assert_eq!(
        bound_at(&offer, OFFER_SLOT),
        vec![OFFER_SET],
        "completing 704 must bind the offer (10000)"
    );

    let mut other = staged(&pool, vec![completed_704()]).await;
    fire_mission_completed(ENTITY, PLAYER, 1520, &engine, &tx, &mut other).await;
    assert!(
        bound_at(&other, OFFER_SLOT).is_empty(),
        "completing a mission other than 704 must bind nothing"
    );

    let mut running = staged(
        &pool,
        vec![completed_704(), medic_mission(STEP_A, &[OBJ_A])],
    )
    .await;
    fire_mission_completed(ENTITY, PLAYER, PREREQ, &engine, &tx, &mut running).await;
    assert!(
        bound_at(&running, OFFER_SLOT).is_empty(),
        "completing 704 while 10300 is running must bind nothing"
    );
}
