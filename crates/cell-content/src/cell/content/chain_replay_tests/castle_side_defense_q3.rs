//! Castle defense_q3 side story (mission 10312, *Hold the Bravo Route*),
//! CS-04b3c1: the chains `castle_side_stories_chains.sql` seeds as 10120-10129.
//!
//! The offer binds the giver (1000010 at slot 425) once 10310 is completed.
//! The giver click (10123) accepts; the nested `mission_accepted` (10124)
//! unbinds the giver and shows dialog 10717. Each Bravo officer kill
//! completes its own objective (10125-10128), and the fourth completes the
//! mission and shows dialog 10718 (10129). Every click and kill goes through
//! the real dispatcher (`fire_interact_tag`, `fire_player_loaded`,
//! `fire_entity_death`), so a resolve-only check cannot pass a broken
//! executor arm.
//!
//! Each test fails if its seed row is removed: without 10123 the click
//! accepts nothing, and without 10129 the fourth kill shows no completion.
//! The three gate tests (re-kill, outside the window, beside 708) also compare
//! what the engine resolves, since a dropped condition row can hide behind the
//! executor's own state check. The exact row shapes are in
//! castle_side_defense_q3_chains.

use std::collections::BTreeSet;

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
use super::super::{fire_entity_death, fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ChainEngine;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// The giver NPC, the interaction target of the 10123 click.
const GIVER: u32 = 2;
const CASTLE: &str = "Castle";
const GIVER_TAG: &str = "Castle_DefenseQ3_Giver";
const MISSION: i32 = 10312;
const PREREQ: i32 = 10310;
const STEP: i32 = 10366;
/// The four objectives, one per Bravo officer and the Muelbach kill.
const OBJECTIVES: [i32; 4] = [10486, 10487, 10488, 10489];
/// The kill tags, paired index for index with `OBJECTIVES`.
const KILL_TAGS: [&str; 4] = [
    "Castle_BravoOfficer1",
    "Castle_BravoOfficer2",
    "Castle_BravoOfficer3",
    "Castle_Muelbach",
];
/// The giver offer: dialog-set map 1000010 bound at slot 425.
const OFFER_SET: i32 = 1000010;
const OFFER_SLOT: i32 = 425;
const OFFER_DIALOG: i32 = 10717;
const DONE_DIALOG: i32 = 10718;
/// Wire index of the client method onDialogDisplay.
const ON_DIALOG_DISPLAY: u16 = 105;
/// Every chain of the defense_q3 side story, 10120-10129.
const ALL_CHAINS: [i32; 10] = [
    10120, 10121, 10122, 10123, 10124, 10125, 10126, 10127, 10128, 10129,
];

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

/// Castle with player `ENTITY` holding `missions`, the giver and the four
/// kill targets, with the caches the accept and the dialog path read.
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
    mgr.create_entity(GIVER, CASTLE, [3.0; 3], [0.0; 3])
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

/// 10310 completed: the prerequisite the offer gates on.
fn completed_prereq() -> MissionInstance {
    let mut mission = MissionInstance::new(PREREQ, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 10312 at its single step with every objective in `done` completed and
/// the rest active, as the accept and the kills would leave it.
fn active_mission(done: &[i32]) -> MissionInstance {
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

fn mission_of(mgr: &SpaceManager) -> Option<&MissionInstance> {
    mgr.get_entity(ENTITY)
        .unwrap()
        .missions
        .get_mission(MISSION)
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

/// Position of the `MissionUpdate` for 10312 at `step`, if sent.
fn update_at(out: &[CellToBaseMsg], step: Option<i32>) -> Option<usize> {
    out.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate { mission_id: MISSION, current_step_id, .. }
                if *current_step_id == step
        )
    })
}

/// Position of the onDialogDisplay call that shows `dialog`, if sent.
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

fn any_mission_traffic(out: &[CellToBaseMsg]) -> bool {
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

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(ENTITY, PLAYER, CASTLE, engine, &tx, mgr).await;
}

/// Click the giver; returns whether a chain handled the click and everything
/// sent to the cell-to-base channel.
async fn click_giver(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, GIVER_TAG, GIVER, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

/// Kill the NPC tagged `tag` as `ENTITY`; returns everything sent.
async fn kill(engine: &ChainEngine, mgr: &mut SpaceManager, tag: &str) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_entity_death(ENTITY, PLAYER, tag, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// The chain ids the engine resolves for a kill of `tag`, built as
/// `fire_entity_death` builds the context. Resolution sees the conditions
/// themselves: the executor re-checks the mission state, so a dropped
/// condition row can hide behind it in the traffic and show here.
fn resolved_on_kill(engine: &ChainEngine, mgr: &SpaceManager, tag: &str) -> BTreeSet<i64> {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(ENTITY as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    populate_world_context(ENTITY, mgr, &mut ctx);
    let entity = mgr.get_entity(ENTITY).unwrap();
    populate_mission_context(entity, &mut ctx);
    ctx.set_param("archetype".to_string(), serde_json::json!(1));
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: Some(cimmeria_common::EntityId(ENTITY as i32)),
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

/// **The offer binds the giver only once 10310 is completed.** With 10310
/// completed, the `mission_completed 10310` event (10120) and the login
/// (10121) each bind 1000010 at slot 425; with no prerequisite neither binds
/// anything. Revert proof: drop the 10310 condition from 10120 or 10121 and
/// the matching no-prerequisite half fails.
#[tokio::test]
async fn live_db_castle_side_defense_q3_offer_binds_after_prereq() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let (tx, _rx) = mpsc::channel(256);
    let mut on_completion = staged(&pool, vec![completed_prereq()]).await;
    fire_mission_completed(ENTITY, PLAYER, PREREQ, &engine, &tx, &mut on_completion).await;
    assert_eq!(
        bound_at(&on_completion, OFFER_SLOT),
        vec![OFFER_SET],
        "10310 completing must bind the offer at slot 425 (chain 10120)"
    );
    let mut no_prereq = staged(&pool, Vec::new()).await;
    fire_mission_completed(ENTITY, PLAYER, PREREQ, &engine, &tx, &mut no_prereq).await;
    assert!(
        bound_at(&no_prereq, OFFER_SLOT).is_empty(),
        "10310 not completed: the completion event must bind nothing"
    );

    let mut open = staged(&pool, vec![completed_prereq()]).await;
    log_in(&engine, &mut open).await;
    assert_eq!(
        bound_at(&open, OFFER_SLOT),
        vec![OFFER_SET],
        "10310 completed: the offer must be bound at slot 425"
    );

    let mut closed = staged(&pool, Vec::new()).await;
    log_in(&engine, &mut closed).await;
    assert!(
        bound_at(&closed, OFFER_SLOT).is_empty(),
        "10310 not completed: no offer may be bound"
    );
}

/// **Accept unbinds the giver and shows 10717, in that order.** A giver
/// click accepts 10312 at step 10366 (10123); the nested `mission_accepted`
/// (10124) unbinds slot 425 and sends onDialogDisplay for 10717. Traffic
/// order must be accept, then dialog. Revert proof: delete 10123 and the
/// click accepts nothing.
#[tokio::test]
async fn live_db_castle_side_defense_q3_accept_unbinds_and_shows_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![completed_prereq()]).await;
    log_in(&engine, &mut mgr).await;
    assert_eq!(bound_at(&mgr, OFFER_SLOT), vec![OFFER_SET]);

    let (handled, out) = click_giver(&engine, &mut mgr).await;

    assert!(handled, "chain 10123 must handle the giver click");
    let accept = update_at(&out, Some(STEP)).expect("the accept must persist 10312 at 10366");
    let dialog = dialog_at(&out, OFFER_DIALOG).expect("10124 must send dialog 10717");
    assert!(
        accept < dialog,
        "traffic order must be accept, then dialog; got {accept}, {dialog}"
    );
    assert!(
        bound_at(&mgr, OFFER_SLOT).is_empty(),
        "the giver must be unbound after accept"
    );
    assert_eq!(
        offered(&mgr),
        vec![OFFER_DIALOG],
        "dialog 10717 must be shown"
    );
    assert_eq!(mission_of(&mgr).unwrap().status, MISSION_ACTIVE);
}

/// **A repeat click at the active step fires nothing.** The mission is
/// already active, so 10123's `not_active` gate fails: no chain handles the
/// click and there is no accept, dialog or mission traffic. Revert proof:
/// drop 10123's `mission_status 10312` condition and the click is handled.
#[tokio::test]
async fn live_db_castle_side_defense_q3_repeat_click_noop() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![active_mission(&[])]).await;

    let (handled, out) = click_giver(&engine, &mut mgr).await;

    assert!(!handled, "an active 10312 must not accept again");
    assert!(
        out.is_empty(),
        "a repeat click must send nothing; got {out:?}"
    );
    assert_eq!(step_is(&mgr), Some(STEP));
}

fn step_is(mgr: &SpaceManager) -> Option<i32> {
    mission_of(mgr).and_then(|m| m.current_step_id)
}

/// **Each tag completes exactly its own objective.** For each of the four
/// kill tags, one kill on an active 10312 completes that tag's objective and
/// no other. Revert proof: swap two objective ids in the seed and the
/// per-tag comparison fails.
#[tokio::test]
async fn live_db_castle_side_defense_q3_each_target_completes_its_objective() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for (tag, objective) in KILL_TAGS.iter().zip(OBJECTIVES) {
        let mut mgr = staged(&pool, vec![active_mission(&[])]).await;
        let out = kill(&engine, &mut mgr, tag).await;
        assert!(
            update_at(&out, Some(STEP)).is_some(),
            "{tag} must send a 10312 update"
        );
        assert_eq!(
            mission_of(&mgr).unwrap().completed_objectives,
            vec![objective],
            "{tag} must complete exactly objective {objective}"
        );
    }
}

