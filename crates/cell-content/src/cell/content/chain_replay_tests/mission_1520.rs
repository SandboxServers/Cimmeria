//! Mission 1520 (Castle Infirmary) graph, CS-02c2: the chains that carry a
//! player from the Ogilvie talk to the completed mission.
//!
//! `castle_1520_chains.sql`: 1378 (Ogilvie close), 1371 (dispenser starts
//! Livewire), 1372 (Livewire victory), 1379 (4984 close), 1373 (4985 close),
//! plus the relog chains 1381-1383 for the two-player and "other step"
//! cases. The entry half (1367, 1368, 1380) is `mission_1520_entry`; the
//! static seed guard for 1371/1372 is `mission_1520_seed_guard`.
//!
//! Every event goes through the real dispatcher (`fire_dialog_choice`,
//! `fire_interact_tag`, `fire_player_loaded`) except the victory callback,
//! which goes through `fire_guarded_minigame_chain_by_id`, the seam CS-02L
//! sealed for Castle callbacks. A resolve-only test cannot see the executor
//! arms (the binds, the dialog offers, the completion), so every assertion
//! reads the resulting player state or the cell-to-base traffic.
//!
//! The 1372 "emits nothing" cases are the behavioural half of the seed guard:
//! each fails if 1372's `step_status` row is removed (the chain would then
//! advance a player who is not on 4453).

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::{
    fire_dialog_choice, fire_enter_region, fire_guarded_minigame_chain_by_id, fire_interact_tag,
    fire_player_loaded,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{RegionData, SpaceManager, REGION_FLAG_CLIENT_HINTED};
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
/// A second player and the dispenser actor, for the two-player and walk tests.
const ENTITY_B: u32 = 2;
const PLAYER_B: i32 = 43;
const DISPENSER: u32 = 3;
const CASTLE: &str = "Castle";
const INFIRMARY: &str = "Castle.Infirmary";
const TAG: &str = "Castle_MedDispenser";
const MISSION: i32 = 1520;
const S_OGILVIE: i32 = 4452;
const S_DISPENSER: i32 = 4453;
const S_RADIO: i32 = 4454;
/// Every chain the 1520 graph uses, entry included (the walk needs them all).
const ALL_CHAINS: [i32; 11] = [
    1367, 1368, 1371, 1372, 1373, 1378, 1379, 1380, 1381, 1382, 1383,
];

// ── Harness ──────────────────────────────────────────────────────────────

async fn engine_with(pool: &PgPool, ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    let mut seen = Vec::new();
    for &id in ids {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

/// Castle with player `ENTITY` (archetype 1) holding `missions`, a dispenser
/// actor, and the dialog-set and monologue caches loaded from the seed. The
/// monologue cache matters: 4984 is a monologue (speaker 0) and binds the
/// player, while 4985 needs an NPC and resolves it from the pin.
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
    // The accept's step-activation replay re-fires `enter_region` only for a
    // client-hinted volume that contains the server-known position.
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
    mgr.create_entity(DISPENSER, CASTLE, [3.0; 3], [0.0; 3])
        .unwrap();
    add_player(&mut mgr, ENTITY, PLAYER, missions);
    mgr
}

fn add_player(mgr: &mut SpaceManager, eid: u32, pid: i32, missions: Vec<MissionInstance>) {
    mgr.create_entity(eid, CASTLE, [0.0; 3], [0.0; 3]).unwrap();
    let entity = mgr.get_entity_mut(eid).unwrap();
    entity.is_player = true;
    entity.player_id = Some(pid);
    entity.archetype_id = Some(1);
    for mission in missions {
        entity.missions.add_mission(mission);
    }
}

/// 1520 at `step` with its objectives active, as the accept would leave it.
fn at(step: i32, objectives: &[i32]) -> MissionInstance {
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

/// The objectives each step holds (CS-02S points 4, 6, 8).
fn step_objectives(step: i32) -> &'static [i32] {
    match step {
        S_OGILVIE => &[5171, 5172],
        S_DISPENSER => &[5173],
        _ => &[5174],
    }
}

fn mission_at(step: i32) -> MissionInstance {
    at(step, step_objectives(step))
}

fn completed_1520() -> MissionInstance {
    let mut mission = mission_at(S_RADIO);
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

fn mission_of(mgr: &SpaceManager, eid: u32) -> Option<&MissionInstance> {
    mgr.get_entity(eid).unwrap().missions.get_mission(MISSION)
}

fn step_of(mgr: &SpaceManager, eid: u32) -> Option<i32> {
    mission_of(mgr, eid).and_then(|m| m.current_step_id)
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// `(game, difficulty, victory chains)` of every `StartMinigame` in `msgs`.
fn starts(msgs: &[CellToBaseMsg]) -> Vec<(String, u32, Vec<i64>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::StartMinigame {
                game_name,
                difficulty,
                on_victory_chains,
                ..
            } => Some((game_name.clone(), *difficulty, on_victory_chains.clone())),
            _ => None,
        })
        .collect()
}

/// Close a dialog (button -1, the wire value for a closed screen).
async fn close(engine: &ChainEngine, mgr: &mut SpaceManager, eid: u32, pid: i32, dialog: i32) {
    let (tx, _rx) = mpsc::channel(256);
    fire_dialog_choice(eid, pid, dialog, -1, engine, &tx, mgr).await;
}

async fn victory(engine: &ChainEngine, mgr: &mut SpaceManager, eid: u32) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    fire_guarded_minigame_chain_by_id(1372, eid, engine, &tx, mgr).await;
    drain(&mut rx)
}

async fn click_dispenser(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, TAG, DISPENSER, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

async fn log_in(engine: &ChainEngine, mgr: &mut SpaceManager, eid: u32, pid: i32) {
    let (tx, _rx) = mpsc::channel(256);
    fire_player_loaded(eid, pid, CASTLE, engine, &tx, mgr).await;
}

/// The dialog-set-map ids bound on `eid` at `slot`, ascending.
fn bound_at(mgr: &SpaceManager, eid: u32, slot: i32) -> Vec<i32> {
    let mut ids: Vec<i32> = mgr
        .get_entity(eid)
        .unwrap()
        .available_interactions
        .get(&slot)
        .map(|v| v.iter().map(|&(id, _, _)| id).collect())
        .unwrap_or_default();
    ids.sort_unstable();
    ids
}

/// Every populated bind slot of `eid`, so "nothing else is bound" is testable.
fn binds(mgr: &SpaceManager, eid: u32) -> Vec<(i32, Vec<i32>)> {
    let mut slots: Vec<i32> = mgr
        .get_entity(eid)
        .unwrap()
        .available_interactions
        .keys()
        .copied()
        .collect();
    slots.sort_unstable();
    slots
        .into_iter()
        .map(|s| (s, bound_at(mgr, eid, s)))
        .filter(|(_, ids)| !ids.is_empty())
        .collect()
}

fn offered(mgr: &SpaceManager, eid: u32) -> Vec<i32> {
    mgr.get_entity(eid).unwrap().offered_dialogs()
}

// ── G1/G2: the Ogilvie close, chain 1378 ─────────────────────────────────

/// **The Ogilvie close advances 4452 to 4453 and swaps the binds.** The login
/// at 4452 (1381) gives the player the Ogilvie bind (5827 at slot 179); the
/// 4983 close then removes it, advances, and binds the dispenser (5828 at
/// slot 412). Advancing force-completes both 4452 objectives, 5171 and the
/// hidden 5172.
#[tokio::test]
async fn live_db_1520_ogilvie_close_advances_to_4453_and_binds_dispenser() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![mission_at(S_OGILVIE)]).await;
    log_in(&engine, &mut mgr, ENTITY, PLAYER).await;
    assert_eq!(
        binds(&mgr, ENTITY),
        vec![(179, vec![5827])],
        "pre-close bind"
    );

    close(&engine, &mut mgr, ENTITY, PLAYER, 4983).await;

    assert_eq!(
        step_of(&mgr, ENTITY),
        Some(S_DISPENSER),
        "1378 must advance"
    );
    assert_eq!(
        binds(&mgr, ENTITY),
        vec![(412, vec![5828])],
        "Ogilvie's bind goes, the dispenser's arrives, nothing else"
    );
    let done = &mission_of(&mgr, ENTITY).unwrap().completed_objectives;
    assert!(
        done.contains(&5171) && done.contains(&5172),
        "advance_step credits both 4452 objectives; got {done:?}"
    );
    assert_eq!(mission_of(&mgr, ENTITY).unwrap().status, MISSION_ACTIVE);
}

