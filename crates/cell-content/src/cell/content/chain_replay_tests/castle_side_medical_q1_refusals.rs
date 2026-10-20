//! Castle Q1 medical side story (mission 10300), CS-04a1c4: what the chains
//! 10000-10006 must NOT do. The accept and the happy path are in
//! `castle_side_medical_q1`; this file pins the refusals.
//!
//! * the medic click is refused in the wrong world and without 704
//!   (`live_db_castle_side_medical_q1_wrong_world_and_missing_704_refused`);
//! * a forged dialog close, button or open of dialog 10700 moves nothing
//!   (`live_db_castle_side_medical_q1_forged_close_is_inert`);
//! * no event key resolves two of the seven chains in any reachable
//!   704/10300 state (`live_db_castle_side_medical_q1_siblings_disjoint`).
//!
//! The range refusal needs the real `interact` method, which lives in
//! `cimmeria-cell-methods`; its test is `castle_medic_range_tests` there.
//!
//! Every click goes through the real dispatchers, so a dispatcher that skips
//! `populate_world_context` or the mission context fails here, not only a
//! missing condition row.

use std::collections::BTreeSet;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::mission_context::{populate_mission_context, populate_world_context};
use super::super::{fire_dialog_choice, fire_dialog_open, fire_interact_tag, fire_player_loaded};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::{require_db_or_skip, LogCapture};

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const MEDIC: u32 = 3;
const CASTLE: &str = "Castle";
/// `resources.worlds` row 1; any world other than 8 refuses the medic.
const COMBAT_SIM: &str = "CombatSim";
const MEDIC_TAG: &str = "Castle_Pop_Infirmary_Medic1";
const COURTYARD: &str = "Castle.CastleFrontCourtyard";
const MISSION: i32 = 10300;
const PREREQ: i32 = 704;
const STEP_A: i32 = 10340;
const STEP_B: i32 = 10341;
const OBJ_A: i32 = 10460;
const OBJ_B: i32 = 10461;
const OFFER_SLOT: i32 = 177;
const DIALOG: i32 = 10700;
/// The accept chain, the only one on the medic tag.
const ACCEPT_CHAIN: i64 = 10003;
const ALL_CHAINS: [i32; 7] = [10000, 10001, 10002, 10003, 10004, 10005, 10006];

// ── Harness ──────────────────────────────────────────────────────────────

