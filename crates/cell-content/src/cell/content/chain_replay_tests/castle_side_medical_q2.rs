//! Castle medical side story Q2 (mission 10301), CS-04a2c1: the chains
//! `castle_side_stories_chains.sql` seeds as 10010-10017.
//!
//! The offer binds the prisoner (1000002 at slot 417) once the Q1 medical
//! story (10300) is completed. The prisoner click (10013) accepts; the nested
//! `mission_accepted` (10014) advances A to B, swaps the offer for the medic's
//! turn-in (1000003 at slot 177) and shows dialog 10701. The medic click at B
//! (10015) shows 10702 and completes the mission. Every click goes through the
//! real dispatcher (`fire_interact_tag`, `fire_player_loaded`), so a
//! resolve-only check cannot pass a broken executor arm. The seeded shape of
//! the eight chains is pinned in `castle_side_medical_q2_chains.rs`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
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

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// The prisoner NPC, the interaction target of the 10013 click.
const PRISONER: u32 = 2;
/// The medic NPC, the interaction target of the 10015 click.
const MEDIC: u32 = 3;
const CASTLE: &str = "Castle";
const PRISONER_TAG: &str = "Castle_Pop_Prisoner5";
const MEDIC_TAG: &str = "Castle_Pop_Infirmary_Medic1";
const MISSION: i32 = 10301;
/// The Q1 medical story, the prerequisite the offer gates on.
const Q1: i32 = 10300;
const STEP_A: i32 = 10342;
const STEP_B: i32 = 10343;
const OBJ_A: i32 = 10462;
const OBJ_B: i32 = 10463;
/// Q1's step B and its objective, used for the "10300 active" refusal.
const Q1_STEP_B: i32 = 10341;
const Q1_OBJ_B: i32 = 10461;
/// The prisoner offer: dialog-set map 1000002 bound at slot 417.
const OFFER_SET: i32 = 1000002;
const OFFER_SLOT: i32 = 417;
/// The medic turn-in: dialog-set map 1000003 bound at slot 177.
const TURN_IN_SET: i32 = 1000003;
const TURN_IN_SLOT: i32 = 177;
const ACCEPT_DIALOG: i32 = 10701;
const TURN_IN_DIALOG: i32 = 10702;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the medical side story, 10010-10017.
const ALL_CHAINS: [i32; 8] = [10010, 10011, 10012, 10013, 10014, 10015, 10016, 10017];

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

/// Castle with player `ENTITY` holding `missions`, the prisoner and the medic
/// actors, with the dialog-set and monologue caches loaded from the seed.
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
    mgr.create_entity(PRISONER, CASTLE, [3.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(MEDIC, CASTLE, [3.0; 3], [0.0; 3])
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

/// A mission that is completed: the state a finished story leaves behind.
fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// A mission active at `step` with `objectives` active, as an accept or an
/// advance would leave it.
fn active_at(mission_id: i32, step: i32, objectives: &[i32]) -> MissionInstance {
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

/// Click the NPC `target` with `tag`; returns whether a chain handled the
/// click (false falls through to the default dialog path) and everything sent
/// to the cell-to-base channel.
async fn click(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
    tag: &str,
    target: u32,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, tag, target, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Position of the MissionUpdate for `mission` at `step` in `out`.
fn update_at(out: &[CellToBaseMsg], mission: i32, step: i32) -> Option<usize> {
    out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate { mission_id, current_step_id: Some(current), .. }
                if *mission_id == mission && *current == step
        )
    })
}

/// Position of the onDialogDisplay call for `dialog` in `out`.
fn dialog_at(out: &[CellToBaseMsg], dialog: i32) -> Option<usize> {
    out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
                if *entity_id == ENTITY
                    && *method_index == ON_DIALOG_DISPLAY
                    && args.len() >= 8
                    && i32::from_le_bytes(args[4..8].try_into().unwrap()) == dialog
        )
    })
}

/// True when `out` holds any mission or dialog traffic.
fn has_game_traffic(out: &[CellToBaseMsg]) -> bool {
    out.iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )
    })
}

// ── The offer and the accept ─────────────────────────────────────────────