/// **The 4983 close does nothing at any other step or state.** Each case must
/// leave the step and the binds exactly as they were.
#[tokio::test]
async fn live_db_1520_ogilvie_close_is_inert_off_step_4452() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: Vec<(&str, Vec<MissionInstance>, Option<i32>)> = vec![
        ("4453", vec![mission_at(S_DISPENSER)], Some(S_DISPENSER)),
        ("4454", vec![mission_at(S_RADIO)], Some(S_RADIO)),
        ("not active", vec![], None),
        ("completed", vec![completed_1520()], None),
    ];
    for (label, missions, expected_step) in cases {
        let mut mgr = staged(&pool, missions).await;
        close(&engine, &mut mgr, ENTITY, PLAYER, 4983).await;
        assert_eq!(step_of(&mgr, ENTITY), expected_step, "state {label}");
        assert!(
            binds(&mgr, ENTITY).is_empty(),
            "state {label}: the close must bind nothing; got {:?}",
            binds(&mgr, ENTITY)
        );
    }
}

// ── G3/G4: the dispenser click, chain 1371 ───────────────────────────────

/// **The dispenser at 4453 starts Livewire carrying `[1372]`.** The click is
/// handled, and exactly one `StartMinigame` leaves the cell with the authored
/// game, difficulty 1 and only the victory chain 1372.
#[tokio::test]
async fn live_db_1520_dispenser_at_4453_starts_livewire_with_victory_1372() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![mission_at(S_DISPENSER)]).await;

    let (handled, out) = click_dispenser(&engine, &mut mgr).await;

    assert!(handled, "1371 must claim the click at 4453");
    assert_eq!(
        starts(&out),
        vec![("Livewire".to_string(), 1, vec![1372])],
        "one Livewire, difficulty 1, victory [1372]; got {out:?}"
    );
}