async fn engine(pool: &PgPool) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for id in ALL_CHAINS {
        let chain = load_single_chain_for_test(pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

/// The player (and the medic NPC) in `world`, holding `missions`, with the
/// production caches loaded from the seed.
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
    mgr.create_entity(MEDIC, world, [3.0; 3], [0.0; 3]).unwrap();
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

/// 704 as the player holds it, or not at all.
#[derive(Clone, Copy, Debug)]
enum Prereq {
    Absent,
    Active,
    Completed,
}

/// 10300 as the player holds it.
#[derive(Clone, Copy, Debug)]
enum Q1 {
    Absent,
    AtA,
    AtB,
    Done,
}

fn missions(prereq: Prereq, q1: Q1) -> Vec<MissionInstance> {
    let mut out = Vec::new();
    match prereq {
        Prereq::Absent => {}
        Prereq::Active => out.push(MissionInstance::new(PREREQ, 0, Vec::new())),
        Prereq::Completed => {
            let mut done = MissionInstance::new(PREREQ, 0, Vec::new());
            done.status = MISSION_COMPLETED;
            done.current_step_id = None;
            out.push(done);
        }
    }
    let at = |step: i32, objective: i32| {
        let mut mission = MissionInstance::new(
            MISSION,
            step,
            vec![MissionObjective {
                objective_id: objective,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        mission.current_step_id = Some(step);
        mission
    };
    match q1 {
        Q1::Absent => {}
        Q1::AtA => out.push(at(STEP_A, OBJ_A)),
        Q1::AtB => out.push(at(STEP_B, OBJ_B)),
        Q1::Done => {
            let mut done = at(STEP_B, OBJ_B);
            done.status = MISSION_COMPLETED;
            done.current_step_id = None;
            out.push(done);
        }
    }
    out
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Everything the Q1 story can change on the player, as one comparable value:
/// 10300's status and step, the offer binding and the dialogs on screen.
fn snapshot(mgr: &SpaceManager) -> (Option<(i8, Option<i32>)>, Vec<i32>, Vec<i32>) {
    let entity = mgr.get_entity(ENTITY).unwrap();
    let q1 = entity
        .missions
        .get_mission(MISSION)
        .map(|m| (m.status, m.current_step_id));
    let mut bound: Vec<i32> = entity
        .available_interactions
        .get(&OFFER_SLOT)
        .map(|v| v.iter().map(|&(id, _, _)| id).collect())
        .unwrap_or_default();
    bound.sort_unstable();
    (q1, bound, entity.offered_dialogs())
}

/// Click the medic; returns whether a chain handled it, and the traffic.
async fn click_medic(engine: &ChainEngine, mgr: &mut SpaceManager) -> (bool, Vec<CellToBaseMsg>) {
    let (tx, mut rx) = mpsc::channel(256);
    let handled = fire_interact_tag(ENTITY, PLAYER, MEDIC_TAG, MEDIC, engine, &tx, mgr).await;
    (handled, drain(&mut rx))
}

// ── Wrong world, missing 704 ─────────────────────────────────────────────

/// **The medic click accepts only in world 8 with 704 completed.** A
/// positive control (world 8, 704 completed, 10300 not active) is handled by
/// chain 10003, so the refusals below cannot pass vacuously. Then the same
/// click in world 1, in the Castle with 704 active and in the Castle with no
/// 704 at all is handled by nothing, sends nothing, leaves 10300 unstarted,
/// and logs a `condition_failed` row for chain 10003 naming the failing
/// condition (0 = world, 1 = `mission_status 704`). Revert proof: drop
/// 10003's `world` condition and the world-1 case is handled.
#[tokio::test]
async fn live_db_castle_side_medical_q1_wrong_world_and_missing_704_refused() {
    let pool = require_db_or_skip!();
    let engine = engine(&pool).await;

    let mut mgr = staged(&pool, CASTLE, missions(Prereq::Completed, Q1::Absent)).await;
    let (handled, out) = click_medic(&engine, &mut mgr).await;
    assert!(
        handled,
        "control: chain 10003 must accept in world 8 after 704"
    );
    assert!(
        out.iter().any(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                ..
            }
        )),
        "control: the accept must persist 10300"
    );

    // (world, 704, index of the condition that must fail)
    let refused = [
        (COMBAT_SIM, Prereq::Completed, 0),
        (CASTLE, Prereq::Active, 1),
        (CASTLE, Prereq::Absent, 1),
    ];
    for (world, prereq, failed_index) in refused {
        let capture = LogCapture::install();
        let mut mgr = staged(&pool, world, missions(prereq, Q1::Absent)).await;
        let want_world = if world == CASTLE { 8 } else { 1 };
        assert_eq!(
            mgr.get_entity_world_id(ENTITY),
            Some(want_world),
            "{world}: the player must be staged in world {want_world}"
        );
        let (handled, out) = click_medic(&engine, &mut mgr).await;
        assert!(
            !handled,
            "{world} with 704 {prereq:?}: no chain may handle the medic click"
        );
        assert!(
            !out.iter().any(|m| matches!(
                m,
                CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
            )),
            "{world} with 704 {prereq:?}: no mission or dialog traffic; got {out:?}"
        );
        assert!(
            mgr.get_entity(ENTITY)
                .unwrap()
                .missions
                .get_mission(MISSION)
                .is_none(),
            "{world} with 704 {prereq:?}: 10300 must not start"
        );
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::DEBUG
                    && c.target == "content.resolve"
                    && c.has_field("chain_id", &ACCEPT_CHAIN.to_string())
            })
            .collect();
        assert_eq!(rows.len(), 1, "{world} with 704 {prereq:?}: {rows:#?}");
        assert!(rows[0].has_field("reason", "condition_failed"), "{rows:#?}");
        assert!(
            rows[0].has_field("failed_condition_index", &failed_index.to_string()),
            "{world} with 704 {prereq:?}: condition {failed_index} must be the one that fails; {rows:#?}"
        );
    }
}

// ── Forged close ─────────────────────────────────────────────────────────

/// **A forged close, button or open of dialog 10700 moves nothing.** The seed
/// keys no chain on dialog 10700 (no `dialog_choice` or `dialog_open` trigger
/// with that key, and none at all under scope 10300), and the real
/// dispatchers confirm it in every Q1 state: the close (`button -1`), a
/// button ordinal and the open that an `initialResponse` ends in send no
/// traffic and leave 10300, the offer binding and the dialogs on screen as
/// they were. Revert proof: add a `dialog_choice` trigger for 10700 to any
/// 10300 chain and the seed count fails.
#[tokio::test]
async fn live_db_castle_side_medical_q1_forged_close_is_inert() {
    let pool = require_db_or_skip!();
    let keyed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.content_triggers t \
         JOIN resources.content_chains c ON c.chain_id = t.chain_id \
         WHERE (t.event_type IN ('dialog_choice', 'dialog_open') AND t.event_key = $1) \
            OR (c.scope_type = 'mission' AND c.scope_id = $2 \
                AND t.event_type IN ('dialog_choice', 'dialog_open', 'dialog_set_open'))",
    )
    .bind(DIALOG.to_string())
    .bind(MISSION)
    .fetch_one(&pool)
    .await
    .expect("dialog trigger query must succeed");
    assert_eq!(
        keyed, 0,
        "no chain may key on dialog {DIALOG} or on a dialog event under scope {MISSION}"
    );

    let engine = engine(&pool).await;
    for (prereq, q1) in [
        (Prereq::Completed, Q1::Absent),
        (Prereq::Completed, Q1::AtA),
        (Prereq::Completed, Q1::AtB),
        (Prereq::Completed, Q1::Done),
    ] {
        let mut mgr = staged(&pool, CASTLE, missions(prereq, q1)).await;
        if matches!(q1, Q1::Absent) {
            let (tx, _rx) = mpsc::channel(256);
            fire_player_loaded(ENTITY, PLAYER, CASTLE, &engine, &tx, &mut mgr).await;
        }
        let before = snapshot(&mgr);
        let (tx, mut rx) = mpsc::channel(256);
        for button in [-1, 0, 1] {
            fire_dialog_choice(ENTITY, PLAYER, DIALOG, button, &engine, &tx, &mut mgr).await;
        }
        fire_dialog_open(ENTITY, PLAYER, DIALOG, &engine, &tx, &mut mgr).await;
        let out = drain(&mut rx);
        assert!(
            out.is_empty(),
            "{q1:?}: forged dialog events must send nothing; got {out:?}"
        );
        assert_eq!(
            snapshot(&mgr),
            before,
            "{q1:?}: forged dialog events moved state"
        );
    }
}

// ── Sibling disjointness ─────────────────────────────────────────────────

/// The six event keys the seven chains listen on, as the dispatchers build
/// them: the trigger type and the one param that carries the key.
fn event_keys() -> [(TriggerType, Option<(&'static str, serde_json::Value)>); 6] {
    [
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
            Some(("entity_tag", MEDIC_TAG.into())),
        ),
        (
            TriggerType::MissionAccepted,
            Some(("mission_id", MISSION.into())),
        ),
        (
            TriggerType::RegionEnter,
            Some(("region_key", COURTYARD.into())),
        ),
    ]
}

/// **For each event key and every reachable 704/10300 state in the Castle, at
/// most one of chains 10000-10006 resolves; across all of them every chain
/// resolves somewhere.** The contexts are built as the dispatchers build
/// them (world, mission state, archetype, the key param), so this exercises
/// the shipped conditions, not a hand-made context. The second half stops a
/// loader that drops a chain from passing the first half vacuously. Revert
/// proof: change 10001's `mission_status 10300` from `not_active` to
/// `active` and 10001 and 10006 both resolve on `player_loaded` at step A.
#[tokio::test]
async fn live_db_castle_side_medical_q1_siblings_disjoint() {
    let pool = require_db_or_skip!();
    let engine = engine(&pool).await;
    let mut seen: BTreeSet<i64> = BTreeSet::new();
    for prereq in [Prereq::Absent, Prereq::Active, Prereq::Completed] {
        for q1 in [Q1::Absent, Q1::AtA, Q1::AtB, Q1::Done] {
            let mgr = staged(&pool, CASTLE, missions(prereq, q1)).await;
            for (trigger_type, key) in event_keys() {
                let mut ctx =
                    ExecutionContext::new().with_source(cimmeria_common::EntityId(ENTITY as i32));
                if let Some((name, value)) = key {
                    ctx.set_param(name.to_string(), value);
                }
                populate_world_context(ENTITY, &mgr, &mut ctx);
                let entity = mgr.get_entity(ENTITY).unwrap();
                populate_mission_context(entity, &mut ctx);
                ctx.set_param("archetype".to_string(), serde_json::json!(1));
                let event = TriggerEvent {
                    trigger_type: trigger_type.clone(),
                    source_entity: Some(cimmeria_common::EntityId(ENTITY as i32)),
                    target_entity: None,
                    params: ctx.params.clone(),
                };
                let fired: BTreeSet<i64> = engine
                    .resolve_event(&event, &ctx)
                    .actions
                    .iter()
                    .map(|(id, _)| *id)
                    .collect();
                assert!(
                    fired.len() <= 1,
                    "{trigger_type:?} with 704 {prereq:?}, 10300 {q1:?} resolved {fired:?}; \
                     sibling chains must be pairwise disjoint on the 10300 status"
                );
                seen.extend(fired);
            }
        }
    }
    let all: BTreeSet<i64> = ALL_CHAINS.iter().map(|&id| id as i64).collect();
    assert_eq!(
        seen, all,
        "every Q1 chain must resolve in some reachable state"
    );
}
