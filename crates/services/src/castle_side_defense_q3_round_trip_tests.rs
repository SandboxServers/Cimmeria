//! Live-DB guard for the Castle side story `defense_q3` (CS-04b3c2, seal c2
//! test 8): accept, kills, relog, kills and completion go through the cell's
//! `CellToBaseMsg::MissionUpdate` and the production cell-to-base bridge
//! (`handle_cell_message`) into `sgw_mission`, and reload per character
//! through the production hydrator.
//!
//! Beside `castle_side_medical_q1_round_trip_tests`. Sentinel window
//! `0x7000_9D00..=0x7000_9D0F`, the only ids this file writes. Real ids:
//! mission 10312, step 10366, objectives 10486-10489.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_entity::missions::{
    MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::missions::{accept_mission, complete_objective, send_mission_update};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{load_mission_defs, load_step_objectives};
use crate::test_support::{require_db_or_skip, TestTransport};

const ACCOUNT_A: i32 = 0x7000_9D00;
const PLAYER_A: i32 = 0x7000_9D01;
const ACCOUNT_B: i32 = 0x7000_9D02;
const PLAYER_B: i32 = 0x7000_9D03;
const ENTITY_ID: u32 = 1;
const MISSION_ID: i32 = 10312;
const STEP_ID: i32 = 10366;
/// One objective per Bravo officer and Muelbach, in the seal's order.
const OBJECTIVES: [i32; 4] = [10486, 10487, 10488, 10489];

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
        .bind(format!("castle-defense-q3-{account_id}"))
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
    .bind(format!("defense-q3-{player_id}"))
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

/// The four required objectives of step 10366, all active, as the accept seeds.
fn step_objectives() -> Vec<MissionObjective> {
    OBJECTIVES
        .iter()
        .map(|&objective_id| MissionObjective {
            objective_id,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        })
        .collect()
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

/// Complete `objective_id` on the cell and save it through the bridge, as the
/// `complete_objective` chain action and the cell's mission update do.
async fn kill(
    objective_id: i32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    mgr: &mut SpaceManager,
    db_pool: &Option<Arc<PgPool>>,
) {
    assert!(
        complete_objective(ENTITY_ID, MISSION_ID, objective_id, tx, mgr).await,
        "objective {objective_id} must complete"
    );
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", tx, mgr).await;
    forward(rx, db_pool).await;
}

/// The saved row of 10312 for `player_id`: status, step and the sorted
/// completed and active objective ids.
async fn saved_row(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Option<(i8, Option<i32>, Vec<i32>, Vec<i32>)> {
    let saved = query_saved_missions(db_pool, player_id).await;
    saved.iter().find(|m| m.mission_id == MISSION_ID).map(|m| {
        let mut completed = m.completed_objective_ids.clone();
        completed.sort_unstable();
        let mut active = m.active_objective_ids.clone();
        active.sort_unstable();
        (m.status, m.current_step_id, completed, active)
    })
}

/// Accept, two kills, a relog through the production hydrator, two more kills
/// and the completion, all saved through the production bridge; a second
/// character with one kill is untouched by the first's run. Reverting the
/// `MissionUpdate` on the fourth completion (skipping its forward) leaves the
/// reload at 3 of 4 and active, and the completed check fails.
#[tokio::test]
async fn live_db_castle_side_defense_q3_round_trip_persists_per_character() {
    let pool = require_db_or_skip!();
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
    }
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(128);

    // Character A: accept, then two kills.
    let mut a = make_mgr(&pool, PLAYER_A).await;
    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_ID,
            step_objectives(),
            &tx,
            &mut a
        )
        .await,
        "A's accept must create 10312"
    );
    send_mission_update(ENTITY_ID, PLAYER_A, MISSION_ID, "test", &tx, &a).await;
    forward(&mut rx, &db_pool).await;
    kill(OBJECTIVES[0], PLAYER_A, &tx, &mut rx, &mut a, &db_pool).await;
    kill(OBJECTIVES[1], PLAYER_A, &tx, &mut rx, &mut a, &db_pool).await;

    // Character B: accept and one kill (the third officer), on its own cell.
    let mut b = make_mgr(&pool, PLAYER_B).await;
    assert!(
        accept_mission(
            ENTITY_ID,
            MISSION_ID,
            STEP_ID,
            step_objectives(),
            &tx,
            &mut b
        )
        .await,
        "B's accept must create 10312"
    );
    send_mission_update(ENTITY_ID, PLAYER_B, MISSION_ID, "test", &tx, &b).await;
    forward(&mut rx, &db_pool).await;
    kill(OBJECTIVES[2], PLAYER_B, &tx, &mut rx, &mut b, &db_pool).await;

    let mid_a = saved_row(&db_pool, PLAYER_A).await;

    // A relogs: a fresh cell, missions rebuilt from the saved rows by the
    // production hydrator.
    let mut relogged = make_mgr(&pool, PLAYER_A).await;
    let saved = query_saved_missions(&db_pool, PLAYER_A).await;
    let restored = cimmeria_cell::cell::build_restored_missions_for_test(&saved, &relogged);
    {
        let entity = relogged.get_entity_mut(ENTITY_ID).unwrap();
        entity.missions.mark_loaded();
        for mission in restored {
            entity.missions.add_mission(mission);
        }
    }
    let after_relog = relogged
        .get_entity(ENTITY_ID)
        .unwrap()
        .missions
        .get_mission(MISSION_ID)
        .map(|m| {
            let mut done = m.completed_objectives.clone();
            done.sort_unstable();
            (m.status, done)
        });

    // The remaining two kills after the relog; the fourth completes the mission.
    kill(
        OBJECTIVES[2],
        PLAYER_A,
        &tx,
        &mut rx,
        &mut relogged,
        &db_pool,
    )
    .await;
    kill(
        OBJECTIVES[3],
        PLAYER_A,
        &tx,
        &mut rx,
        &mut relogged,
        &db_pool,
    )
    .await;

    let final_a = saved_row(&db_pool, PLAYER_A).await;
    let final_b = saved_row(&db_pool, PLAYER_B).await;
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
    }

    let mid_a = mid_a.expect("A keeps a row after two kills");
    assert_eq!(
        (mid_a.0, mid_a.1, mid_a.2),
        (
            MISSION_ACTIVE,
            Some(STEP_ID),
            vec![OBJECTIVES[0], OBJECTIVES[1]]
        ),
        "A is active at 2 of 4 before the relog"
    );
    assert_eq!(
        after_relog,
        Some((MISSION_ACTIVE, vec![OBJECTIVES[0], OBJECTIVES[1]])),
        "the production hydrator restores A at 2 of 4"
    );
    let final_a = final_a.expect("A keeps its completed row");
    assert_eq!(
        (final_a.0, final_a.2),
        (MISSION_COMPLETED, OBJECTIVES.to_vec()),
        "A's fourth completion must be saved: a 3 of 4 active row here means \
         the last MissionUpdate never reached sgw_mission"
    );
    let final_b = final_b.expect("B keeps its row");
    assert_eq!(
        (final_b.0, final_b.2),
        (MISSION_ACTIVE, vec![OBJECTIVES[2]]),
        "B's run is its own: one kill, still active, untouched by A's completion"
    );
}