/// **The dispenser at any other step does nothing.** The click falls through
/// unhandled and no minigame starts: at 4452 and 4454, with 1520 not taken,
/// and with it completed.
#[tokio::test]
async fn live_db_1520_dispenser_off_step_4453_starts_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: Vec<(&str, Vec<MissionInstance>)> = vec![
        ("4452", vec![mission_at(S_OGILVIE)]),
        ("4454", vec![mission_at(S_RADIO)]),
        ("not active", vec![]),
        ("completed", vec![completed_1520()]),
    ];
    for (label, missions) in cases {
        let mut mgr = staged(&pool, missions).await;
        let (handled, out) = click_dispenser(&engine, &mut mgr).await;
        assert!(!handled, "state {label}: the click must not be claimed");
        assert!(
            starts(&out).is_empty(),
            "state {label}: no minigame may start; got {out:?}"
        );
    }
}

// ── G5/G6: the victory callback, chain 1372 ──────────────────────────────

/// **Victory at 4453 advances to 4454, drops the dispenser bind and shows
/// 4984.** The player starts with the dispenser bind (via the 4453 login).
#[tokio::test]
async fn live_db_1520_victory_at_4453_advances_unbinds_and_shows_4984() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![mission_at(S_DISPENSER)]).await;
    log_in(&engine, &mut mgr, ENTITY, PLAYER).await;
    assert_eq!(
        binds(&mgr, ENTITY),
        vec![(412, vec![5828])],
        "pre-victory bind"
    );

    let out = victory(&engine, &mut mgr, ENTITY).await;

    assert!(!out.is_empty(), "the victory chain must send something");
    assert_eq!(step_of(&mgr, ENTITY), Some(S_RADIO), "1372 must advance");
    assert!(
        binds(&mgr, ENTITY).is_empty(),
        "the dispenser bind must be removed; got {:?}",
        binds(&mgr, ENTITY)
    );
    assert!(
        mission_of(&mgr, ENTITY)
            .unwrap()
            .completed_objectives
            .contains(&5173),
        "advance_step credits 5173"
    );
    assert_eq!(offered(&mgr, ENTITY), vec![4984], "4984 must be displayed");
    assert_eq!(
        mission_of(&mgr, ENTITY).unwrap().status,
        MISSION_ACTIVE,
        "the victory must not complete the mission"
    );
}

