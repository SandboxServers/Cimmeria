//! Live-DB guards for the Castle side story `repeatable_q1` (Supply Run,
//! mission 10330; CS-05b1c2, seal items 5 and 6).
//!
//! * `live_db_castle_side_repeatable_q1_round_trip`: accept, advance and
//!   complete three times, then the refused fourth accept and the refused
//!   abandon, all go through the cell's `CellToBaseMsg::MissionUpdate` and the
//!   production cell-to-base bridge (`handle_cell_message`) into
//!   `sgw_mission`, and reload per character with `repeats`.
//! * `live_db_castle_side_repeatable_q1_out_of_range_click_refused`: the real
//!   player `interact` method (through the wire dispatcher, the whole seeded
//!   chain engine, the real spawned officer) refuses a far click before any
//!   chain sees it.
//!
//! Beside `castle_side_medical_q1_round_trip_tests`. Sentinel window
//! `0x7000_9A30..=0x7000_9A3F`, the only ids this file writes to the DB. Real
//! ids: mission 10330, steps 10384/10385, objectives 10504/10505, chain 10234.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE,
    STATUS_ACTIVE,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::missions::{
    abandon_mission, accept_mission, advance_step, complete_objective, send_mission_update,
};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_spawns_from_db, load_step_objectives, load_world_rows,
};
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};

const ACCOUNT_A: i32 = 0x7000_9A30;
const PLAYER_A: i32 = 0x7000_9A31;
const ACCOUNT_B: i32 = 0x7000_9A32;
const PLAYER_B: i32 = 0x7000_9A33;
const ENTITY_ID: u32 = 1;
const MISSION_ID: i32 = 10330;
const PREREQ: i32 = 704;
const STEP_A: i32 = 10384;
const STEP_B: i32 = 10385;
const OBJECTIVE_A: i32 = 10504;
const OBJECTIVE_B: i32 = 10505;
/// `num_repeats = 2` in the seed, so the cap admits three completions.
const CAP_RUNS: i32 = 3;
const OFFICER_TAG: &str = "Castle_HubSupplyOfficer";
const OFFER_CHAIN: &str = "10234";
const ON_PLAYER_COMMUNICATION: u16 = 28;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("castle-repeatable-q1-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("repeatable-q1-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// A cell for one character, with the seeded mission, step and objective
/// rows loaded from the live DB (`num_repeats` 2 for 10330).
async fn make_mgr(pool: &PgPool, player_id: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.mission_defs = load_mission_defs(pool).await.unwrap();
    mgr.step_objectives = load_step_objectives(pool).await.unwrap();
    mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(ENTITY_ID).unwrap().player_id = Some(player_id);
    mgr
}

fn step_a_objectives() -> Vec<MissionObjective> {
    vec![MissionObjective {
        objective_id: OBJECTIVE_A,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }]
}

/// Hand every `MissionUpdate` the cell queued to the production bridge, in
/// order, as the base's cell-message loop does.
async fn forward(rx: &mut mpsc::Receiver<CellToBaseMsg>, db_pool: &Option<Arc<PgPool>>) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::MissionUpdate { .. }) {
            handle_cell_message(
                msg,
                &transport,
                &connected,
                &entity_to_addr,
                &cell_tx,
                db_pool,
                &None,
                "127.0.0.1",
                0,
            )
            .await;
        }
    }
}

/// Persist 10330's current cell state through the bridge.
async fn save(
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    mgr: &SpaceManager,
    db_pool: &Option<Arc<PgPool>>,
) {
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", tx, mgr).await;
    forward(rx, db_pool).await;
}

/// One full run, each transition saved as the chain actions save it: accept
/// at step A, advance to B, complete B's objective.
async fn play_run(
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    db_pool: &Option<Arc<PgPool>>,
) {
    assert!(
        accept_mission(ENTITY_ID, MISSION_ID, STEP_A, step_a_objectives(), tx, mgr).await,
        "the accept must succeed under the cap"
    );
    save(player_id, tx, rx, mgr, db_pool).await;
    assert!(advance_step(ENTITY_ID, MISSION_ID, STEP_B, tx, mgr).await);
    save(player_id, tx, rx, mgr, db_pool).await;
    assert!(complete_objective(ENTITY_ID, MISSION_ID, OBJECTIVE_B, tx, mgr).await);
    save(player_id, tx, rx, mgr, db_pool).await;
}