/// **The fourth distinct kill completes 10312 and shows 10718.** After three
/// objectives, the fourth kill completes the mission (engine auto-complete)
/// and 10129 sends onDialogDisplay for 10718. A resolve-only check cannot
/// see the completion. Revert proof: delete 10129 and the dialog is absent.
#[tokio::test]
async fn live_db_castle_side_defense_q3_fourth_kill_completes_and_shows_dialog() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![active_mission(&OBJECTIVES[..3])]).await;

    let out = kill(&engine, &mut mgr, KILL_TAGS[3]).await;

    assert_eq!(
        mission_of(&mgr).unwrap().status,
        MISSION_COMPLETED,
        "the fourth distinct objective must complete 10312"
    );
    assert!(
        dialog_at(&out, DONE_DIALOG).is_some(),
        "10129 must send dialog 10718"
    );
}

/// **A re-kill of a completed objective is inert.** After 10486 completes,
/// the same tag resolves no chain for 10312: its `objective_status` gate
/// fails, the completion list is unchanged and no mission frame is sent. The
/// resolve check is the one that sees the gate, because the executor does
/// not report a completion that changes nothing. Revert proof: drop or blank
/// 10125's `objective_status` row and 10125 resolves on the re-kill.
#[tokio::test]
async fn live_db_castle_side_defense_q3_rekill_is_inert() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![active_mission(&[OBJECTIVES[0]])]).await;

    assert_eq!(
        resolved_on_kill(&engine, &mgr, KILL_TAGS[0]),
        BTreeSet::new(),
        "a re-kill must resolve no chain"
    );
    let out = kill(&engine, &mut mgr, KILL_TAGS[0]).await;

    assert!(
        !any_mission_traffic(&out),
        "a re-kill must send no mission frame"
    );
    assert_eq!(
        mission_of(&mgr).unwrap().completed_objectives,
        vec![OBJECTIVES[0]],
        "a re-kill must not count again"
    );
}