/// **The prisoner click accepts, advances A to B, swaps the offer for the
/// medic's turn-in and shows 10701, in that order.** Q1 completed: the login
/// binds the offer (10011), the click runs 10013 (accept), whose
/// `mission_accepted` runs 10014. The traffic must show the accept, then the
/// step-B update, then the dialog; the state must show B active, the prisoner
/// offer unbound, the medic bound and 10701 offered. A resolve-only check
/// would not see the executor arms.
#[tokio::test]
async fn live_db_castle_side_medical_q2_prisoner_click_accepts_and_shows_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed(Q1)]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, OFFER_SLOT),
        vec![OFFER_SET],
        "the prisoner offer must be bound before the click"
    );

    let (handled, out) = click(&engine, &mut mgr, PRISONER_TAG, PRISONER).await;

    assert!(handled, "chain 10013 must handle the click");
    let accept = update_at(&out, MISSION, STEP_A);
    let advance = update_at(&out, MISSION, STEP_B);
    let dialog = dialog_at(&out, ACCEPT_DIALOG);
    let (accept, advance, dialog) = (
        accept.expect("the accept must persist 10301 at step A"),
        advance.expect("the nested advance must persist step B"),
        dialog.expect("the nested chain must send onDialogDisplay for dialog 10701"),
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
        "the prisoner offer must be unbound after accept"
    );
    assert_eq!(
        bound_at(&mgr, TURN_IN_SLOT),
        vec![TURN_IN_SET],
        "the medic turn-in must be bound after accept"
    );
    assert_eq!(
        offered(&mgr),
        vec![ACCEPT_DIALOG],
        "dialog 10701 must be shown"
    );
}

/// **The prisoner offer needs 10300 completed.** With 10300 not started, and
/// with 10300 still active at its step B, the login binds nothing and the
/// prisoner click is not handled by any chain. Revert proof: drop 10010's and
/// 10011's `mission_status 10300 completed` condition and the offer binds
/// with Q1 unfinished.
#[tokio::test]
async fn live_db_castle_side_medical_q2_offer_needs_10300_completed() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let cases = [
        ("10300 not started", Vec::new()),
        (
            "10300 active at step B",
            vec![active_at(Q1, Q1_STEP_B, &[Q1_OBJ_B])],
        ),
    ];
    for (label, missions) in cases {
        let mut mgr = staged(&pool, missions).await;
        log_in(&engine, &mut mgr).await;
        assert!(
            bound_at(&mgr, OFFER_SLOT).is_empty(),
            "{label}: the prisoner offer must not bind"
        );

        let (handled, out) = click(&engine, &mut mgr, PRISONER_TAG, PRISONER).await;

        assert!(!handled, "{label}: no chain may handle the prisoner click");
        assert!(
            !has_game_traffic(&out),
            "{label}: the prisoner click must send no mission or dialog traffic"
        );
        assert!(
            mission_of(&mgr, MISSION).is_none(),
            "{label}: 10301 must not be started"
        );
    }
}

// ── The turn-in ──────────────────────────────────────────────────────────