/// `(status, current_step_id, repeats)` of the saved 10330 row, or `None`.
async fn saved_supply(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Option<(i8, Option<i32>, i32)> {
    query_saved_missions(db_pool, player_id)
        .await
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .map(|m| (m.status, m.current_step_id, m.repeats))
}

/// **Supply Run persists through the production dispatch, per character.**
/// Character A completes three runs; after each, the reloaded row is
/// COMPLETED with `repeats` 1, 2, 3 (the next accept is saved ACTIVE and
/// carries the count forward). The fourth accept and an abandon of the
/// COMPLETED record are refused and the saved row stays COMPLETED at 3.
/// Character B completes one run, accepts a second and abandons it in flight:
/// the saved row is NOT_ACTIVE with `repeats` 1, and A's row is untouched.
/// Revert proof: skip the `MissionUpdate` after the objective that completes
/// the run; the reload shows the ACTIVE row with the old count and the
/// COMPLETED assertion fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_round_trip() {
    let pool = require_db_or_skip!();
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
    }
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(128);

    let mut a = make_mgr(&pool, PLAYER_A).await;
    let mut during = Vec::new();
    for run in 1..=CAP_RUNS {
        play_run(PLAYER_A, &tx, &mut rx, &mut a, &db_pool).await;
        during.push((run, saved_supply(&db_pool, PLAYER_A).await));
    }
    assert!(
        !accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_A,
            step_a_objectives(),
            &tx,
            &mut a
        )
        .await,
        "the fourth accept must be refused at the cap"
    );
    forward(&mut rx, &db_pool).await;
    assert!(
        !abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut a).await,
        "a completed record at the cap must not abandon"
    );
    forward(&mut rx, &db_pool).await;
    let after_refusals = saved_supply(&db_pool, PLAYER_A).await;

    let mut b = make_mgr(&pool, PLAYER_B).await;
    play_run(PLAYER_B, &tx, &mut rx, &mut b, &db_pool).await;
    let b_done = saved_supply(&db_pool, PLAYER_B).await;
    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_A,
            step_a_objectives(),
            &tx,
            &mut b
        )
        .await
    );
    save(PLAYER_B, &tx, &mut rx, &b, &db_pool).await;
    let b_active = saved_supply(&db_pool, PLAYER_B).await;
    assert!(abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut b).await);
    forward(&mut rx, &db_pool).await;
    let b_abandoned = saved_supply(&db_pool, PLAYER_B).await;
    let a_last = saved_supply(&db_pool, PLAYER_A).await;
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
    }

    for (run, row) in during {
        assert_eq!(
            row,
            Some((MISSION_COMPLETED, None, run)),
            "after run {run} the reload must show COMPLETED with {run} runs"
        );
    }
    assert_eq!(
        after_refusals,
        Some((MISSION_COMPLETED, None, CAP_RUNS)),
        "the refused accept and abandon must leave the saved row at the cap"
    );
    assert_eq!(
        b_done,
        Some((MISSION_COMPLETED, None, 1)),
        "character B's first run is saved on its own player id"
    );
    assert_eq!(
        b_active,
        Some((MISSION_ACTIVE, Some(STEP_A), 1)),
        "the second accept is saved ACTIVE and carries the count forward"
    );
    assert_eq!(
        b_abandoned,
        Some((MISSION_NOT_ACTIVE, None, 1)),
        "an abandoned attempt reloads not active with the count kept"
    );
    assert_eq!(
        a_last,
        Some((MISSION_COMPLETED, None, CAP_RUNS)),
        "character B's saves must not touch character A's row"
    );
}

// ── The range gate ───────────────────────────────────────────────────────

