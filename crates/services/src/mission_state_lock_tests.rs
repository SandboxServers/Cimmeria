//! TG-DB-12 (D-TG18): when the saved missions can't be read at world entry,
//! the player's missions lock instead of loading as none, so a re-accept of
//! a completed mission can't overwrite its row (the #411 shape).
//!
//! Drives both halves: the base's retried read and lock
//! (`missions::state_lock` in `cimmeria-base-methods`, the persist backstop
//! through the production cell-to-base bridge) and the cell's tracker lock
//! (`cell::missions::state_lock`), against a real `sgw_mission` row.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::state_lock::{self, SavedMissionsLoad};
use crate::base::world_entry::methods::missions::{handle_mission_update, query_saved_missions};
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::missions::{accept_mission, send_mission_update};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};
use cimmeria_entity::missions::{MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE};
use cimmeria_mercury::transport::Transport;

/// Sentinels: this file owns `0x7000_D9A0..=0x7000_D9AF`.
const ACCOUNT_ID: i32 = 0x7000_D9A0;
const PLAYER_ID: i32 = 0x7000_D9A1;
const MISSION_ID: i32 = 0x7000_D9A2;
const STEP_ID: i32 = 0x7000_D9A3;
const OBJ_ID: i32 = 0x7000_D9A4;
const ENTITY_ID: u32 = 1;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1")
        .bind(PLAYER_ID)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT_ID)
        .execute(pool)
        .await;
}

async fn seed(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT_ID)
        .bind(format!("mission-lock-{ACCOUNT_ID}"))
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
    .bind(ACCOUNT_ID)
    .bind(PLAYER_ID)
    .bind(format!("test-{PLAYER_ID}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// A pool whose every acquire fails fast: nothing listens on port 1.
fn unreachable_pool() -> Arc<PgPool> {
    Arc::new(
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_millis(50))
            .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
            .expect("connect_lazy must succeed for any well-formed URL"),
    )
}

async fn saved_row(pool: &PgPool) -> (i32, i32) {
    sqlx::query_as(
        "SELECT status, repeats FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
    )
    .bind(PLAYER_ID)
    .bind(MISSION_ID)
    .fetch_one(pool)
    .await
    .expect("the completed row must still exist")
}

#[tokio::test]
async fn live_db_unreadable_missions_block_reaccept() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    seed(&pool).await;
    let db_pool = Some(Arc::new(pool.clone()));

    // A completed, non-repeatable mission on the saved row.
    handle_mission_update(
        PLAYER_ID,
        MISSION_ID,
        MISSION_COMPLETED,
        None,
        &[STEP_ID],
        &[OBJ_ID],
        &[OBJ_ID],
        &[],
        1,
        &db_pool,
    )
    .await;
    assert_eq!(saved_row(&pool).await, (MISSION_COMPLETED as i32, 1));

    let capture = LogCapture::install();

    // ── World entry: the read fails on every retry, so the missions lock.
    let load =
        state_lock::load_saved_missions_with_retry(&Some(unreachable_pool()), PLAYER_ID).await;
    assert!(
        matches!(load, SavedMissionsLoad::Unreadable),
        "an unreadable read must lock, never load as none"
    );
    state_lock::lock(PLAYER_ID, ENTITY_ID);

    // The cell entity, its tracker empty (what InitPlayerState carries on a
    // lock) and locked by `MissionStateUnknown`.
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(ENTITY_ID).unwrap().player_id = Some(PLAYER_ID);
    crate::cell::missions::state_lock::mark_state_unknown(ENTITY_ID, &mut mgr);

    // ── The re-accept, as the executor's accept arm runs it.
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    let objectives = vec![MissionObjective {
        objective_id: OBJ_ID,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }];
    let accepted = accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives, &tx, &mut mgr).await;
    send_mission_update(ENTITY_ID, PLAYER_ID, MISSION_ID, "accept", &tx, &mgr).await;
    assert!(!accepted, "a locked tracker must refuse the re-accept");

    // Everything the cell emitted goes through the production bridge, plus
    // the update an unguarded cell would have sent: the base backstop must
    // refuse it too.
    let mut emitted = Vec::new();
    while let Ok(m) = rx.try_recv() {
        emitted.push(m);
    }
    assert!(
        !emitted
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::MissionUpdate { .. })),
        "the cell must not save a locked tracker: {emitted:?}"
    );
    let feedback = cimmeria_wire::cell::chat::serialize_on_player_communication(
        "SYSTEM",
        0,
        cimmeria_wire::cell::chat::CHAN_FEEDBACK,
        crate::cell::missions::state_lock::UNAVAILABLE_LINE,
    );
    assert!(
        emitted.iter().any(
            |m| matches!(m, CellToBaseMsg::EntityMethodCall { args, .. } if *args == feedback)
        ),
        "the player must see the refusal: {emitted:?}"
    );
    emitted.push(CellToBaseMsg::MissionUpdate {
        player_id: PLAYER_ID,
        mission_id: MISSION_ID,
        status: 1,
        current_step_id: Some(STEP_ID),
        completed_step_ids: vec![],
        completed_objective_ids: vec![],
        active_objective_ids: vec![OBJ_ID],
        failed_objective_ids: vec![],
        repeats: 0,
    });
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
    let minigame_registry = None;
    for msg in emitted {
        handle_cell_message(
            msg,
            &transport,
            &connected,
            &entity_to_addr,
            &cell_tx,
            &db_pool,
            &minigame_registry,
            "127.0.0.1",
            0,
        )
        .await;
    }

    // ── The refusal is visible and logged; the completed row is untouched.
    let cell_warn = capture
        .all()
        .into_iter()
        .find(|c| {
            c.level == Level::WARN
                && c.has_field("event", "mission_state_unknown")
                && c.has_field("reason", "saved_missions_unreadable")
                && c.has_field("site", "accept")
        })
        .expect("the cell's refusal WARN");
    assert!(cell_warn.fields.contains_key("player_id"));
    assert!(cell_warn.fields.contains_key("mission_id"));
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.level == Level::WARN && c.has_field("site", "persist")),
        "the base backstop must refuse the row write with a WARN"
    );
    assert_eq!(
        saved_row(&pool).await,
        (MISSION_COMPLETED as i32, 1),
        "the completed row must be unchanged"
    );

    // Recovery: the base unlocks, and the row reads back as it was.
    assert!(state_lock::unlock(PLAYER_ID, ENTITY_ID));
    let restored = query_saved_missions(&db_pool, PLAYER_ID).await;
    assert_eq!(
        restored
            .iter()
            .find(|m| m.mission_id == MISSION_ID)
            .map(|m| m.status),
        Some(MISSION_COMPLETED)
    );

    cleanup(&pool).await;
}