/// **The same callback at 4454, at 4452, with 1520 completed or never taken,
/// emits nothing.** A stale or replayed callback must not move a player, and
/// the channel stays completely silent. Revert proof: remove 1372's
/// `step_status` row and the 4454 case advances, completes or re-shows 4984.
#[tokio::test]
async fn live_db_1520_victory_off_step_4453_emits_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let cases: Vec<(&str, Vec<MissionInstance>, Option<i32>)> = vec![
        ("4454", vec![mission_at(S_RADIO)], Some(S_RADIO)),
        ("4452", vec![mission_at(S_OGILVIE)], Some(S_OGILVIE)),
        ("completed", vec![completed_1520()], None),
        ("not active", vec![], None),
    ];
    for (label, missions, expected_step) in cases {
        let mut mgr = staged(&pool, missions).await;
        let out = victory(&engine, &mut mgr, ENTITY).await;
        assert!(
            out.is_empty(),
            "state {label}: nothing may be sent; got {out:?}"
        );
        assert_eq!(step_of(&mgr, ENTITY), expected_step, "state {label}");
        assert!(offered(&mgr, ENTITY).is_empty(), "state {label}: no dialog");
    }
}

/// **The victory in the wrong world emits nothing.** The world condition is
/// the other half of 1372's guard; the same mission state in another world
/// must not advance.
#[tokio::test]
async fn live_db_1520_victory_in_another_world_emits_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![]).await;
    mgr.parse_spaces_xml(
        r#"<Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#)
        .unwrap();
    mgr.stamp_world_rows(&load_world_rows(&pool).await.expect("world cache"));
    mgr.create_entity(ENTITY_B, "Castle_CellBlock", [1.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(ENTITY_B).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER_B);
    e.missions.add_mission(mission_at(S_DISPENSER));

    let out = victory(&engine, &mut mgr, ENTITY_B).await;

    assert!(
        out.is_empty(),
        "wrong world: nothing may be sent; got {out:?}"
    );
    assert_eq!(step_of(&mgr, ENTITY_B), Some(S_DISPENSER));
}

// ── G7/G8: the radio call and the completion ─────────────────────────────

/// **The 4984 close shows 4985, once, only at 4454.** 4985 is not a
/// monologue, so its speaker resolves from the pin; the test pins the
/// dispenser, as `handle_interact` does on the real click. At 4453 the close
/// shows nothing.
#[tokio::test]
async fn live_db_1520_4984_close_shows_4985_only_at_4454() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut mgr = staged(&pool, vec![mission_at(S_RADIO)]).await;
    mgr.get_entity_mut(ENTITY).unwrap().last_interaction_target = Some(DISPENSER);
    close(&engine, &mut mgr, ENTITY, PLAYER, 4984).await;
    assert_eq!(offered(&mgr, ENTITY), vec![4985], "4984 close shows 4985");
    assert_eq!(step_of(&mgr, ENTITY), Some(S_RADIO), "it must not advance");

    let mut early = staged(&pool, vec![mission_at(S_DISPENSER)]).await;
    early
        .get_entity_mut(ENTITY)
        .unwrap()
        .last_interaction_target = Some(DISPENSER);
    close(&engine, &mut early, ENTITY, PLAYER, 4984).await;
    assert!(
        offered(&early, ENTITY).is_empty(),
        "at 4453 the 4984 close must show nothing; got {:?}",
        offered(&early, ENTITY)
    );
}

