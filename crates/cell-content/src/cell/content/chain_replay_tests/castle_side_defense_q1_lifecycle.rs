//! Castle defense_q1 sentry side story (mission 10310), CS-04b1c2: the relog,
//! abandon, refusal, forged-dialog, sibling-disjointness and Q1-completion
//! replays on top of c1's chains 10100-10106. The loading idiom and the
//! helpers follow `castle_side_defense_q1.rs`; the Q1 shapes follow
//! `castle_side_medical_q1_lifecycle.rs` and `castle_side_medical_q1_refusals.rs`.
//!
//! This file holds the shared harness, the relog and the abandon tests. The
//! refusals and the forged dialog are in `refusals`; the sibling and Q1
//! hand-off checks are in `siblings`. The out-of-range click needs the real
//! `interact` method of `cimmeria-cell-methods` (`castle_defense_q1_range_tests`
//! there), and the persistence round trip is in `cimmeria-services`
//! (`castle_side_defense_q1_round_trip_tests`).
//!
//! Every login, click, region entry, abandon and dialog event goes through the
//! real dispatcher, so a resolve-only check cannot pass a broken executor arm.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_enter_region, fire_interact_tag, fire_player_loaded};
use crate::cell::content::fire_mission_abandoned;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions::abandon_mission;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

mod refusals;
mod siblings;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// The sentry NPC, the interaction target of the 10103 click.
const SENTRY: u32 = 3;
const CASTLE: &str = "Castle";
const COMBAT_SIM: &str = "CombatSim";
const SENTRY_TAG: &str = "Castle_DefenseQ1_Sentry";
const COURTYARD: &str = "Castle.CastleFrontCourtyard";
const INFIRMARY: &str = "Castle.Infirmary";
/// Q1 (10300), the prerequisite, and its steps and objectives.
const PREREQ: i32 = 10300;
const Q1_STEP_A: i32 = 10340;
const Q1_STEP_B: i32 = 10341;
const Q1_OBJ_A: i32 = 10460;
const Q1_OBJ_B: i32 = 10461;
/// 10310, the sentry story, and its steps and objectives.
const MISSION: i32 = 10310;
const STEP_A: i32 = 10354;
const STEP_B: i32 = 10355;
const OBJ_A: i32 = 10474;
const OBJ_B: i32 = 10475;
/// The sentry offer: dialog-set map 1000006 bound at slot 419.
const OFFER_SET: i32 = 1000006;
const OFFER_SLOT: i32 = 419;
const DIALOG: i32 = 10709;
/// The click chain (accept) and the Infirmary completion chain.
const ACCEPT_CHAIN: i64 = 10103;
const INFIRMARY_CHAIN: i64 = 10105;
/// Chains 10100-10106, the defense_q1 side story.
const ALL_CHAINS: [i32; 7] = [10100, 10101, 10102, 10103, 10104, 10105, 10106];

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

/// Player `ENTITY` in `world` holding `missions`, with the sentry actor and
/// the dialog-set and monologue caches loaded from the seed.
async fn staged(pool: &PgPool, world: &str, missions: Vec<MissionInstance>) -> SpaceManager {
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
    mgr.create_entity(SENTRY, world, [3.0; 3], [0.0; 3])
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

/// Where a mission stands for the player.
#[derive(Clone, Copy, Debug)]
enum Stage {
    Absent,
    AtA,
    AtB,
    Done,
}

fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// `mission_id` at `step` with `objectives` active, as the live state holds it.
fn active(mission_id: i32, step: i32, objectives: &[i32]) -> MissionInstance {
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

/// Q1 (10300) in `stage`; `None` when the player does not hold it.
fn q1_at(stage: Stage) -> Option<MissionInstance> {
    match stage {
        Stage::Absent => None,
        Stage::AtA => Some(active(PREREQ, Q1_STEP_A, &[Q1_OBJ_A])),
        Stage::AtB => Some(active(PREREQ, Q1_STEP_B, &[Q1_OBJ_B])),
        Stage::Done => Some(completed(PREREQ)),
    }
}

/// 10310 in `stage`; `None` when the player does not hold it.
fn sentry_at(stage: Stage) -> Option<MissionInstance> {
    match stage {
        Stage::Absent => None,
        Stage::AtA => Some(active(MISSION, STEP_A, &[OBJ_A])),
        Stage::AtB => Some(active(MISSION, STEP_B, &[OBJ_B])),
        Stage::Done => Some(completed(MISSION)),
    }
}

/// Q1 completed 704 (the Q1 offer's prerequisite), then Q1 and 10310 as given.
fn stages(q1: Stage, sentry: Stage) -> Vec<MissionInstance> {
    let mut missions = vec![completed(704)];
    missions.extend(q1_at(q1));
    missions.extend(sentry_at(sentry));
    missions
}

fn mission_of(mgr: &SpaceManager, mission_id: i32) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(mission_id)
}

fn status_of(mgr: &SpaceManager, mission_id: i32) -> Option<i8> {
    mission_of(mgr, mission_id).map(|m| m.status)
}

fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr, MISSION).and_then(|m| m.current_step_id)
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

