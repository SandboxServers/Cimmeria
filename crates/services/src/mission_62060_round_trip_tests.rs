//! Live-DB round trip for mission 62060 "Before the Patrol Returns"
//! (CB-N06-7): a real click through the cell's wire dispatcher
//! (`dispatch(PLAYER, INTERACT, ..)`), the content chains 11030-11041 on the
//! seeded engine, and the cell's `MissionUpdate` / `GrantItem` messages
//! through the base's real `handle_cell_message` into `sgw_mission` and
//! `sgw_inventory`. The choice must grant exactly two Health Slappacks
//! (item 2893) once, and the outcome must persist per character.
//!
//! The cell half is `cimmeria-cell-methods` / `cimmeria-cell-content` and
//! the base half `cimmeria-base-world-entry` / `cimmeria-base-methods`;
//! neither track can depend on the other, so the round trip lives in the
//! facade, like the bank, consumable and 62050 round trips.
//!
//! Sentinels (the CB-N06 block, `0x7000_8F10`..`0x7000_8F2F`): account
//! `0x7000_8F10`, players `0x7000_8F11` / `0x7000_8F12`, entity
//! `0x7000_8F20`. Port 40940.
//!
//! Revert proof: delete chain 11037's `step_status` condition row from the
//! seed and `..._aid_then_panel_grants_two_once` fails (a second click on
//! Prisoner 2 after completion grants again).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use cimmeria_mercury::transport::Transport;

use crate::base::world_entry::handle_cell_message;
use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::content::{build_engine, fire_player_loaded};
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::missions::send_mission_update;
use crate::cell::space_manager::{spawn_npcs_from_records, SpaceManager};
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_monologue_dialog_ids,
    load_spawns_from_db, load_step_objectives, load_world_rows,
};
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

const WORLD: &str = "Castle_CellBlock";
const ACCOUNT: i32 = 0x7000_8F10;
const PLAYER_A: i32 = 0x7000_8F11;
const PLAYER_B: i32 = 0x7000_8F12;
const ENTITY: u32 = 0x7000_8F20;
const PORT: u16 = 40940;
const MISSION: i32 = 62060;
const SLAPPACK: i32 = 2893;
const PRISONER_1: &str = "Cellblock_Ambient_Prisoner1";
const PRISONER_2: &str = "Cellblock_Ambient_Prisoner2";
const PANEL: &str = "329_CellDoorButton";

async fn teardown(pool: &PgPool) {
    for sql in [
        "DELETE FROM cell_event_outbox WHERE entity_id = $1",
        "DELETE FROM sgw_inventory WHERE character_id = $1",
        "DELETE FROM sgw_mission WHERE player_id = $1",
    ] {
        for id in [PLAYER_A, PLAYER_B] {
            let _ = sqlx::query(sql).bind(id).execute(pool).await;
        }
    }
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await;
}

/// The account and two characters on it; whatever a crashed run left is
/// removed first.
async fn seed(pool: &PgPool) {
    teardown(pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("mission-62060-{ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");
    for player_id in [PLAYER_A, PLAYER_B] {
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', $4, 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0)",
        )
        .bind(ACCOUNT)
        .bind(player_id)
        .bind(format!("patrol-{player_id}"))
        .bind(WORLD)
        .execute(pool)
        .await
        .expect("insert player");
    }
}