/// **The medic click at B unbinds, shows 10702 and completes 10301; anywhere
/// else it does nothing.** At B with 10300 completed, the relog binds the
/// turn-in (10017), and the click runs 10015: the unbind, dialog 10702 and the
/// completion all happen. At A, and with 10301 not started, the same click
/// fires nothing. Revert proof: drop 10015's step gate and the click at A runs the
/// completion.
#[tokio::test]
async fn live_db_castle_side_medical_q2_medic_click_completes_only_at_b() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    // At B: the turn-in completes the mission.
    let mut mgr = staged(
        &pool,
        vec![completed(Q1), active_at(MISSION, STEP_B, &[OBJ_B])],
    )
    .await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(
        bound_at(&mgr, TURN_IN_SLOT),
        vec![TURN_IN_SET],
        "the turn-in must be bound at B on login"
    );
    let (handled, out) = click(&engine, &mut mgr, MEDIC_TAG, MEDIC).await;
    assert!(handled, "chain 10015 must handle the click at B");
    assert!(
        dialog_at(&out, TURN_IN_DIALOG).is_some(),
        "the turn-in must send onDialogDisplay for dialog 10702"
    );
    assert_eq!(
        mission_of(&mgr, MISSION).unwrap().status,
        MISSION_COMPLETED,
        "the turn-in must complete 10301"
    );
    assert!(
        bound_at(&mgr, TURN_IN_SLOT).is_empty(),
        "the turn-in must be unbound after completion"
    );
    assert_eq!(offered(&mgr), vec![TURN_IN_DIALOG], "dialog 10702 shown");

    // At A: 10015 needs B active, so the click falls through.
    let mut mgr = staged(
        &pool,
        vec![completed(Q1), active_at(MISSION, STEP_A, &[OBJ_A])],
    )
    .await;
    let (handled, out) = click(&engine, &mut mgr, MEDIC_TAG, MEDIC).await;
    assert!(!handled, "no chain may complete 10301 from step A");
    assert!(
        dialog_at(&out, TURN_IN_DIALOG).is_none(),
        "no dialog 10702 may be shown at A"
    );
    assert_eq!(step_of(&mgr), Some(STEP_A), "the step must stay at A");
    assert_eq!(
        mission_of(&mgr, MISSION).unwrap().status,
        MISSION_ACTIVE,
        "10301 must stay active at A"
    );

    // Q1 only (not started): 10015 needs 10301 active, so the click falls through.
    let mut mgr = staged(&pool, vec![completed(Q1)]).await;
    let (handled, out) = click(&engine, &mut mgr, MEDIC_TAG, MEDIC).await;
    assert!(!handled, "no chain may complete an unstarted 10301");
    assert!(
        !has_game_traffic(&out),
        "a medic click with 10301 not started must send no traffic"
    );
}

// ── Repeat clicks ────────────────────────────────────────────────────────

/// **A repeat click fires nothing.** After accept the mission is active at A,
/// so 10013's `not_active` gate fails: a second prisoner click is handled by
/// no chain and sends no traffic. After completion the medic click finds
/// 10301 completed, and nothing fires. Revert proof: relax 10013's
/// `mission_status 10301 not_active` to allow active and the second prisoner
/// click accepts again.
#[tokio::test]
async fn live_db_castle_side_medical_q2_repeat_click_noop() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    // A real accept first (the mission is then active at B), then a second click.
    let mut mgr = staged(&pool, vec![completed(Q1)]).await;
    log_in(&engine, &mut mgr).await;
    let (handled, _) = click(&engine, &mut mgr, PRISONER_TAG, PRISONER).await;
    assert!(handled, "the first prisoner click must accept");
    assert_eq!(step_of(&mgr), Some(STEP_B), "the accept must reach B");
    let (handled, out) = click(&engine, &mut mgr, PRISONER_TAG, PRISONER).await;
    assert!(
        !handled,
        "no chain may handle a second prisoner click (10013 needs not_active)"
    );
    assert!(
        !has_game_traffic(&out),
        "a second prisoner click must send no mission or dialog traffic"
    );
    assert_eq!(step_of(&mgr), Some(STEP_B), "the step must stay at B");

    // Mid-accept state (active at A, e.g. a crash before the nested advance).
    let mut mgr = staged(
        &pool,
        vec![completed(Q1), active_at(MISSION, STEP_A, &[OBJ_A])],
    )
    .await;
    let (handled, out) = click(&engine, &mut mgr, PRISONER_TAG, PRISONER).await;
    assert!(
        !handled,
        "no chain may handle a prisoner click with 10301 active at A"
    );
    assert!(
        !has_game_traffic(&out),
        "a prisoner click with 10301 active must send no mission or dialog traffic"
    );
    assert_eq!(step_of(&mgr), Some(STEP_A), "the step must stay at A");
    assert!(offered(&mgr).is_empty(), "no dialog may be shown");

    let mut mgr = staged(&pool, vec![completed(Q1), completed(MISSION)]).await;
    let (handled, out) = click(&engine, &mut mgr, MEDIC_TAG, MEDIC).await;
    assert!(
        !handled,
        "no chain may handle a medic click after 10301 completed"
    );
    assert!(
        !has_game_traffic(&out),
        "a medic click after completion must send no mission or dialog traffic"
    );
    assert_eq!(
        mission_of(&mgr, MISSION).unwrap().status,
        MISSION_COMPLETED,
        "10301 must stay completed"
    );
}
