//! Castle return_q2 side story (mission 10321), CS-04c2c1: the chains
//! `castle_side_stories_chains.sql` seeds as 10140-10146.
//!
//! Once Marsh's return report (10320) completes, the Moh'katan bind (1000014
//! at slot 54) offers the mission. The Moh'katan click (10142) accepts; the
//! nested `mission_accepted` (10143) advances A to B, unbinds and shows
//! dialog 10725. Dialling Harset at B (10144) completes it. Every click and
//! dial goes through the real dispatcher, so a resolve-only check cannot
//! pass a broken executor arm.
//!
//! Each test fails if its seed row is removed: without 10142 the click
//! accepts nothing, and without 10143 the step never reaches 10379. The seed
//! shape of the seven chains is in `castle_side_return_q2_chains.rs`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::fire_mission_completed;
use super::super::{fire_interact_tag, fire_player_loaded, fire_stargate_dialed};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// The Moh'katan NPC, the interaction target of the 10142 click.
const MOHKATAN: u32 = 4;
const CASTLE: &str = "Castle";
const MOHKATAN_TAG: &str = "Castle_Mohkatan";
const HARSET: &str = "Harset";
const MISSION: i32 = 10321;
const PREREQ: i32 = 10320;
const STEP_A: i32 = 10378;
const STEP_B: i32 = 10379;
const OBJ_A: i32 = 10498;
const OBJ_B: i32 = 10499;
/// The Moh'katan offer: dialog-set map 1000014 bound at slot 54.
const OFFER_SET: i32 = 1000014;
const OFFER_SLOT: i32 = 54;
const DIALOG: i32 = 10725;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the return_q2 side story, 10140-10146 (offer, relog and
/// abandon rebinds, accept, nested advance, dial completion, heal).
const ALL_CHAINS: [i32; 7] = [10140, 10141, 10142, 10143, 10144, 10145, 10146];

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

/// Castle with player `ENTITY` holding `missions` and the Moh'katan actor,
/// with the dialog-set and monologue caches loaded from the seed.
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
    mgr.create_entity(MOHKATAN, CASTLE, [3.0; 3], [0.0; 3])
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

/// 10320 (the Marsh return report) completed: the prerequisite of the offer.
fn completed_prereq() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10321 at `step` with `objectives` active, as the accept would leave it.
fn mohkatan_mission(step: i32, objectives: &[i32]) -> MissionInstance {
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

/// 10321 already completed (no active step).
fn completed_mission() -> MissionInstance {
    let mut mission = MissionInstance::new(MISSION, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

fn mission_of(mgr: &SpaceManager, mission_id: i32) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(mission_id)
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

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// Click Moh'katan; returns whether a chain handled the click (false falls
/// through to the default dialog path) and everything sent to the
/// cell-to-base channel.
async fn click_mohkatan(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, MOHKATAN_TAG, MOHKATAN, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Dial `destination` at the DHD; returns everything sent to the channel.
async fn dial(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
    destination: &str,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_stargate_dialed(ENTITY, PLAYER, destination, engine, &tx, mgr).await;
    drain(&mut rx)
}

// ── The offer ────────────────────────────────────────────────────────────

/// **Completing 10320 binds the Moh'katan offer only while 10321 is
/// not_active; the login rebinds it under the same guard.** `mission_completed
/// 10320` (10140) and the login (10141) bind slot 54 when 10321 is
/// not_active; completing another mission binds nothing; with 10321 active
/// neither event binds. Revert proof: drop 10140's `mission_status 10321`
/// guard and the active completion case binds the offer; drop 10141's and the
/// active login case does.
#[tokio::test]
async fn live_db_castle_side_return_q2_offer_on_q1_completion() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let (tx, _rx) = mpsc::channel(256);

    let mut completion = staged(&pool, vec![completed_prereq()]).await;
    fire_mission_completed(ENTITY, PLAYER, PREREQ, &engine, &tx, &mut completion).await;
    assert_eq!(
        bound_at(&completion, OFFER_SLOT),
        vec![OFFER_SET],
        "completing 10320 with 10321 not_active must bind the offer (10140)"
    );

    let mut other = staged(&pool, vec![completed_prereq()]).await;
    fire_mission_completed(ENTITY, PLAYER, 1520, &engine, &tx, &mut other).await;
    assert!(
        bound_at(&other, OFFER_SLOT).is_empty(),
        "completing a mission other than 10320 must bind nothing"
    );

    let mut running = staged(
        &pool,
        vec![completed_prereq(), mohkatan_mission(STEP_A, &[OBJ_A])],
    )
    .await;
    fire_mission_completed(ENTITY, PLAYER, PREREQ, &engine, &tx, &mut running).await;
    assert!(
        bound_at(&running, OFFER_SLOT).is_empty(),
        "completing 10320 while 10321 is active must bind nothing"
    );

    let mut fresh = staged(&pool, vec![completed_prereq()]).await;
    log_in(&engine, &mut fresh).await;
    assert_eq!(
        bound_at(&fresh, OFFER_SLOT),
        vec![OFFER_SET],
        "10320 completed with 10321 not_active must bind the offer at slot 54"
    );

    let mut active = staged(
        &pool,
        vec![completed_prereq(), mohkatan_mission(STEP_A, &[OBJ_A])],
    )
    .await;
    log_in(&engine, &mut active).await;
    assert!(
        bound_at(&active, OFFER_SLOT).is_empty(),
        "with 10321 active the login must bind nothing; got {:?}",
        bound_at(&active, OFFER_SLOT)
    );
}

// ── The accept ───────────────────────────────────────────────────────────

/// **The Moh'katan click accepts, advances A to B, unbinds and shows 10725,
/// in that order.** 10320 completed and 10321 not_active: the login binds the
/// offer, the click runs 10142 (accept), whose `mission_accepted` runs 10143.
/// The traffic must show the accept, then the step-B update, then the dialog;
/// the state must show B active, the offer unbound and 10725 offered. A
/// resolve-only check would not see the executor arms. Revert proof: swap
/// 10143's `advance_step` and `display_dialog`; the order assertion fails.
#[tokio::test]
async fn live_db_castle_side_return_q2_click_accepts_then_shows_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_prereq()]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, OFFER_SLOT),
        vec![OFFER_SET],
        "the offer must be bound before the click"
    );

    let (handled, out) = click_mohkatan(&engine, &mut mgr).await;

    assert!(handled, "chain 10142 must handle the click");
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
        accept.expect("the accept must persist 10321 at step A"),
        advance.expect("the nested advance must persist step B"),
        dialog.expect("the nested chain must send onDialogDisplay for dialog 10725"),
    );
    assert!(
        accept < advance && advance < dialog,
        "traffic order must be accept, advance to B, dialog; got {accept}, {advance}, {dialog}",
    );
    assert_eq!(step_of(&mgr), Some(STEP_B), "A must advance to B");
    assert_eq!(
        mission_of(&mgr, MISSION).unwrap().status,
        MISSION_ACTIVE,
        "the accept must leave the mission active"
    );
    assert!(
        bound_at(&mgr, OFFER_SLOT).is_empty(),
        "the Moh'katan offer must be unbound after accept; got {:?}",
        bound_at(&mgr, OFFER_SLOT)
    );
    assert_eq!(offered(&mgr), vec![DIALOG], "dialog 10725 must be shown");
}