/// **Kills outside the active window are inert.** Before accept (no 10312)
/// and after completion, a Bravo kill resolves no chain and sends no mission
/// frame. Dropping only `mission_status 10312 eq active` from 10125 does not
/// change what resolves here (the `objective_status` row refuses both cases
/// on its own, checked by mutation), so live_db_castle_side_defense_q3_chains_load
/// is the test that pins that row; this one pins the behaviour.
#[tokio::test]
async fn live_db_castle_side_defense_q3_kill_outside_window_is_inert() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut before = staged(&pool, vec![completed_prereq()]).await;
    assert_eq!(
        resolved_on_kill(&engine, &before, KILL_TAGS[0]),
        BTreeSet::new(),
        "a kill before accept must resolve no chain"
    );
    let out = kill(&engine, &mut before, KILL_TAGS[0]).await;
    assert!(
        !any_mission_traffic(&out),
        "a kill before accept must be inert"
    );
    assert!(mission_of(&before).is_none());

    let mut mission = active_mission(&OBJECTIVES);
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    let mut after = staged(&pool, vec![mission]).await;
    assert_eq!(
        resolved_on_kill(&engine, &after, KILL_TAGS[0]),
        BTreeSet::new(),
        "a kill after completion must resolve no chain"
    );
    let out = kill(&engine, &mut after, KILL_TAGS[0]).await;
    assert!(
        !any_mission_traffic(&out),
        "a kill after completion must be inert"
    );
}

/// **A kill credits 10312 while 708 is active, and 708 still advances.** With
/// 708 active at step 2416 and 10312 active, one Bravo kill completes its
/// 10312 objective and also fires 708's chain 1346 (advance to 2417); the
/// two chains share the tag and neither may block the other. Revert proof: add
/// `mission_status 708 neq active` to 10125 and 10125 stops resolving beside
/// 1346.
#[tokio::test]
async fn live_db_castle_side_defense_q3_coexists_with_708() {
    let pool = require_db_or_skip!();
    let mut ids = ALL_CHAINS.to_vec();
    ids.push(1346);
    let engine = engine_with(&pool, &ids).await;
    let mut mission_708 = MissionInstance::new(708, 2416, Vec::new());
    mission_708.status = MISSION_ACTIVE;
    mission_708.current_step_id = Some(2416);
    let mut mgr = staged(&pool, vec![active_mission(&[]), mission_708]).await;

    assert_eq!(
        resolved_on_kill(&engine, &mgr, KILL_TAGS[0]),
        BTreeSet::from([1346, 10125]),
        "the kill must resolve both 10125 and 708's 1346"
    );
    let out = kill(&engine, &mut mgr, KILL_TAGS[0]).await;

    assert!(
        update_at(&out, Some(STEP)).is_some(),
        "10125 must complete 10486 while 708 is active"
    );
    assert_eq!(
        mission_of(&mgr).unwrap().completed_objectives,
        vec![OBJECTIVES[0]]
    );
    // The same kill must also fire the 708 chain 1346 (advance to 2417):
    // neither quest's chain may block the other.
    assert!(
        out.iter().any(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: 708,
                current_step_id: Some(2417),
                ..
            }
        )),
        "chain 1346 must advance 708 to 2417 on the same kill; got {out:?}"
    );
}
