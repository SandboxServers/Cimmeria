//! Castle defense_q1 sentry side story (mission 10310), CS-04b1c1: replay of the
//! chains `castle_side_stories_chains.sql` seeds as 10100-10106. The seed shape,
//! slot binding and sentry placement are pinned in `castle_side_defense_q1_seed`.
//!
//! The prerequisite is 10300 (the medical story) completed. The offer binds
//! the sentry (dialog-set map 1000006 at slot 419) on that completion, on
//! login and on abandon. The sentry click (10103) accepts; the nested
//! `mission_accepted` (10104) advances A to B, unbinds the sentry and shows
//! dialog 10709. Step B completes on entering `Castle.Infirmary` (10105).
//! Every click and region entry goes through the real dispatcher, so a
//! resolve-only check cannot pass a broken executor arm.
//!
//! Each test fails if its seed row is removed: without 10103 the click accepts
//! nothing, and without 10104 the step never reaches 10355.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{fire_enter_region, fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// The sentry NPC, the interaction target of the 10103 click.
const SENTRY: u32 = 3;
const CASTLE: &str = "Castle";
const SENTRY_TAG: &str = "Castle_DefenseQ1_Sentry";
const PREREQ: i32 = 10300;
const MISSION: i32 = 10310;
const STEP_A: i32 = 10354;
const STEP_B: i32 = 10355;
const OBJ_A: i32 = 10474;
const OBJ_B: i32 = 10475;
/// The sentry offer: dialog-set map 1000006 bound at slot 419.
const OFFER_SET: i32 = 1000006;
const OFFER_SLOT: i32 = 419;
const DIALOG: i32 = 10709;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the defense_q1 side story, 10100-10106 (offer, relog and
/// abandon rebinds, accept, nested advance, Infirmary completion, heal).
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

/// Castle with player `ENTITY` holding `missions` and the sentry actor, with
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
    mgr.create_entity(SENTRY, CASTLE, [3.0; 3], [0.0; 3])
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

/// A completed mission `mission_id`: the prerequisite the offer gates on.
fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10310 at `step` with `objectives` active, as the accept would leave it.
fn active_mission(step: i32, objectives: &[i32]) -> MissionInstance {
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

/// Click the sentry; returns whether a chain handled the click (false falls
/// through to the default dialog path) and everything sent to the
/// cell-to-base channel.
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

// ── The accept ───────────────────────────────────────────────────────────

/// **The sentry click accepts, advances A to B, unbinds and shows 10709, in
/// that order.** 10300 done and 10310 not_active: the login binds the offer,
/// the click runs 10103 (accept), whose `mission_accepted` runs 10104. The
/// traffic must show the accept, then the step-B update, then the dialog;
/// the state must show B active, the offer unbound and 10709 offered. A
/// resolve-only check would not see the executor arms.
/// Revert proof: delete 10104's nested `advance_step` and the step-B update
/// never arrives, so the order assertion fails.
#[tokio::test]
async fn live_db_castle_side_defense_q1_accept_advances_and_shows_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed(PREREQ)]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, OFFER_SLOT),
        vec![OFFER_SET],
        "the offer must be bound before the click"
    );

    let (handled, out) = click_sentry(&engine, &mut mgr).await;

    assert!(handled, "chain 10103 must handle the click");
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
        accept.expect("the accept must persist 10310 at step A"),
        advance.expect("the nested advance must persist step B"),
        dialog.expect("the nested chain must send onDialogDisplay for dialog 10709"),
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
        "the sentry offer must be unbound after accept; got {:?}",
        bound_at(&mgr, OFFER_SLOT)
    );
    assert_eq!(offered(&mgr), vec![DIALOG], "dialog 10709 must be shown");
}

/// **A repeat click after accept fires nothing.** The mission is already
/// active at B, so 10103's `not_active` gate fails: no chain handles the
/// click, and there is no accept, step change, dialog or mission traffic.
/// Revert proof: drop 10103's `mission_status 10310` condition and the click
/// is handled by the chain again.
#[tokio::test]
async fn live_db_castle_side_defense_q1_repeat_click_noop() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed(PREREQ)]).await;
    log_in(&engine, &mut mgr).await;
    let (first, _) = click_sentry(&engine, &mut mgr).await;
    assert!(first, "the first click must accept");
    assert_eq!(step_of(&mgr), Some(STEP_B), "the accept must reach B");

    let (handled, out) = click_sentry(&engine, &mut mgr).await;

    assert!(
        !handled,
        "a repeat click must fall through, not run a chain"
    );
    assert!(
        !has_update(&out, None),
        "a repeat click must send no mission traffic; got {out:?}"
    );
    assert_eq!(step_of(&mgr), Some(STEP_B), "the step must not move");
    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_ACTIVE,
        "the mission must stay active"
    );
}

// ── The Infirmary completion ─────────────────────────────────────────────

/// **Entering `Castle.Infirmary` completes 10310 only at step B.** At B the
/// region completes objective 10475 and the mission. At A, not_active or
/// completed the chain's step or mission gate fails and nothing changes.
/// Revert proof: drop 10105's `step_status 10355` condition and the entry at
/// A completes the mission.
#[tokio::test]
async fn live_db_castle_side_defense_q1_infirmary_completes_only_at_b() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut at_b = staged(
        &pool,
        vec![completed(PREREQ), active_mission(STEP_B, &[OBJ_B])],
    )
    .await;
    let out = enter(&engine, &mut at_b, "Castle.Infirmary").await;
    assert!(
        has_update(&out, None),
        "entering the Infirmary at B must send mission traffic"
    );
    assert_eq!(
        mission_of(&at_b).unwrap().status,
        MISSION_COMPLETED,
        "the Infirmary entry at B must complete 10310"
    );

    let mut at_a = staged(
        &pool,
        // Both objectives are live so only 10105's step condition stops the
        // entry from completing 10475 at A.
        vec![completed(PREREQ), active_mission(STEP_A, &[OBJ_A, OBJ_B])],
    )
    .await;
    let out = enter(&engine, &mut at_a, "Castle.Infirmary").await;
    assert!(!has_update(&out, None), "at A the entry must send nothing");
    assert_eq!(step_of(&at_a), Some(STEP_A), "at A the step must not move");
    assert_eq!(
        mission_of(&at_a).unwrap().status,
        MISSION_ACTIVE,
        "at A the mission must stay active"
    );

    let mut not_active = staged(&pool, vec![completed(PREREQ)]).await;
    let out = enter(&engine, &mut not_active, "Castle.Infirmary").await;
    assert!(!has_update(&out, None), "not_active must send nothing");
    assert!(
        mission_of(&not_active).is_none(),
        "not_active must stay absent"
    );

    let mut done = staged(&pool, vec![completed(PREREQ), completed(MISSION)]).await;
    let out = enter(&engine, &mut done, "Castle.Infirmary").await;
    assert!(
        !has_update(&out, None),
        "a completed mission must send nothing"
    );
    assert_eq!(
        mission_of(&done).unwrap().status,
        MISSION_COMPLETED,
        "a completed mission must stay completed"
    );
}