/// The cell for `player_id` in `WORLD`: the two prisoners and the panel from
/// their real spawn rows, the real mission defs, stamped world rows,
/// dialog-set maps and container sets, and one connected player.
async fn stage(pool: &PgPool, player_id: i32) -> SpaceManager {
    let tags = [PRISONER_1, PRISONER_2, PANEL];
    let records: Vec<_> = load_spawns_from_db(pool)
        .await
        .expect("load spawnlist")
        .into_iter()
        .filter(|r| r.world_name == WORLD)
        .filter(|r| r.tag.as_deref().is_some_and(|t| tags.contains(&t)))
        .collect();
    assert_eq!(
        records.len(),
        tags.len(),
        "each tagged spawn is seeded once"
    );

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    assert_eq!(spawn_npcs_from_records(&records, &mut mgr), tags.len());
    mgr.stamp_world_rows(&load_world_rows(pool).await.expect("world rows"));
    mgr.mission_defs = load_mission_defs(pool).await.expect("mission defs");
    mgr.step_objectives = load_step_objectives(pool).await.expect("step objectives");
    mgr.dialog_set_maps = load_dialog_set_maps(pool).await.expect("dialog set maps");
    mgr.monologue_dialog_ids = load_monologue_dialog_ids(pool).await.expect("monologues");
    mgr.item_containers = load_item_containers(pool).await.expect("item containers");

    mgr.create_entity(ENTITY, WORLD, [-270.0, 65.45, -135.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(ENTITY).unwrap();
    p.is_player = true;
    p.player_id = Some(player_id);
    p.account_id = Some(ACCOUNT as u32);
    mgr.connect_entity(ENTITY);
    mgr
}

/// The base's side: one in-world client for `ENTITY`.
struct Base {
    transport: Arc<dyn Transport>,
    e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    conn: Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>,
    pool: Option<Arc<PgPool>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

impl Base {
    fn new(pool: &PgPool) -> Base {
        let addr: SocketAddr = format!("127.0.0.1:{PORT}").parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(ENTITY);
        let (tx, cell_rx) = mpsc::channel(256);
        Base {
            transport: Arc::new(TestTransport::new()),
            e2a: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
            conn: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            pool: Some(Arc::new(pool.clone())),
            cell_tx: Some(tx),
            cell_rx,
        }
    }

    /// Hand every `MissionUpdate` and `GrantItem` the cell queued to the
    /// production bridge, in order, as the base's cell-message loop does.
    /// Returns how many of each it forwarded.
    async fn forward(&mut self, rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (usize, usize) {
        let (mut updates, mut grants) = (0, 0);
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::MissionUpdate { .. } => updates += 1,
                CellToBaseMsg::GrantItem { .. } => grants += 1,
                _ => continue,
            }
            handle_cell_message(
                msg,
                &self.transport,
                &self.conn,
                &self.e2a,
                &self.cell_tx,
                &self.pool,
                &None,
                "127.0.0.1",
                0,
            )
            .await;
            while self.cell_rx.try_recv().is_ok() {}
        }
        (updates, grants)
    }
}

/// Complete 638 for the cell player and save it, as a played-through
/// character would have it on disk.
async fn finish_638(
    mgr: &mut SpaceManager,
    player_id: i32,
    base: &mut Base,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
) {
    let mut m = MissionInstance::new(638, 0, Vec::new());
    m.complete();
    mgr.get_entity_mut(ENTITY).unwrap().missions.add_mission(m);
    send_mission_update(ENTITY, player_id, 638, "test", tx, mgr).await;
    base.forward(rx).await;
}

/// Right-click `tag`'s entity from `metres` away (a straight line along x).
async fn click(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    tag: &str,
    metres: f32,
) -> bool {
    let npc = mgr.find_entity_by_tag(ENTITY, tag).expect("tag is spawned");
    let at = mgr.get_entity(npc).unwrap().position;
    mgr.update_entity_position(ENTITY, [at.x + metres, at.y, at.z], [0; 3], [0.0; 3]);
    dispatch(
        ENTITY,
        INTERACT,
        &(npc as i32).to_le_bytes(),
        tx,
        mgr,
        engine,
    )
    .await
}

/// Every saved `sgw_inventory` row of item 2893 for `player_id` as stacks.
async fn slappacks(pool: &PgPool, player_id: i32) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT stack_size FROM sgw_inventory WHERE character_id = $1 AND type_id = $2 \
         ORDER BY item_id",
    )
    .bind(player_id)
    .bind(SLAPPACK)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The saved status of mission 62060 for `player_id`.
async fn saved_status(db: &Option<Arc<PgPool>>, player_id: i32) -> Option<i8> {
    query_saved_missions(db, player_id)
        .await
        .iter()
        .find(|m| m.mission_id == MISSION)
        .map(|m| m.status)
}

fn status(mgr: &SpaceManager) -> Option<i8> {
    mgr.get_entity(ENTITY)?
        .missions
        .get_mission(MISSION)
        .map(|m| m.status)
}

fn bound(mgr: &SpaceManager, slot: i32) -> bool {
    mgr.get_entity(ENTITY)
        .unwrap()
        .available_interactions
        .get(&slot)
        .is_some_and(|v| !v.is_empty())
}

/// Rebuild a fresh cell for `player_id` from its saved rows, as the login
/// does, then fire `player_loaded`.
async fn relog(
    pool: &PgPool,
    db: &Option<Arc<PgPool>>,
    player_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> SpaceManager {
    let mut fresh = stage(pool, player_id).await;
    for row in query_saved_missions(db, player_id).await {
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
            .get_entity_mut(ENTITY)
            .unwrap()
            .missions
            .add_mission(mi);
    }
    fire_player_loaded(ENTITY, player_id, WORLD, engine, tx, &mut fresh).await;
    fresh
}

/// Prisoner 1 accepts, Prisoner 2 (the aid branch) completes and grants two
/// Health Slappacks as one stack; the saved mission row is completed; a
/// later panel click and further prisoner clicks add nothing.
#[tokio::test]
async fn live_db_mission_62060_round_trip_aid_then_panel_grants_two_once() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let db = Some(Arc::new(pool.clone()));
    let mut mgr = stage(&pool, PLAYER_A).await;
    let engine = build_engine(Some(&pool)).await;
    let mut base = Base::new(&pool);
    let (tx, mut rx) = mpsc::channel(8192);
    finish_638(&mut mgr, PLAYER_A, &mut base, &tx, &mut rx).await;
    fire_player_loaded(ENTITY, PLAYER_A, WORLD, &engine, &tx, &mut mgr).await;
    assert!(bound(&mgr, 176), "chain 11031 binds the prisoner offer");
    base.forward(&mut rx).await;

    assert!(click(&mut mgr, &engine, &tx, PRISONER_1, 1.0).await);
    let (updates, grants) = base.forward(&mut rx).await;
    assert!(updates >= 1, "the accept is saved");
    assert_eq!(grants, 0, "accepting grants nothing");
    assert_eq!(saved_status(&db, PLAYER_A).await, Some(MISSION_ACTIVE));
    assert!(bound(&mgr, 22), "the accept binds the panel choice");
    assert!(slappacks(&pool, PLAYER_A).await.is_empty());

    assert!(click(&mut mgr, &engine, &tx, PRISONER_2, 1.0).await);
    let (_, grants) = base.forward(&mut rx).await;
    assert_eq!(grants, 1, "the aid branch sends exactly one GrantItem");
    assert_eq!(
        slappacks(&pool, PLAYER_A).await,
        vec![2],
        "exactly one row of item 2893, stack 2"
    );
    assert_eq!(saved_status(&db, PLAYER_A).await, Some(MISSION_COMPLETED));
    assert!(!bound(&mgr, 22) && !bound(&mgr, 176), "both choices unbind");

    // The sabotage branch after the aid: nothing more.
    click(&mut mgr, &engine, &tx, PANEL, 1.0).await;
    click(&mut mgr, &engine, &tx, PRISONER_1, 1.0).await;
    click(&mut mgr, &engine, &tx, PRISONER_2, 1.0).await;
    let (_, grants) = base.forward(&mut rx).await;
    assert_eq!(grants, 0, "no second grant after completion");
    assert_eq!(slappacks(&pool, PLAYER_A).await, vec![2], "still one row");

    teardown(&pool).await;
}

/// A click from 6 m (beyond `MAX_INTERACT_DISTANCE` 5.0) accepts nothing
/// and grants nothing; the same click from 1 m is accepted.
#[tokio::test]
async fn live_db_mission_62060_round_trip_far_click_refused() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let db = Some(Arc::new(pool.clone()));
    let mut mgr = stage(&pool, PLAYER_A).await;
    let engine = build_engine(Some(&pool)).await;
    let mut base = Base::new(&pool);
    let (tx, mut rx) = mpsc::channel(8192);
    finish_638(&mut mgr, PLAYER_A, &mut base, &tx, &mut rx).await;
    fire_player_loaded(ENTITY, PLAYER_A, WORLD, &engine, &tx, &mut mgr).await;
    base.forward(&mut rx).await;

    click(&mut mgr, &engine, &tx, PRISONER_1, 6.0).await;
    base.forward(&mut rx).await;
    assert_eq!(status(&mgr), None, "the far click does not accept 62060");
    assert_eq!(saved_status(&db, PLAYER_A).await, None);
    assert!(slappacks(&pool, PLAYER_A).await.is_empty());

    assert!(click(&mut mgr, &engine, &tx, PRISONER_1, 1.0).await);
    base.forward(&mut rx).await;
    assert_eq!(status(&mgr), Some(MISSION_ACTIVE), "the near click accepts");
    assert_eq!(saved_status(&db, PLAYER_A).await, Some(MISSION_ACTIVE));

    teardown(&pool).await;
}