/// The state a refusal or a forged event must leave untouched.
fn snapshot(mgr: &SpaceManager) -> (Option<i32>, Option<i8>, Vec<i32>, Vec<i32>) {
    (
        step_of(mgr),
        status_of(mgr, MISSION),
        bound_at(mgr, OFFER_SLOT),
        offered(mgr),
    )
}

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// Click the sentry; returns whether a chain handled the click and everything
/// sent to the cell-to-base channel.
async fn click_sentry(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, SENTRY_TAG, SENTRY, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Enter `region`; returns everything sent to the cell-to-base channel.
async fn enter(engine: &ChainEngine, mgr: &mut SpaceManager, region: &str) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, region, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// Whether `out` holds a mission update for 10310 at `step` (`None`: any).
fn has_update(out: &[CellToBaseMsg], step: Option<i32>) -> bool {
    out.iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate { mission_id: MISSION, current_step_id, .. }
                if step.is_none() || *current_step_id == step
        )
    })
}

/// Whether `out` holds any mission or dialog traffic.
fn has_mission_or_dialog(out: &[CellToBaseMsg]) -> bool {
    out.iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )
    })
}

// ── Relog ────────────────────────────────────────────────────────────────

/// **Each login has one owner.** With Q1 done and 10310 not_active the login
/// rebinds the sentry (10101). At A the login heals to B (10106) and binds
/// nothing, because the offer gate needs not_active. At B nothing moves.
/// Completed binds nothing, and a login with Q1 still running binds nothing.
/// Revert proof: remove 10106 and the A login stays at A.
#[tokio::test]
async fn live_db_castle_side_defense_q1_relog_each_step() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut not_active = staged(&pool, CASTLE, stages(Stage::Done, Stage::Absent)).await;
    log_in(&engine, &mut not_active).await;
    assert_eq!(
        bound_at(&not_active, OFFER_SLOT),
        vec![OFFER_SET],
        "a not_active login with Q1 done must rebind the sentry (10101)"
    );
    assert!(
        mission_of(&not_active, MISSION).is_none(),
        "a not_active login must not create 10310"
    );

    let mut q1_running = staged(&pool, CASTLE, stages(Stage::AtB, Stage::Absent)).await;
    log_in(&engine, &mut q1_running).await;
    assert!(
        bound_at(&q1_running, OFFER_SLOT).is_empty(),
        "a login with Q1 still running must bind nothing"
    );

    let mut at_a = staged(&pool, CASTLE, stages(Stage::Done, Stage::AtA)).await;
    log_in(&engine, &mut at_a).await;
    assert_eq!(
        step_of(&at_a),
        Some(STEP_B),
        "a login at A must heal to B (10106)"
    );
    assert_eq!(
        status_of(&at_a, MISSION),
        Some(MISSION_ACTIVE),
        "the heal must leave 10310 active"
    );
    assert!(
        bound_at(&at_a, OFFER_SLOT).is_empty(),
        "a login at A must not bind the sentry"
    );
    assert!(offered(&at_a).is_empty(), "the heal must show no dialog");

    let mut at_b = staged(&pool, CASTLE, stages(Stage::Done, Stage::AtB)).await;
    let before = snapshot(&at_b);
    log_in(&engine, &mut at_b).await;
    assert_eq!(snapshot(&at_b), before, "a login at B must change nothing");

    let mut completed_run = staged(&pool, CASTLE, stages(Stage::Done, Stage::Done)).await;
    log_in(&engine, &mut completed_run).await;
    assert!(
        bound_at(&completed_run, OFFER_SLOT).is_empty(),
        "a login after completion must bind nothing"
    );
    assert_eq!(
        status_of(&completed_run, MISSION),
        Some(MISSION_COMPLETED),
        "a login after completion must leave 10310 completed"
    );
}

// ── Abandon ──────────────────────────────────────────────────────────────

/// **An abandon at A or at B rebinds the sentry.** The real abandon removes
/// 10310 first, then `mission_abandoned` fires 10102, which binds the offer
/// again at slot 419. Revert proof: remove 10102 and the offer stays unbound.
#[tokio::test]
async fn live_db_castle_side_defense_q1_abandon_rebinds() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    for (label, stage) in [("at A", Stage::AtA), ("at B", Stage::AtB)] {
        let mut mgr = staged(&pool, CASTLE, stages(Stage::Done, stage)).await;
        let (tx, _rx) = mpsc::channel(256);
        assert!(
            abandon_mission(ENTITY, MISSION, &tx, &mut mgr).await,
            "{label}: the abandon must remove 10310"
        );
        fire_mission_abandoned(ENTITY, PLAYER, MISSION, &engine, &tx, &mut mgr).await;
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "{label}: 10310 must be gone after the abandon"
        );
        assert_eq!(
            bound_at(&mgr, OFFER_SLOT),
            vec![OFFER_SET],
            "{label}: mission_abandoned must rebind the sentry (10102)"
        );
    }
}