/// **A repeat click after accept fires nothing.** The mission is already
/// active at A, so 10142's `not_active` gate fails: no chain handles the
/// click, and there is no accept, step change, dialog or mission traffic.
/// Revert proof: drop 10142's `mission_status 10321` condition and the click
/// is handled by the chain again.
#[tokio::test]
async fn live_db_castle_side_return_q2_repeat_click_noop() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(
        &pool,
        vec![completed_prereq(), mohkatan_mission(STEP_A, &[OBJ_A])],
    )
    .await;

    let (handled, out) = click_mohkatan(&engine, &mut mgr).await;

    assert!(
        !handled,
        "no chain may handle a click with 10321 active (10142 is gated on not_active)"
    );
    assert!(
        out.iter().all(|m| !matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )),
        "a repeat click must send no mission or dialog traffic; got {} messages",
        out.len()
    );
    assert_eq!(step_of(&mgr), Some(STEP_A), "the step must stay at A");
}

// ── The dial ─────────────────────────────────────────────────────────────

/// **Dialling Harset completes 10321 at B and nowhere else.** At B, the dial
/// fires 10144, whose `complete_objective 10499` credits B and completes the
/// mission. At A, with 10321 not_active, with 10321 completed, or with another
/// gate key, nothing fires and the state does not move. Revert proof: remove
/// 10144's `step_status` condition and the dial at A completes the mission;
/// change its event key and the dial at B no longer completes.
#[tokio::test]
async fn live_db_castle_side_return_q2_dial_completes_only_at_b() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    // B's objective is pending as well, so only the step gate stands between
    // the dial at A and a completion.
    let mut at_a = staged(
        &pool,
        vec![
            completed_prereq(),
            mohkatan_mission(STEP_A, &[OBJ_A, OBJ_B]),
        ],
    )
    .await;
    let out = dial(&engine, &mut at_a, HARSET).await;
    assert!(
        out.is_empty(),
        "a dial at A must fire nothing; got {} messages",
        out.len()
    );
    assert_eq!(
        step_of(&at_a),
        Some(STEP_A),
        "the dial at A must not move the step"
    );
    assert_eq!(
        mission_of(&at_a, MISSION).unwrap().status,
        MISSION_ACTIVE,
        "the dial at A must not complete 10321"
    );

    let mut not_started = staged(&pool, vec![completed_prereq()]).await;
    let out = dial(&engine, &mut not_started, HARSET).await;
    assert!(
        out.is_empty() && mission_of(&not_started, MISSION).is_none(),
        "a dial with 10321 not_active must fire nothing; got {} messages",
        out.len()
    );

    let mut done = completed_mission();
    done.completed_objectives.push(OBJ_B);
    let mut finished = staged(&pool, vec![completed_prereq(), done]).await;
    let out = dial(&engine, &mut finished, HARSET).await;
    assert!(
        out.is_empty(),
        "a dial with 10321 completed must fire nothing; got {} messages",
        out.len()
    );

    let mut other_gate = staged(
        &pool,
        vec![completed_prereq(), mohkatan_mission(STEP_B, &[OBJ_B])],
    )
    .await;
    let out = dial(&engine, &mut other_gate, "Dakara").await;
    assert!(
        out.is_empty(),
        "a dial to another gate at B must fire nothing; got {} messages",
        out.len()
    );
    assert_eq!(
        mission_of(&other_gate, MISSION).unwrap().status,
        MISSION_ACTIVE,
        "a dial to another gate must not complete 10321"
    );

    let mut at_b = staged(
        &pool,
        vec![completed_prereq(), mohkatan_mission(STEP_B, &[OBJ_B])],
    )
    .await;
    dial(&engine, &mut at_b, HARSET).await;
    let mission =
        mission_of(&at_b, MISSION).expect("10321 must stay on the record after completion");
    assert_eq!(
        mission.status, MISSION_COMPLETED,
        "dialling Harset at B must complete 10321"
    );
    assert!(
        mission.completed_objectives.contains(&OBJ_B),
        "complete_objective 10499 must credit B; got {:?}",
        mission.completed_objectives
    );
}