/// After the aid flow, a relog rebuilds from the saved rows: 62060 is
/// completed, nothing binds, further clicks change nothing and the grant
/// stays two. The other character on the account has no row and no item.
#[tokio::test]
async fn live_db_mission_62060_round_trip_relog_keeps_the_outcome_per_character() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let db = Some(Arc::new(pool.clone()));
    let mut mgr = stage(&pool, PLAYER_A).await;
    let engine = build_engine(Some(&pool)).await;
    let mut base = Base::new(&pool);
    let (tx, mut rx) = mpsc::channel(8192);
    finish_638(&mut mgr, PLAYER_A, &mut base, &tx, &mut rx).await;
    fire_player_loaded(ENTITY, PLAYER_A, WORLD, &engine, &tx, &mut mgr).await;
    assert!(click(&mut mgr, &engine, &tx, PRISONER_1, 1.0).await);
    assert!(click(&mut mgr, &engine, &tx, PRISONER_2, 1.0).await);
    base.forward(&mut rx).await;
    assert_eq!(saved_status(&db, PLAYER_A).await, Some(MISSION_COMPLETED));
    assert_eq!(slappacks(&pool, PLAYER_A).await, vec![2]);

    let (tx2, mut rx2) = mpsc::channel(8192);
    let mut fresh = relog(&pool, &db, PLAYER_A, &engine, &tx2).await;
    assert_eq!(status(&fresh), Some(MISSION_COMPLETED), "relog: completed");
    assert!(!bound(&fresh, 176) && !bound(&fresh, 22), "relog: no binds");
    for tag in [PRISONER_1, PRISONER_2, PANEL] {
        click(&mut fresh, &engine, &tx2, tag, 1.0).await;
    }
    let (_, grants) = base.forward(&mut rx2).await;
    assert_eq!(grants, 0, "relog: no click grants again");
    assert_eq!(status(&fresh), Some(MISSION_COMPLETED));
    assert_eq!(slappacks(&pool, PLAYER_A).await, vec![2], "still two");

    assert_eq!(
        saved_status(&db, PLAYER_B).await,
        None,
        "B has no 62060 row"
    );
    assert!(slappacks(&pool, PLAYER_B).await.is_empty(), "B has no 2893");

    teardown(&pool).await;
}
