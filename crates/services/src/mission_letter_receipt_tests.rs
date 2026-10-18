//! The Marsh letter receipt end to end (HA-1360b2): the Cellblock grant, the
//! Castle crossing that persists step 4038, a fresh hydrate in Harset, the
//! world-68 6502 bind and a real `CM_INTERACT` on Marsh. The cell's messages
//! are routed through `route_cell_message` with the shipped `base_plugins()`
//! table, so the base's turn-in route is the production one.
//!
//! This file is the harness only (step A). The remaining tests and the revert
//! probes are added on top of it. Test-only.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::ConnectedClientState;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::MissionInstance;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::{
    BaseToCellMsg, LetterTurnInEntryEpoch, LetterTurnInSession, LetterTurnInToken,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::base::world_entry::methods::query_player_load_data;
use crate::base::world_entry::route_cell_message;
use crate::cell::content::{
    fire_dialog_open, fire_player_loaded, fire_stargate_crossed, load_single_chain_for_test,
};
use crate::cell::dispatch::dispatch_cell_method;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_item_containers, load_mission_defs, load_step_objectives, load_world_rows,
};
use crate::plugins::base_plugins;
use crate::test_support::{require_db_or_skip, TestTransport};

/// Sentinel block for this file's accounts and players, `0x7000_8600..=0x7000_86FF`.
/// Free at HEAD (`git grep -n "0x7000_8[5-9A-F]" -- crates`). Cleanup deletes
/// by exact id, never by range.
const TEST_BASE: i32 = 0x7000_8600;

/// The `CM_INTERACT` player method index (`wireclient` `calls.rs`).
const CM_INTERACT: u16 = 74;
/// The item the Cellblock grant hands over (the real Frost's Letter row).
const LETTER_ITEM: i32 = 3730;
/// The cell entity of the one character in each scenario.
const PLAYER_EID: u32 = 1;
/// The live Marsh, tag `CmdCenter_Marsh`, template 10.
const MARSH_EID: u32 = 2;

