//! Live-DB round trip for mission 62050 "Remaining Pods" (CB-N05-6): the
//! cell's `accept_mission` / `advance_step` / `complete_objective` /
//! `abandon_mission` → `send_mission_update` → the production cell-to-base
//! bridge (`handle_cell_message`) → `sgw_mission` → the re-login query.
//! A one-time mission must survive a relog at each step, must not leak
//! between two characters on one account, and must not be offered again
//! after completion.
//!
//! Beside `mission_abandon_round_trip_tests`, on sentinel IDs that no other
//! test uses (`0x7000_8F00..=0x7000_8F04`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::missions::{
    abandon_mission, accept_mission, advance_step, complete_objective, send_mission_update,
};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{load_mission_defs, load_step_objectives};
use crate::test_support::{require_db_or_skip, TestTransport};

const WORLD: &str = "Castle_CellBlock";
const ACCOUNT_A: i32 = 0x7000_8F00;
const PLAYER_A: i32 = 0x7000_8F01;
const PLAYER_B: i32 = 0x7000_8F02;
const ACCOUNT_C: i32 = 0x7000_8F03;
const PLAYER_C: i32 = 0x7000_8F04;
const ENTITY_ID: u32 = 1;
const MISSION_ID: i32 = 62050;
const STEP_ACCEPT: i32 = 62051;
const STEP_PODS: i32 = 62054;
const OBJ_ACCEPT: i32 = 62052;
const OBJ_MARSH: i32 = 62055;

async fn cleanup(pool: &PgPool, account_id: i32, player_ids: &[i32]) {
    for player_id in player_ids {
        let _ = sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
    }
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_account(pool: &PgPool, account_id: i32) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("mission-62050-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");
}

async fn insert_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', $4, 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("pods-{player_id}"))
    .bind(WORLD)
    .execute(pool)
    .await
    .expect("insert player");
}

