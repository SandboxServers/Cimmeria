//! Live-DB guards for saving an abandon (#1315): the cell's
//! `abandon_mission` → `CellToBaseMsg::MissionUpdate` → the production
//! cell-to-base bridge (`handle_cell_message`) → `sgw_mission` → the re-login
//! query. Before the fix an abandon changed cell memory only, so the saved
//! row stayed active and the mission came back after a relog.
//!
//! Beside `mission_round_trip_tests`, in the free top of its sentinel window
//! (`0x7000_84BC..=0x7000_84C1`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE,
    STATUS_ACTIVE,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::missions::{abandon_mission, accept_mission, send_mission_update};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::MissionDefEntry;
use crate::test_support::{require_db_or_skip, TestTransport};

/// The top of `mission_round_trip_tests`' window (`0x7000_8200..=0x7000_84FF`,
/// whose own rows end at `+601`, `0x7000_8459`).
const ACCOUNT_A: i32 = 0x7000_84BC;
const PLAYER_A: i32 = 0x7000_84BD;
const ACCOUNT_B: i32 = 0x7000_84BE;
const PLAYER_B: i32 = 0x7000_84BF;
const ACCOUNT_C: i32 = 0x7000_84C0;
const PLAYER_C: i32 = 0x7000_84C1;
const ENTITY_ID: u32 = 1;
const MISSION_ID: i32 = 0x7000_0710;
const STEP_ID: i32 = 0x7000_0711;

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
        .bind(format!("mission-abandon-{account_id}"))
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
    .bind(format!("abandon-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

fn make_mgr(player_id: i32, num_repeats: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(ENTITY_ID).unwrap().player_id = Some(player_id);
    mgr.mission_defs.insert(
        MISSION_ID,
        MissionDefEntry {
            step_id: STEP_ID,
            objectives: vec![],
            is_hidden: false,
            num_repeats,
            can_repeat_on_fail: false,
        },
    );
    mgr
}

fn objectives() -> Vec<MissionObjective> {
    vec![MissionObjective {
        objective_id: 0x7000_0712,
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

/// Accept and save, abandon, re-login: the mission is gone from the saved
/// rows. Reverting the abandon save leaves the row active (the #1315 bug).
#[tokio::test]
async fn live_db_abandoned_mission_does_not_return_after_relog() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (ACCOUNT_A, PLAYER_A);
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let db_pool = Some(Arc::new(pool.clone()));
    let mut mgr = make_mgr(player_id, 0);
    let (tx, mut rx) = mpsc::channel(64);

    assert!(accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives(), &tx, &mut mgr).await);
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", &tx, &mgr).await;
    forward(&mut rx, &db_pool).await;
    let saved = query_saved_missions(&db_pool, player_id).await;
    assert_eq!(
        saved
            .iter()
            .map(|m| (m.mission_id, m.status))
            .collect::<Vec<_>>(),
        vec![(MISSION_ID, MISSION_ACTIVE)],
        "precondition: the accept is saved"
    );

    assert!(abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await);
    forward(&mut rx, &db_pool).await;
    let saved = query_saved_missions(&db_pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert!(
        saved.iter().all(|m| m.mission_id != MISSION_ID),
        "the abandoned mission must not reload: {:?}",
        saved
            .iter()
            .map(|m| (m.mission_id, m.status))
            .collect::<Vec<_>>()
    );
}

/// A repeatable mission completed twice, re-accepted and abandoned reloads
/// as not active with `repeats` 2 (#118 kept the count across re-accepts;
/// a delete here would reset it to 0).
#[tokio::test]
async fn live_db_abandoned_repeatable_mission_keeps_its_repeats() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (ACCOUNT_B, PLAYER_B);
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let db_pool = Some(Arc::new(pool.clone()));
    let mut mgr = make_mgr(player_id, 5);
    let mut done = MissionInstance::new(MISSION_ID, STEP_ID, vec![]);
    done.complete();
    done.repeats = 2;
    mgr.get_entity_mut(ENTITY_ID)
        .unwrap()
        .missions
        .add_mission(done);
    let (tx, mut rx) = mpsc::channel(64);
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", &tx, &mgr).await;
    forward(&mut rx, &db_pool).await;

    assert!(accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives(), &tx, &mut mgr).await);
    send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", &tx, &mgr).await;
    assert!(abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await);
    forward(&mut rx, &db_pool).await;

    let saved = query_saved_missions(&db_pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    let row = saved
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("a repeated mission keeps a row for its count");
    assert_eq!(
        (row.status, row.current_step_id, row.repeats),
        (MISSION_NOT_ACTIVE, None, 2),
        "not active, no step, the count kept"
    );
    assert_ne!(row.status, MISSION_COMPLETED);
}

/// **Regression guard (CS-08 review R1).** An abandon of a completed mission
/// leaves its saved row alone, for a row written since #118 (`repeats` 1) and
/// for a pre-#118 one (`repeats` 0, which a not-active save would DELETE).
/// Before the active-only guard the first reloaded as not active and the
/// second was gone, so the completion was lost for good and the mission
/// could be earned again.
#[tokio::test]
async fn live_db_abandon_of_a_completed_mission_keeps_its_saved_row() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (ACCOUNT_C, PLAYER_C);
    for repeats in [1, 0] {
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));
        let mut mgr = make_mgr(player_id, 0);
        let mut done = MissionInstance::new(MISSION_ID, STEP_ID, vec![]);
        done.complete();
        done.repeats = repeats;
        mgr.get_entity_mut(ENTITY_ID)
            .unwrap()
            .missions
            .add_mission(done);
        let (tx, mut rx) = mpsc::channel(64);
        send_mission_update(ENTITY_ID, player_id, MISSION_ID, "test", &tx, &mgr).await;
        forward(&mut rx, &db_pool).await;

        assert!(
            !abandon_mission(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await,
            "repeats {repeats}: a completed mission is not abandoned"
        );
        forward(&mut rx, &db_pool).await;

        let saved = query_saved_missions(&db_pool, player_id).await;
        cleanup(&pool, account_id, player_id).await;
        let row = saved
            .iter()
            .find(|m| m.mission_id == MISSION_ID)
            .unwrap_or_else(|| panic!("repeats {repeats}: the completed row must survive"));
        assert_eq!(
            (row.status, row.repeats),
            (MISSION_COMPLETED, repeats),
            "repeats {repeats}: still completed"
        );
    }
}
