//! Mission 1664's Castle grant, chain 1398 (FZ-18): the frozen seed rows and
//! the real `mission_completed` dispatcher. No chain 1399 exists (O18-02).
//!
//! The harness is copied from [`super::mission_567_castle_lifecycle`] rather
//! than shared, so a change to one file cannot move the other's assertions.
//! The chain ships enabled; this file loads it as shipped.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::fire_mission_completed;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_step_objectives, load_world_rows,
};
use crate::test_support::require_db_or_skip;

const ENTITY: u32 = 1;
const PLAYER: i32 = 42;
const TARGET: u32 = 3;
const MISSION: i32 = 1664;
const STEP_KAERLIA: i32 = 5224;
const CASTLE: &str = "Castle";
const HARSET: &str = "Harset";
const CHAIN: i32 = 1398;

// ── Harness ──────────────────────────────────────────────────────────────

/// An engine holding `ids`, loaded as shipped.
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

/// `world` with player `ENTITY` holding `missions`, with the dialog-set and
/// monologue caches loaded from the seed.
async fn staged(pool: &PgPool, world: &str, missions: Vec<MissionInstance>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let space = format!(
        r#"<Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />"#
    );
    mgr.parse_spaces_xml(&format!("<Spaces>{space}</Spaces>"))
        .unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world cache"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission cache");
    mgr.step_objectives = load_step_objectives(pool).await.expect("objective cache");
    mgr.item_containers = load_item_containers(pool).await.expect("item cache");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");
    mgr.create_entity(TARGET, world, [3.0; 3], [0.0; 3])
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

/// A completed mission (the grant gate 708, or 1664 itself).
fn completed(mission_id: i32) -> MissionInstance {
    let mut mission = MissionInstance::new(mission_id, 0, Vec::new());
    mission.status = MISSION_COMPLETED;
    mission.current_step_id = None;
    mission
}

/// 1664 active at `step` with `objective` active.
fn mission_1664(step: i32, objective: i32) -> MissionInstance {
    let objective = MissionObjective {
        objective_id: objective,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    };
    let mut mission = MissionInstance::new(MISSION, step, vec![objective]);
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

/// Fire one event against the stage and return everything sent to the
/// cell-to-base channel, as a compact trace in send order.
async fn traffic(fire: impl AsyncFnOnce(&mpsc::Sender<CellToBaseMsg>)) -> Vec<String> {
    let (tx, mut rx) = mpsc::channel(256);
    fire(&tx).await;
    let mut trace: Vec<String> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let label = match msg {
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION,
                status,
                current_step_id,
                ..
            } => format!("1664 {status} {current_step_id:?}"),
            _ => continue,
        };
        trace.push(label);
    }
    trace
}