/// A fresh cell manager for `player_id` in `WORLD`. The caller loads the
/// real mission defs into `mission_defs`.
fn make_mgr(player_id: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{WORLD}" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#
    ))
    .unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{WORLD}" /></Spaces>"#
    ))
    .unwrap();
    mgr.create_entity(ENTITY_ID, WORLD, [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(ENTITY_ID).unwrap().player_id = Some(player_id);
    mgr
}

fn accept_objectives() -> Vec<MissionObjective> {
    vec![MissionObjective {
        objective_id: OBJ_ACCEPT,
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

/// Save the cell's current state of mission 62050 for `player_id`, then
/// hand the queued updates to the bridge so `sgw_mission` is written.
async fn persist(
    mgr: &SpaceManager,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    db_pool: &Option<Arc<PgPool>>,
) {
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", tx, mgr).await;
    forward(rx, db_pool).await;
}

/// The saved row of mission 62050 for `player_id` as (status, step, repeats).
async fn saved_row(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Option<(i8, Option<i32>, i32)> {
    query_saved_missions(db_pool, player_id)
        .await
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .map(|m| (m.status, m.current_step_id, m.repeats))
}

/// Player A progresses 62050 and relogs; player B on the same account never
/// has a row. After completion a fresh login shows it done and refuses a
/// re-accept (`num_repeats` 0).
#[tokio::test]
async fn live_db_mission_62050_progress_survives_relog_per_character() {
    let pool = require_db_or_skip!();
    cleanup(&pool, ACCOUNT_A, &[PLAYER_A, PLAYER_B]).await;
    insert_account(&pool, ACCOUNT_A).await;
    insert_player(&pool, ACCOUNT_A, PLAYER_A).await;
    insert_player(&pool, ACCOUNT_A, PLAYER_B).await;
    let db_pool = Some(Arc::new(pool.clone()));
    let mut mgr = make_mgr(PLAYER_A);
    mgr.mission_defs = load_mission_defs(&pool).await.unwrap();
    mgr.step_objectives = load_step_objectives(&pool).await.unwrap();
    let (tx, mut rx) = mpsc::channel(64);

    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_ACCEPT,
            accept_objectives(),
            &tx,
            &mut mgr
        )
        .await
    );
    persist(&mgr, PLAYER_A, &tx, &mut rx, &db_pool).await;
    assert_eq!(
        saved_row(&db_pool, PLAYER_A).await,
        Some((MISSION_ACTIVE, Some(STEP_ACCEPT), 0)),
        "accept: A's row is active at 62051"
    );
    assert_eq!(
        saved_row(&db_pool, PLAYER_B).await,
        None,
        "accept: B has no row"
    );

    assert!(advance_step(ENTITY_ID, MISSION_ID, STEP_PODS, &tx, &mut mgr).await);
    persist(&mgr, PLAYER_A, &tx, &mut rx, &db_pool).await;
    assert_eq!(
        saved_row(&db_pool, PLAYER_A).await,
        Some((MISSION_ACTIVE, Some(STEP_PODS), 0)),
        "advance: A's row is active at 62054"
    );
    assert_eq!(
        saved_row(&db_pool, PLAYER_B).await,
        None,
        "advance: B has no row"
    );

    assert!(complete_objective(ENTITY_ID, MISSION_ID, OBJ_MARSH, &tx, &mut mgr).await);
    persist(&mgr, PLAYER_A, &tx, &mut rx, &db_pool).await;
    let done = saved_row(&db_pool, PLAYER_A)
        .await
        .expect("completion: A keeps a row");
    assert_eq!(
        (done.0, done.2),
        (MISSION_COMPLETED, 1),
        "completion: A's row is completed with repeats 1"
    );
    assert_eq!(
        saved_row(&db_pool, PLAYER_B).await,
        None,
        "completion: B has no row"
    );

    // Relog: a fresh manager rebuilt from A's saved rows, as the login does.
    let mut fresh = make_mgr(PLAYER_A);
    fresh.mission_defs = load_mission_defs(&pool).await.unwrap();
    fresh.step_objectives = load_step_objectives(&pool).await.unwrap();
    {
        let saved = query_saved_missions(&db_pool, PLAYER_A).await;
        let row = saved
            .iter()
            .find(|m| m.mission_id == MISSION_ID)
            .expect("the completed row survives the relog");
        let objs: Vec<MissionObjective> = row
            .active_objective_ids
            .iter()
            .map(|&oid| MissionObjective {
                objective_id: oid,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            })
            .collect();
        let mut mi = MissionInstance::new(row.mission_id, row.current_step_id.unwrap_or(0), objs);
        mi.status = row.status;
        mi.completed_steps = row.completed_step_ids.clone();
        mi.completed_objectives = row.completed_objective_ids.clone();
        mi.repeats = row.repeats;
        fresh
            .get_entity_mut(ENTITY_ID)
            .unwrap()
            .missions
            .add_mission(mi);
    }
    assert_eq!(
        fresh
            .get_entity(ENTITY_ID)
            .and_then(|e| e.missions.get_mission(MISSION_ID))
            .map(|m| m.status),
        Some(MISSION_COMPLETED),
        "relog: 62050 shows completed"
    );
    assert!(
        !accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_ACCEPT,
            accept_objectives(),
            &tx,
            &mut fresh
        )
        .await,
        "relog: a completed one-time mission is not offered again"
    );

    cleanup(&pool, ACCOUNT_A, &[PLAYER_A, PLAYER_B]).await;
}

/// Accept and advance to 62054, abandon: the saved row is not active. A
/// re-accept starts at 62051 again.
#[tokio::test]
async fn live_db_mission_62050_abandon_then_reaccept() {
    let pool = require_db_or_skip!();
    cleanup(&pool, ACCOUNT_C, &[PLAYER_C]).await;
    insert_account(&pool, ACCOUNT_C).await;
    insert_player(&pool, ACCOUNT_C, PLAYER_C).await;
    let db_pool = Some(Arc::new(pool.clone()));
    let mut mgr = make_mgr(PLAYER_C);
    mgr.mission_defs = load_mission_defs(&pool).await.unwrap();
    mgr.step_objectives = load_step_objectives(&pool).await.unwrap();
    let (tx, mut rx) = mpsc::channel(64);

    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_ACCEPT,
            accept_objectives(),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(advance_step(ENTITY_ID, MISSION_ID, STEP_PODS, &tx, &mut mgr).await);
    persist(&mgr, PLAYER_C, &tx, &mut rx, &db_pool).await;
    assert_eq!(
        saved_row(&db_pool, PLAYER_C).await,
        Some((MISSION_ACTIVE, Some(STEP_PODS), 0)),
        "precondition: the advance is saved"
    );

    assert!(abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await);
    forward(&mut rx, &db_pool).await;
    assert!(
        saved_row(&db_pool, PLAYER_C)
            .await
            .is_none_or(|r| r.0 != MISSION_ACTIVE),
        "the abandoned mission is not saved active"
    );

    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_ACCEPT,
            accept_objectives(),
            &tx,
            &mut mgr
        )
        .await,
        "re-accept succeeds after an abandon"
    );
    persist(&mgr, PLAYER_C, &tx, &mut rx, &db_pool).await;
    assert_eq!(
        saved_row(&db_pool, PLAYER_C).await,
        Some((MISSION_ACTIVE, Some(STEP_ACCEPT), 0)),
        "re-accept: active again at 62051"
    );

    cleanup(&pool, ACCOUNT_C, &[PLAYER_C]).await;
}