/// **The 4985 close completes 1520 at 4454, and only there.** Advancing past
/// the last step does not complete a mission; `complete_mission` does, and
/// credits the final step's objective 5174. No chain here authors
/// `complete_objective` (see the static guard below).
#[tokio::test]
async fn live_db_1520_4985_close_completes_only_at_4454() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;

    let mut mgr = staged(&pool, vec![mission_at(S_RADIO)]).await;
    close(&engine, &mut mgr, ENTITY, PLAYER, 4985).await;
    let mission = mission_of(&mgr, ENTITY).unwrap();
    assert_eq!(mission.status, MISSION_COMPLETED, "1373 must complete 1520");
    assert!(
        mission.completed_objectives.contains(&5174),
        "complete_mission credits 5174; got {:?}",
        mission.completed_objectives
    );

    for (label, missions) in [
        ("4453", vec![mission_at(S_DISPENSER)]),
        ("4452", vec![mission_at(S_OGILVIE)]),
    ] {
        let mut early = staged(&pool, missions).await;
        close(&engine, &mut early, ENTITY, PLAYER, 4985).await;
        assert_eq!(
            mission_of(&early, ENTITY).unwrap().status,
            MISSION_ACTIVE,
            "a 4985 close at {label} must not complete the mission"
        );
    }
}

// ── G9/G10: relog and two players ────────────────────────────────────────

/// **A login at any other state binds nothing and changes nothing.** The
/// three relog chains are keyed by step; a player with 1520 not taken or
/// completed matches none of them. (The three positive cases are in
/// `mission_1520_entry`.)
#[tokio::test]
async fn live_db_1520_relog_off_graph_binds_nothing() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    for (label, missions) in [
        ("not active", vec![]),
        ("completed", vec![completed_1520()]),
    ] {
        let mut mgr = staged(&pool, missions).await;
        log_in(&engine, &mut mgr, ENTITY, PLAYER).await;
        assert!(
            binds(&mgr, ENTITY).is_empty(),
            "state {label}: a login must bind nothing; got {:?}",
            binds(&mgr, ENTITY)
        );
        assert_eq!(
            mission_of(&mgr, ENTITY).map(|m| m.status),
            if label == "completed" {
                Some(MISSION_COMPLETED)
            } else {
                None
            },
            "state {label}"
        );
    }
}

/// **Two players on different steps hold independent state.** A is on 4452,
/// B on 4453, both in Castle. A's bind list holds Ogilvie's 5827 only and
/// B's the dispenser's 5828 only. B's victory advances B and leaves A
/// untouched; A's later 4983 close advances A without disturbing B.
#[tokio::test]
async fn live_db_1520_two_players_keep_independent_binds_and_steps() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, vec![mission_at(S_OGILVIE)]).await;
    add_player(&mut mgr, ENTITY_B, PLAYER_B, vec![mission_at(S_DISPENSER)]);
    log_in(&engine, &mut mgr, ENTITY, PLAYER).await;
    log_in(&engine, &mut mgr, ENTITY_B, PLAYER_B).await;

    assert_eq!(
        binds(&mgr, ENTITY),
        vec![(179, vec![5827])],
        "A holds Ogilvie"
    );
    assert_eq!(
        binds(&mgr, ENTITY_B),
        vec![(412, vec![5828])],
        "B holds dispenser"
    );

    victory(&engine, &mut mgr, ENTITY_B).await;
    assert_eq!(step_of(&mgr, ENTITY_B), Some(S_RADIO), "B advanced");
    assert_eq!(step_of(&mgr, ENTITY), Some(S_OGILVIE), "A untouched");
    assert_eq!(
        binds(&mgr, ENTITY),
        vec![(179, vec![5827])],
        "A's bind survives"
    );
    assert!(
        binds(&mgr, ENTITY_B).is_empty(),
        "B's dispenser bind is gone"
    );

    close(&engine, &mut mgr, ENTITY, PLAYER, 4983).await;
    assert_eq!(step_of(&mgr, ENTITY), Some(S_DISPENSER), "A advanced");
    assert_eq!(step_of(&mgr, ENTITY_B), Some(S_RADIO), "B untouched by A");
    assert_eq!(binds(&mgr, ENTITY), vec![(412, vec![5828])]);
}