/// The `mission_completed 708` event in Castle's player context.
async fn complete_708(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<String> {
    traffic(async |tx| fire_mission_completed(ENTITY, PLAYER, 708, engine, tx, mgr).await).await
}

// ── Seed ─────────────────────────────────────────────────────────────────

/// **The chain 1398 seed rows are the frozen evidence rows.** Enabled; one
/// `mission_completed 708` trigger; the world-8 gate and the two mission
/// status gates; one immediate `accept_mission 1664`. No chain 1399 exists.
/// Revert proof: delete the `(1398, 'world', 8, ...)` tuple and the conditions
/// assertion fails; insert a chain 1399 and the count assertion fails.
#[tokio::test]
async fn live_db_chain_1398_seed_rows_are_frozen() {
    let pool = require_db_or_skip!();

    let (enabled,): (bool,) =
        sqlx::query_as("SELECT enabled FROM resources.content_chains WHERE chain_id = $1")
            .bind(CHAIN)
            .fetch_one(&pool)
            .await
            .expect("chain 1398 must be seeded");
    assert!(enabled, "chain 1398 must ship enabled");

    let triggers: Vec<(String, Option<String>, bool)> = sqlx::query_as(
        "SELECT event_type, event_key, once FROM resources.content_triggers \
         WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(CHAIN)
    .fetch_all(&pool)
    .await
    .expect("triggers query");
    assert_eq!(
        triggers,
        vec![(
            "mission_completed".to_string(),
            Some("708".to_string()),
            false
        )],
        "chain 1398 trigger rows"
    );

    let conditions: Vec<(String, Option<i32>, Option<String>, String, Option<String>)> =
        sqlx::query_as(
            "SELECT condition_type, target_id, target_key, operator, value \
             FROM resources.content_conditions WHERE chain_id = $1 ORDER BY sort_order",
        )
        .bind(CHAIN)
        .fetch_all(&pool)
        .await
        .expect("conditions query");
    assert_eq!(
        conditions,
        vec![
            ("world".to_string(), Some(8), None, "eq".to_string(), None),
            (
                "mission_status".to_string(),
                Some(708),
                None,
                "eq".to_string(),
                Some("completed".to_string()),
            ),
            (
                "mission_status".to_string(),
                Some(1664),
                None,
                "eq".to_string(),
                Some("not_active".to_string()),
            ),
        ],
        "chain 1398 condition rows"
    );

    let actions: Vec<(String, Option<i32>, i32)> = sqlx::query_as(
        "SELECT action_type, target_id, delay_ms FROM resources.content_actions \
         WHERE chain_id = $1 ORDER BY sort_order",
    )
    .bind(CHAIN)
    .fetch_all(&pool)
    .await
    .expect("actions query");
    assert_eq!(
        actions,
        vec![("accept_mission".to_string(), Some(1664), 0)],
        "chain 1398 action rows"
    );

    let (absent,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM resources.content_chains WHERE chain_id = 1399")
            .fetch_one(&pool)
            .await
            .expect("count query");
    assert_eq!(
        absent, 0,
        "chain 1399 must not exist: O18-02 omits it and forbids a re-grant of 1664 after abandonment"
    );
}

// ── Grant ────────────────────────────────────────────────────────────────

/// **708 completed in Castle accepts 1664 once, and a second 708 grants
/// nothing.** The first event puts 1664 at 5224 and echoes it to the base; the
/// second finds 1664 active and `accept_mission`'s offer guard refuses it
/// (lifecycle.rs:89-91), so this half passes with or without the `not_active`
/// gate. Revert proof: delete the `accept_mission` action and 1664 never appears (the
/// first assertion fails); change the trigger key from 708 and the event
/// grants nothing.
#[tokio::test]
async fn live_db_chain_1398_grants_1664_once_when_708_completes() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[CHAIN]).await;
    let mut mgr = staged(&pool, CASTLE, vec![completed(708)]).await;

    let out = complete_708(&engine, &mut mgr).await;

    let mission = mission_of(&mgr).expect("1664 must be accepted by 708 in Castle");
    assert_eq!(
        mission.current_step_id,
        Some(STEP_KAERLIA),
        "1664 must start at 5224"
    );
    assert_ne!(mission.status, MISSION_COMPLETED, "1664 must be running");
    assert!(
        out.iter().any(|l| l.starts_with("1664 ")),
        "the grant must reach the base; got {out:?}"
    );

    let again = complete_708(&engine, &mut mgr).await;
    assert!(
        again.is_empty(),
        "a second 708 must send nothing; got {again:?}"
    );
    assert_eq!(
        step_of(&mgr),
        Some(STEP_KAERLIA),
        "the step must hold at 5224"
    );
}

// ── Refusals ─────────────────────────────────────────────────────────────

/// **708 grants 1664 only in Castle, only once 708 is completed, and only
/// while 1664 has not started.** Each refused case sends nothing and leaves
/// the 1664 state as it was. Revert proof: drop the world-8 condition and the
/// Harset case grants; drop the `mission_status 1664 not_active` condition
/// and the completed case re-accepts 1664 (the offer guard refuses an ACTIVE
/// 1664 anyway, so the running case does not prove that condition); drop the
/// 708 completed condition and the not-completed case grants.
#[tokio::test]
async fn live_db_chain_1398_refuses_outside_its_gate() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[CHAIN]).await;
    let cases: [(&str, &str, Vec<MissionInstance>); 4] = [
        ("708 not completed", CASTLE, vec![]),
        ("in Harset", HARSET, vec![completed(708)]),
        (
            "1664 active",
            CASTLE,
            vec![completed(708), mission_1664(5225, 6060)],
        ),
        (
            "1664 completed",
            CASTLE,
            vec![completed(708), completed(MISSION)],
        ),
    ];
    for (label, world, missions) in cases {
        let mut mgr = staged(&pool, world, missions).await;
        let before = mission_of(&mgr).map(|m| (m.current_step_id, m.status));

        let out = complete_708(&engine, &mut mgr).await;

        assert!(
            !out.iter().any(|l| l.starts_with("1664 ")),
            "{label}: 1664 must not be granted; got {out:?}"
        );
        assert_eq!(
            mission_of(&mgr).map(|m| (m.current_step_id, m.status)),
            before,
            "{label}: 1664 must not move"
        );
    }
}
