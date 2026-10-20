//! Live-DB guard for the Castle side story `return_q1` (CS-04c1c1 item 5, the
//! executable G10 proof): the Harset to Castle crossing accepts mission 10320
//! through the real dial path, the accept's `CellToBaseMsg::MissionUpdate` is
//! enqueued BEFORE the `GateTravel` that tears the player down, and the
//! production cell-to-base bridge (`handle_cell_message`) persists it into
//! `sgw_mission`, so the destination world's read finds it.
//!
//! Beside `castle_side_medical_q1_round_trip_tests`. Sentinel window
//! `0x7000_9B00..=0x7000_9B0F`, the only ids this file writes. Real ids:
//! mission 10320, step 10372, objective 10492, prerequisite 708.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE, MISSION_COMPLETED};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::cell::content::load_single_chain_for_test;
use crate::cell::gate_travel::handle_dial_gate;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_item_containers, load_mission_defs, load_step_objectives, load_world_rows, StargateEntry,
};
use crate::test_support::{require_db_or_skip, TestTransport};

const ACCOUNT_A: i32 = 0x7000_9B00;
const PLAYER_A: i32 = 0x7000_9B01;
const ACCOUNT_B: i32 = 0x7000_9B02;
const PLAYER_B: i32 = 0x7000_9B03;
const ENTITY_ID: u32 = 1;
const TARGET_GATE: i32 = 2;
const MISSION_ID: i32 = 10320;
const STEP_A: i32 = 10372;
const PREREQ: i32 = 708;
/// The crossing accept, the only chain this round trip needs.
const CROSSING_CHAIN: i32 = 10130;

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
        .bind(format!("castle-return-q1-{account_id}"))
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
    .bind(format!("return-q1-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// Harset with one player who holds the Castle address and, when
/// `finished_708`, has completed 708. No stargate region exists, so the dial
/// travels in one call: crossing chains first, then `GateTravel`.
async fn harset_cell(pool: &PgPool, player_id: i32, finished_708: bool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<Spaces>
            <Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
            <Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<Spaces><Space WorldName="Harset" /><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.unwrap());
    mgr.mission_defs = load_mission_defs(pool).await.unwrap();
    mgr.step_objectives = load_step_objectives(pool).await.unwrap();
    mgr.item_containers = load_item_containers(pool).await.unwrap();
    mgr.stargates.insert(
        TARGET_GATE,
        StargateEntry {
            world_name: "Castle".to_string(),
            x: 761.677,
            y: 63.466,
            z: 551.716,
            yaw: 2.152,
            address_origin: 18,
            arrival: None,
            event_set_id: None,
            debug_dial_hub: false,
        },
    );
    mgr.create_entity(ENTITY_ID, "Harset", [10.0; 3], [0.0; 3])
        .unwrap();
    mgr.connect_entity(ENTITY_ID);
    let entity = mgr.get_entity_mut(ENTITY_ID).unwrap();
    entity.is_player = true;
    entity.player_id = Some(player_id);
    entity.archetype_id = Some(1);
    entity.known_stargates = vec![TARGET_GATE];
    if finished_708 {
        let mut done = MissionInstance::new(PREREQ, 0, Vec::new());
        done.status = MISSION_COMPLETED;
        done.current_step_id = None;
        entity.missions.add_mission(done);
    }
    mgr
}

/// Dial Castle from `mgr` through the real handler and return everything the
/// cell queued for the base, in order.
async fn dial_castle(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    handle_dial_gate(ENTITY_ID, TARGET_GATE, 0, &tx, mgr, engine).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Hand every `MissionUpdate` to the production bridge, in queue order.
async fn forward_updates(traffic: Vec<CellToBaseMsg>, db_pool: &Option<Arc<PgPool>>) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx: Option<tokio::sync::mpsc::Sender<BaseToCellMsg>> = None;
    for msg in traffic {
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

/// **The crossing persists 10320 before the player leaves, for that
/// character only.** Character A (708 done) dials Castle: the accept's
/// `MissionUpdate` precedes the `GateTravel`, and after the production bridge
/// runs, `query_saved_missions` returns 10320 active at step 10372 for A. B
/// (708 not done) crosses with no accept and no row. Revert proof: sending the
/// update after the `GateTravel` fails the order assertion; deleting chain
/// 10130 fails the missing-update assertion.
#[tokio::test]
async fn live_db_castle_side_return_q1_crossing_round_trip() {
    let pool = require_db_or_skip!();
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
    }
    let db_pool = Some(Arc::new(pool.clone()));
    let mut engine = ChainEngine::new();
    if let Some(chain) = load_single_chain_for_test(&pool, CROSSING_CHAIN)
        .await
        .unwrap()
    {
        engine.register_chain(chain);
    }

    let mut a = harset_cell(&pool, PLAYER_A, true).await;
    let traffic_a = dial_castle(&engine, &mut a).await;
    let mut b = harset_cell(&pool, PLAYER_B, false).await;
    let traffic_b = dial_castle(&engine, &mut b).await;

    let update = traffic_a.iter().position(|m| {
        matches!(
            m,
            CellToBaseMsg::MissionUpdate {
                mission_id: MISSION_ID,
                current_step_id: Some(STEP_A),
                ..
            }
        )
    });
    let travel = traffic_a
        .iter()
        .position(|m| matches!(m, CellToBaseMsg::GateTravel { .. }));
    let order = (update, travel);
    let b_updates = traffic_b
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::MissionUpdate { .. }))
        .count();
    let b_travels = traffic_b
        .iter()
        .any(|m| matches!(m, CellToBaseMsg::GateTravel { .. }));

    let dump_a = format!("{traffic_a:?}");
    forward_updates(traffic_a, &db_pool).await;
    forward_updates(traffic_b, &db_pool).await;
    let saved_a = query_saved_missions(&db_pool, PLAYER_A).await;
    let saved_b = query_saved_missions(&db_pool, PLAYER_B).await;
    for (account_id, player_id) in [(ACCOUNT_A, PLAYER_A), (ACCOUNT_B, PLAYER_B)] {
        cleanup(&pool, account_id, player_id).await;
    }

    let (Some(update), Some(travel)) = order else {
        panic!("A's crossing must queue a 10320 update and a GateTravel; got {dump_a}");
    };
    assert!(
        update < travel,
        "the accept must be queued before the GateTravel (update {update}, travel {travel})"
    );
    let row = saved_a
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("A's accept must reload from sgw_mission");
    assert_eq!(
        (row.status, row.current_step_id),
        (MISSION_ACTIVE, Some(STEP_A)),
        "A reloads 10320 active at step 10372"
    );
    assert_eq!(b_updates, 0, "B (708 not done) must not accept 10320");
    assert!(b_travels, "B still crosses; only the accept is gated");
    assert!(
        saved_b.iter().all(|m| m.mission_id != MISSION_ID),
        "B must have no 10320 row: {:?}",
        saved_b
            .iter()
            .map(|m| (m.mission_id, m.status))
            .collect::<Vec<_>>()
    );
}