/// The chains the receipt path reads: the Cellblock grant (1003/1121), the
/// seeded Castle bridge (1366), the Marsh receipt (6501) and the Harset bind
/// (6502).
const RECEIPT_CHAINS: [i32; 5] = [1003, 1121, 1366, 6501, 6502];

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
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
        .bind(format!("letter-receipt-{account_id}"))
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
    .bind(format!("receipt-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// One character's path through the receipt: the cell's chain engine, the
/// base's plugin table and dispatch maps, and the channel the cell writes to.
struct Driver {
    pool: PgPool,
    account_id: i32,
    player_id: i32,
    engine: ChainEngine,
    plugins: cimmeria_base_session::base::plugin::BasePlugins,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    base_tx: mpsc::Sender<BaseToCellMsg>,
    /// Held so the base's result channel stays open for the whole scenario.
    _base_rx: mpsc::Receiver<BaseToCellMsg>,
}

impl Driver {
    /// Load the given chains from the seed, as the cell's content engine does.
    async fn new(pool: PgPool, account_id: i32, player_id: i32, chains: &[i32]) -> Self {
        let mut engine = ChainEngine::new();
        for &id in chains {
            let chain = load_single_chain_for_test(&pool, id)
                .await
                .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
                .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
            engine.register_chain(chain);
        }
        let plugins = base_plugins().expect("the shipped base plugin table must build");
        let (base_tx, _base_rx) = mpsc::channel(64);
        Self {
            pool,
            account_id,
            player_id,
            engine,
            plugins,
            transport: Arc::new(TestTransport::new()),
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
            base_tx,
            _base_rx,
        }
    }

    /// A fresh cell hydrate in `world`, from the saved rows: the production
    /// world-entry mission hydrator, the same as a relog or a travel would run.
    async fn hydrate(&self, world: &str) -> SpaceManager {
        let pool = &self.pool;
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(&format!(
            r#"<Spaces><Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#
        ))
        .unwrap();
        mgr.create_startup_spaces(&format!(
            r#"<Spaces><Space WorldName="{world}" /></Spaces>"#
        ))
        .unwrap();
        mgr.stamp_world_rows(&load_world_rows(pool).await.unwrap());
        mgr.mission_defs = load_mission_defs(pool).await.unwrap();
        mgr.step_objectives = load_step_objectives(pool).await.unwrap();
        mgr.item_containers = load_item_containers(pool).await.unwrap();
        mgr.create_entity(PLAYER_EID, world, [0.0; 3], [0.0; 3])
            .unwrap();
        let db_pool = Some(Arc::new(pool.clone()));
        let saved = query_saved_missions(&db_pool, self.player_id).await;
        let restored = cimmeria_cell::cell::build_restored_missions_for_test(&saved, &mgr);
        let load = query_player_load_data(&db_pool, self.account_id as u32, self.player_id).await;
        assert_eq!(
            load.player_id, self.player_id,
            "production player hydration"
        );
        let entity = mgr.get_entity_mut(PLAYER_EID).unwrap();
        entity.player_id = Some(self.player_id);
        entity.archetype_id = Some(load.archetype);
        entity.is_player = true;
        entity.missions.mark_loaded();
        for mission in restored {
            entity.missions.add_mission(mission);
        }
        mgr
    }

    /// Route the cell's messages through the production base dispatcher with
    /// the shipped plugin table, in the order the cell queued them.
    async fn route(&self, messages: Vec<CellToBaseMsg>) {
        let cell_tx = Some(self.base_tx.clone());
        let db_pool = Some(Arc::new(self.pool.clone()));
        for msg in messages {
            route_cell_message(
                msg,
                &self.transport,
                &self.connected,
                &self.entity_to_addr,
                &cell_tx,
                &db_pool,
                &None,
                "127.0.0.1",
                0,
                &self.plugins,
            )
            .await;
        }
    }

    /// The Frost's Letter instances the character holds in the database.
    async fn letter_count(&self) -> usize {
        let load = query_player_load_data(
            &Some(Arc::new(self.pool.clone())),
            self.account_id as u32,
            self.player_id,
        )
        .await;
        load.items.iter().filter(|i| i.dbid == LETTER_ITEM).count()
    }

    /// Mission 1360's persisted current step, `None` when it is not saved.
    async fn step_1360(&self) -> Option<i32> {
        query_saved_missions(&Some(Arc::new(self.pool.clone())), self.player_id)
            .await
            .iter()
            .find(|m| m.mission_id == 1360)
            .and_then(|m| m.current_step_id)
    }
}

/// Drain everything the cell queued since the last call, in order.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut messages = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    messages
}

/// Smoke test for the harness: the Cellblock grant, the Castle crossing, the
/// Harset bind and the Marsh click reach the turn-in request, and nothing
/// succeeds before a correlated result. Later tests build on this path.
#[tokio::test]
async fn live_db_marsh_receipt_click_is_pending_before_the_result() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 1, TEST_BASE + 2);
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let driver = Driver::new(pool.clone(), account_id, player_id, &RECEIPT_CHAINS).await;
    let (tx, mut rx) = mpsc::channel(128);

    // Cellblock: the seeded grant chain hands over the letter and accepts 1360.
    let mut cellblock = driver.hydrate("Castle_CellBlock").await;
    cellblock
        .get_entity_mut(PLAYER_EID)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(622, 2113, vec![]));
    fire_dialog_open(
        PLAYER_EID,
        player_id,
        3995,
        &driver.engine,
        &tx,
        &mut cellblock,
    )
    .await;
    driver.route(drain(&mut rx)).await;
    assert_eq!(driver.letter_count().await, 1, "one letter from the grant");
    assert_eq!(driver.step_1360().await, Some(4037));
    drop(cellblock);

    // Castle: the seeded bridge chain 1366 persists 4038 on the crossing.
    let mut castle = driver.hydrate("Castle").await;
    fire_stargate_crossed(
        PLAYER_EID,
        player_id,
        "Harset",
        &driver.engine,
        &tx,
        &mut castle,
    )
    .await;
    driver.route(drain(&mut rx)).await;
    assert_eq!(
        driver.step_1360().await,
        Some(4038),
        "crossing persists 4038"
    );
    drop(castle);

    // Harset: a fresh hydrate, then the world-68 6502 bind on entry.
    let mut harset = driver.hydrate("Harset_CmdCenter").await;
    let space = harset.get_entity(PLAYER_EID).unwrap().space_id.0 as u32;
    harset
        .create_entity_in_space(MARSH_EID, space, [1.0, 0.0, 0.0], [0.0; 3])
        .expect("Marsh must spawn beside the player");
    harset.connect_entity(PLAYER_EID);
    let marsh = harset.get_entity_mut(MARSH_EID).unwrap();
    marsh.tag = Some("CmdCenter_Marsh".into());
    marsh.template_id = Some(10);
    let player = harset.get_entity_mut(PLAYER_EID).unwrap();
    player.last_interaction_target = Some(MARSH_EID);
    // The turn-in session the Marsh request is bound to (the same test-side
    // seat `stage_player` gives the executor tests).
    player.extensions.insert(LetterTurnInSession {
        player_id,
        token: LetterTurnInToken::from_bytes([8; 16]),
        generation: std::time::Instant::now(),
        epoch: LetterTurnInEntryEpoch::new(1),
    });
    fire_player_loaded(
        PLAYER_EID,
        player_id,
        "Harset_CmdCenter",
        &driver.engine,
        &tx,
        &mut harset,
    )
    .await;
    driver.route(drain(&mut rx)).await;

    // The real click: CM_INTERACT on Marsh through the cell's method router.
    dispatch_cell_method(
        PLAYER_EID,
        CM_INTERACT,
        &(MARSH_EID as i32).to_le_bytes(),
        &tx,
        &mut harset,
        &driver.engine,
        None,
    )
    .await;
    let click = drain(&mut rx);
    let requests = click
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::Plugin(_)))
        .count();
    assert_eq!(requests, 1, "the click queues exactly one turn-in request");
    driver.route(click).await;
    assert_eq!(
        driver.letter_count().await,
        1,
        "no success before a correlated result: the letter is still held"
    );
    assert_eq!(driver.step_1360().await, Some(4038));

    cleanup(&pool, account_id, player_id).await;
}