/// The Castle with the seeded Supply Officer spawned from its record, and the
/// player `dist` metres from it holding a completed 704 and a COMPLETED 10330
/// with two runs (one more is allowed). Returns the manager, the officer's
/// entity id and the production chain engine.
async fn stage_officer(
    pool: &PgPool,
    dist: f32,
) -> (
    SpaceManager,
    u32,
    cimmeria_content_engine::chain::ChainEngine,
) {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<Spaces><Space WorldName="Castle" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<Spaces><Space WorldName="Castle" /></Spaces>"#)
        .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world rows"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission defs");
    mgr.step_objectives = load_step_objectives(pool).await.expect("step objectives");
    mgr.item_containers = load_item_containers(pool).await.expect("item containers");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool)
        .await
        .expect("monologue ids");

    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("spawns must load")
        .into_iter()
        .filter(|r| r.tag.as_deref() == Some(OFFICER_TAG))
        .collect();
    assert_eq!(
        records.len(),
        1,
        "one spawn carries {OFFICER_TAG}: {records:?}"
    );
    let record = &records[0];
    assert_eq!(record.world_name, "Castle");
    let officer = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(officer, record)
        .expect("the officer must spawn from its record");

    mgr.create_entity(
        ENTITY_ID,
        "Castle",
        [record.x + dist, record.y, record.z],
        [0.0; 3],
    )
    .expect("the player must stage in the Castle");
    mgr.connect_entity(ENTITY_ID);
    let player = mgr.get_entity_mut(ENTITY_ID).unwrap();
    player.is_player = true;
    player.player_id = Some(PLAYER_A);
    player.archetype_id = Some(1);
    let mut prereq = MissionInstance::new(PREREQ, 0, Vec::new());
    prereq.status = MISSION_COMPLETED;
    prereq.current_step_id = None;
    player.missions.add_mission(prereq);
    let mut runs = MissionInstance::new(MISSION_ID, 0, Vec::new());
    runs.status = MISSION_COMPLETED;
    runs.current_step_id = None;
    runs.repeats = CAP_RUNS - 1;
    player.missions.add_mission(runs);

    let engine = crate::cell::content::build_engine(Some(pool)).await;
    (mgr, officer, engine)
}

/// Right-click the officer through the wire dispatcher; everything it sent.
async fn click(
    mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
    officer: u32,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            ENTITY_ID,
            INTERACT,
            &(officer as i32).to_le_bytes(),
            &tx,
            mgr,
            engine,
        )
        .await
    );
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

fn supply_state(mgr: &SpaceManager) -> Option<(i8, i32)> {
    mgr.get_entity(ENTITY_ID)
        .unwrap()
        .missions
        .get_mission(MISSION_ID)
        .map(|m| (m.status, m.repeats))
}

/// **A Supply Officer click from 30 u reaches no chain; the same click from
/// 1 u does.** Far: exactly one `onPlayerCommunication`, exactly one INFO
/// `interaction.out_of_range` row, no log row of any kind for chain 10234, and
/// 10330 still COMPLETED with two runs. Near (control): chain 10234 resolves
/// and accepts the third run (ACTIVE, still two completed runs carried), and
/// no out-of-range row is logged. Revert proof: delete the
/// `interact_target_in_range` gate in `interact.rs`; the far click fires
/// 10234 and the far case fails.
#[tokio::test]
async fn live_db_castle_side_repeatable_q1_out_of_range_click_refused() {
    let pool = require_db_or_skip!();

    // Far.
    let (mut mgr, officer, engine) = stage_officer(&pool, 30.0).await;
    let capture = LogCapture::install();
    let out = click(&mut mgr, &engine, officer).await;
    let calls: Vec<u16> = out
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => Some(*method_index),
            _ => None,
        })
        .collect();
    assert_eq!(
        calls,
        vec![ON_PLAYER_COMMUNICATION],
        "a too-far click gets exactly one chat line and nothing else: {out:?}"
    );
    let all = capture.all();
    let refused: Vec<_> = all
        .iter()
        .filter(|c| c.level == Level::INFO && c.has_field("event", "interaction.out_of_range"))
        .collect();
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(refused[0].has_field("reason", "too_far"), "{refused:#?}");
    let chain_rows: Vec<_> = all
        .iter()
        .filter(|c| c.has_field("chain_id", OFFER_CHAIN))
        .collect();
    assert!(
        chain_rows.is_empty(),
        "a refused click must not reach chain {OFFER_CHAIN}: {chain_rows:#?}"
    );
    assert!(
        !out.iter()
            .any(|m| matches!(m, CellToBaseMsg::MissionUpdate { .. })),
        "a refused click must persist nothing: {out:?}"
    );
    assert_eq!(
        supply_state(&mgr),
        Some((MISSION_COMPLETED, CAP_RUNS - 1)),
        "a far click must leave status and repeats unchanged"
    );
    drop(capture);

    // Control: one unit away.
    let (mut mgr, officer, engine) = stage_officer(&pool, 1.0).await;
    let capture = LogCapture::install();
    click(&mut mgr, &engine, officer).await;
    let all = capture.all();
    assert!(
        !all.iter()
            .any(|c| c.has_field("event", "interaction.out_of_range")),
        "the in-range click must not be refused for range"
    );
    assert!(
        all.iter().any(|c| c.has_field("chain_id", OFFER_CHAIN)),
        "the in-range click must reach chain {OFFER_CHAIN} (fired or condition_failed)"
    );
    assert_eq!(
        supply_state(&mgr),
        Some((MISSION_ACTIVE, CAP_RUNS - 1)),
        "the in-range click must accept the third run with the count carried"
    );
}