// ── G11: the full walk ───────────────────────────────────────────────────

/// **Accept to complete, through the real dispatchers.** Entry offers the
/// mission and binds Ogilvie; the 4983 close, the dispenser click, the
/// victory callback, the 4984 close and the 4985 close each fire the chain
/// the seed names. The end state: 1520 completed, the final step's objective
/// 5174 credited, no step left active, and no bind left behind.
#[tokio::test]
async fn live_db_1520_full_walk_accept_to_complete() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let completed_704 = {
        let mut m = MissionInstance::new(704, 0, vec![]);
        m.status = MISSION_COMPLETED;
        m
    };
    let mut mgr = staged(&pool, vec![completed_704]).await;

    let (tx, _rx) = mpsc::channel(256);
    fire_enter_region(ENTITY, PLAYER, INFIRMARY, &engine, &tx, &mut mgr).await;
    assert_eq!(step_of(&mgr, ENTITY), Some(S_OGILVIE), "offered at 4452");
    assert_eq!(
        binds(&mgr, ENTITY),
        vec![(179, vec![5827])],
        "Ogilvie bound"
    );

    close(&engine, &mut mgr, ENTITY, PLAYER, 4983).await;
    assert_eq!(step_of(&mgr, ENTITY), Some(S_DISPENSER));

    let (handled, out) = click_dispenser(&engine, &mut mgr).await;
    assert!(handled);
    assert_eq!(starts(&out).len(), 1, "one Livewire start");
    // `handle_interact` pins the target before the content dispatch.
    mgr.get_entity_mut(ENTITY).unwrap().last_interaction_target = Some(DISPENSER);

    victory(&engine, &mut mgr, ENTITY).await;
    assert_eq!(step_of(&mgr, ENTITY), Some(S_RADIO));
    assert_eq!(offered(&mgr, ENTITY), vec![4984]);

    close(&engine, &mut mgr, ENTITY, PLAYER, 4984).await;
    assert_eq!(offered(&mgr, ENTITY), vec![4984, 4985]);

    close(&engine, &mut mgr, ENTITY, PLAYER, 4985).await;
    let mission = mission_of(&mgr, ENTITY).unwrap();
    assert_eq!(mission.status, MISSION_COMPLETED, "walk ends completed");
    assert_eq!(mission.current_step_id, None, "no step left active");
    for objective in [5171, 5172, 5173, 5174] {
        assert!(
            mission.completed_objectives.contains(&objective),
            "objective {objective} must be credited; got {:?}",
            mission.completed_objectives
        );
    }
    assert!(binds(&mgr, ENTITY).is_empty(), "no bind survives the walk");
}

// ── G12: static guard ────────────────────────────────────────────────────

/// **No `set_interaction_type` anywhere in the 1520 chains, and the only
/// `complete_objective` is 1368's 5171.** A `set_interaction_type` is global
/// (every observer); a stray `complete_objective` on the sole objective of
/// 4453 or 4454 would complete the whole mission mid-list.
#[tokio::test]
async fn live_db_1520_chains_have_no_global_toggle_and_one_complete_objective() {
    let pool = require_db_or_skip!();
    let toggles: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_actions \
         WHERE chain_id BETWEEN 1367 AND 1383 AND action_type = 'set_interaction_type'",
    )
    .fetch_one(&pool)
    .await
    .expect("count toggles");
    assert_eq!(toggles, 0, "1520 binds per player, never globally");

    let credits: Vec<(i32, Option<String>)> = sqlx::query_as(
        "SELECT chain_id, target_key::text FROM resources.content_actions \
         WHERE chain_id BETWEEN 1367 AND 1383 AND action_type = 'complete_objective'",
    )
    .fetch_all(&pool)
    .await
    .expect("list objective credits");
    assert_eq!(
        credits,
        vec![(1368, Some("5171".to_string()))],
        "only 1368's 5171 credit is allowed"
    );
}
