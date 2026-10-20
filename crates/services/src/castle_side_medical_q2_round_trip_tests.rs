//! Live-DB guards for the Castle side story `medical_q2` (CS-04a2c2, seal
//! `CS-04aD2` c2 item 7): accept, advance, complete and abandon go through the
//! cell's `CellToBaseMsg::MissionUpdate` and the production cell-to-base bridge
//! (`handle_cell_message`) into `sgw_mission`, and reload per character.
//!
//! Beside `castle_side_medical_q1_round_trip_tests`. Sentinel window
//! `0x7000_9A10..=0x7000_9A1F`, the only ids this file writes, each cleaned by
//! exact id. Real ids: mission 10301, steps 10342/10343, objectives
//! 10462/10463.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE};
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

const ACCOUNT_A: i32 = 0x7000_9A10;
const PLAYER_A: i32 = 0x7000_9A11;
const ACCOUNT_B: i32 = 0x7000_9A12;
const PLAYER_B: i32 = 0x7000_9A13;
const ACCOUNT_C: i32 = 0x7000_9A14;
const PLAYER_C: i32 = 0x7000_9A15;
const ENTITY_ID: u32 = 1;
const MISSION_ID: i32 = 10301;
const STEP_A: i32 = 10342;
const STEP_B: i32 = 10343;
const OBJECTIVE_A: i32 = 10462;
const OBJECTIVE_B: i32 = 10463;

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
        .bind(format!("castle-medical-q2-{account_id}"))
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
    .bind(format!("medical-q2-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// A cell for one character, with the seeded mission and step rows loaded
/// from the live DB (`resources.missions`, `mission_steps`, `mission_objectives`).
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

fn step_a_objectives() -> Vec<cimmeria_entity::missions::MissionObjective> {
    vec![cimmeria_entity::missions::MissionObjective {
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

/// Accept at step A, advance to B, complete the medic objective, then let
/// character B accept, advance and abandon. Each character's saved rows
/// reload on its own player id. Skipping the save after A's advance leaves the
/// row at 10342 and the mid-way check fails; skipping A's completion save
/// leaves it active at 10343 and the completed check fails.
#[tokio::test]
async fn live_db_castle_side_medical_q2_round_trip_persists_per_character() {
    let pool = require_db_or_skip!();
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
    }
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(128);

    let mut a = make_mgr(&pool, PLAYER_A).await;
    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_A,
            step_a_objectives(),
            &tx,
            &mut a
        )
        .await
    );
    send_mission_update(ENTITY_ID, PLAYER_A, MISSION_ID, "test", &tx, &a).await;
    forward(&mut rx, &db_pool).await;
    assert!(advance_step(ENTITY_ID, MISSION_ID, STEP_B, &tx, &mut a).await);
    send_mission_update(ENTITY_ID, PLAYER_A, MISSION_ID, "test", &tx, &a).await;
    forward(&mut rx, &db_pool).await;
    let midway_a = query_saved_missions(&db_pool, PLAYER_A).await;
    assert!(complete_objective(ENTITY_ID, MISSION_ID, OBJECTIVE_B, &tx, &mut a).await);
    send_mission_update(ENTITY_ID, PLAYER_A, MISSION_ID, "test", &tx, &a).await;
    forward(&mut rx, &db_pool).await;
    // Read A's row before B touches anything, so only A's own forward can have
    // saved it (B's forward would otherwise drain A's queued update).
    let saved_a = query_saved_missions(&db_pool, PLAYER_A).await;

    let mut b = make_mgr(&pool, PLAYER_B).await;
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
    send_mission_update(ENTITY_ID, PLAYER_B, MISSION_ID, "test", &tx, &b).await;
    forward(&mut rx, &db_pool).await;
    assert!(advance_step(ENTITY_ID, MISSION_ID, STEP_B, &tx, &mut b).await);
    send_mission_update(ENTITY_ID, PLAYER_B, MISSION_ID, "test", &tx, &b).await;
    forward(&mut rx, &db_pool).await;
    let midway_b = query_saved_missions(&db_pool, PLAYER_B).await;
    assert!(abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut b).await);
    forward(&mut rx, &db_pool).await;

    let saved_b = query_saved_missions(&db_pool, PLAYER_B).await;
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
    }

    for (who, rows) in [("A", &midway_a), ("B", &midway_b)] {
        let row = rows
            .iter()
            .find(|m| m.mission_id == MISSION_ID)
            .unwrap_or_else(|| panic!("{who}: the advance must be saved"));
        assert_eq!(
            (row.status, row.current_step_id),
            (MISSION_ACTIVE, Some(STEP_B)),
            "{who}: saved active at step B after the advance"
        );
        assert!(
            row.completed_step_ids.contains(&STEP_A),
            "{who}: step A is recorded complete: {:?}",
            row.completed_step_ids
        );
    }
    let row_a = saved_a
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("character A keeps its completed row");
    assert_eq!(
        row_a.status, MISSION_COMPLETED,
        "A's medical_q2 is completed"
    );
    assert!(
        saved_b.iter().all(|m| m.mission_id != MISSION_ID),
        "B's abandoned medical_q2 must not reload: {:?}",
        saved_b
            .iter()
            .map(|m| (m.mission_id, m.status))
            .collect::<Vec<_>>()
    );
}
/// Accept and advance to step B, persist, reload: B is the active step and A
/// is recorded complete, so a relog at B restores B with no rebind.
#[tokio::test]
async fn live_db_castle_side_medical_q2_relog_at_step_b_restores_b() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (ACCOUNT_C, PLAYER_C);
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(128);

    let mut mgr = make_mgr(&pool, player_id).await;
    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_A,
            step_a_objectives(),
            &tx,
            &mut mgr
        )
        .await
    );
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", &tx, &mgr).await;
    forward(&mut rx, &db_pool).await;
    assert!(advance_step(ENTITY_ID, MISSION_ID, STEP_B, &tx, &mut mgr).await);
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", &tx, &mgr).await;
    forward(&mut rx, &db_pool).await;

    let saved = query_saved_missions(&db_pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    let row = saved
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("the accepted mission keeps its row");
    assert_eq!(
        (row.status, row.current_step_id),
        (MISSION_ACTIVE, Some(STEP_B)),
        "active at step B after relog"
    );
    assert!(
        row.completed_step_ids.contains(&STEP_A),
        "step A is recorded complete: {:?}",
        row.completed_step_ids
    );
}
