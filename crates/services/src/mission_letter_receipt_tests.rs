//! The Marsh letter receipt end to end (HA-1360b2): the Cellblock grant, the
//! Castle crossing that persists step 4038, a fresh hydrate in Harset, the
//! world-68 6502 bind and a real `CM_INTERACT` on Marsh. The cell's messages
//! are routed through `route_cell_message` with the shipped `base_plugins()`
//! table, so the base's turn-in route is the production one. The base commits
//! in a spawned task and answers with a correlated result, which the test
//! awaits and applies through the cell's production consumer.
//!
//! Test-only.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cimmeria_base_session::base::helpers::derive_letter_turn_in_token;
use cimmeria_base_session::base::ConnectedClientState;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE, MISSION_COMPLETED};
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::messages::{
    BaseToCellMsg, LetterTurnInEntryEpoch, LetterTurnInResult, LetterTurnInSession,
    LetterTurnInToken,
};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::world_entry::methods::missions::query_saved_missions;
use crate::base::world_entry::methods::query_player_load_data;
use crate::base::world_entry::route_cell_message;
use crate::cell::content::{
    apply_letter_turn_in_result, build_engine, fire_dialog_open, fire_player_loaded,
    fire_stargate_crossed, load_single_chain_for_test,
};
use crate::cell::dispatch::dispatch_cell_method;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{
    load_dialog_set_maps, load_item_containers, load_mission_defs, load_step_objectives,
    load_world_rows,
};
use crate::plugins::base_plugins;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state};

/// Sentinel block for this file's accounts and players, `0x7000_8600..=0x7000_86FF`.
/// Free at HEAD (`git grep -n "0x7000_8[5-9A-F]" -- crates`). Cleanup deletes
/// by exact id, never by range.
const TEST_BASE: i32 = 0x7000_8600;

/// The `CM_INTERACT` player method index (`wireclient` `calls.rs`).
const CM_INTERACT: u16 = 74;
/// The player method index that displays a dialog (`client_methods/player.rs`).
const ON_DIALOG_DISPLAY: u16 = 105;
/// The chat frame the cell's feedback rides on.
const ON_PLAYER_COMMUNICATION: u16 = 28;
/// The processing line the cell shows before the base answers.
const PROCESSING_FEEDBACK: &str = "Checking Frost's Letter...";
/// The mission accept, step and completion frames (`client_methods`).
const MISSION_FRAMES: std::ops::RangeInclusive<u16> = 80..=82;
/// The item the Cellblock grant hands over (the real Frost's Letter row).
const LETTER_ITEM: i32 = 3730;
/// The cell entity of the one character in each scenario.
const PLAYER_EID: u32 = 1;
/// The live Marsh, tag `CmdCenter_Marsh`, template 10.
const MARSH_EID: u32 = 2;
/// The session key the staged client carries; the token is derived from it.
const KEY: [u8; 32] = [9; 32];

/// The chains the receipt path reads: the Cellblock grant (1003/1121), the
/// seeded Castle bridge (1366), the Marsh receipt (6501) and the Harset bind
/// (6502).
const RECEIPT_CHAINS: [i32; 5] = [1003, 1121, 1366, 6501, 6502];

/// The persisted mission 1360 row: status, current step, the completed and
/// active objective arrays, the failed objectives and `repeats`.
type Row1360 = (
    i32,
    Option<i32>,
    Vec<i32>,
    Vec<i32>,
    Vec<i32>,
    Vec<i32>,
    i32,
);

/// The loopback address the staged client reports to the base.
fn client_addr() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 40123))
}

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_mission_1360_receipt WHERE player_id = $1")
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
    /// The base's side of the cell channel: the correlated results arrive here.
    base_rx: mpsc::Receiver<BaseToCellMsg>,
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
        let (base_tx, base_rx) = mpsc::channel(64);
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
            base_rx,
        }
    }

    /// The production engine: every enabled seeded chain with all of its trigger
    /// rows, so chains with two triggers (6511's `mission_completed` row) fire.
    async fn production(pool: PgPool, account_id: i32, player_id: i32) -> Self {
        let mut driver = Self::new(pool.clone(), account_id, player_id, &[]).await;
        driver.engine = build_engine(Some(&pool)).await;
        driver
    }

    /// Register the live client for `PLAYER_EID` the way world entry does, and
    /// return the cell session that matches it. Without this the base treats
    /// every click as stale.
    fn connect(&self) -> LetterTurnInSession {
        let mut state = test_default_connected_client_state();
        state.key = KEY;
        state.enc = MercuryEncryption::from_session_key(KEY);
        state.player_entity_id = Some(PLAYER_EID);
        state.active_player_id = Some(self.player_id);
        state.world_name = Some("Harset_CmdCenter".to_owned());
        state.extensions.insert(LetterTurnInEntryEpoch::new(1));
        let session = LetterTurnInSession {
            player_id: self.player_id,
            token: derive_letter_turn_in_token(&KEY),
            generation: state.connected_at,
            epoch: LetterTurnInEntryEpoch::new(1),
        };
        self.entity_to_addr
            .lock()
            .unwrap()
            .insert(PLAYER_EID, client_addr());
        self.connected.lock().unwrap().insert(client_addr(), state);
        session
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
        mgr.dialog_set_maps = load_dialog_set_maps(pool).await.unwrap();
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

    /// The base's next correlated result for the accepted click.
    async fn next_result(&mut self) -> LetterTurnInResult {
        // Poll with a short sleep rather than await `recv()` under a timeout: the
        // spawned commit only finished while the test task was sleeping.
        for _ in 0..2000 {
            if let Ok(BaseToCellMsg::LetterTurnInResult(result)) = self.base_rx.try_recv() {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("the base must answer the accepted click with one correlated result")
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

    /// The persisted mission 1360 row, read directly.
    async fn row_1360(&self) -> Row1360 {
        sqlx::query_as(
            "SELECT status, current_step_id, completed_step_ids, completed_objective_ids, \
             active_objective_ids, failed_objective_ids, repeats \
             FROM sgw_mission WHERE player_id = $1 AND mission_id = 1360",
        )
        .bind(self.player_id)
        .fetch_one(&self.pool)
        .await
        .expect("mission 1360 row")
    }

    /// How many receipt rows the character has for mission 1360.
    async fn receipt_rows(&self) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM sgw_mission_1360_receipt WHERE player_id = $1 AND mission_id = 1360",
        )
        .bind(self.player_id)
        .fetch_one(&self.pool)
        .await
        .expect("receipt count")
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

/// The method indices of the mission and dialog frames in `messages`, in order.
fn frame_methods(messages: &[CellToBaseMsg]) -> Vec<u16> {
    messages
        .iter()
        .filter_map(|msg| match msg {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => Some(*method_index),
            _ => None,
        })
        .collect()
}

/// How `stage` builds the scene around Marsh.
#[derive(Clone, Copy)]
struct Stage {
    /// The base sees the live client, so a click can commit.
    connect: bool,
    /// The Castle crossing ran, so 4038 is persisted (otherwise 1360 stays on 4037).
    cross: bool,
    /// The world the character enters last.
    world: &'static str,
    /// The tag on the NPC beside the character.
    tag: &'static str,
}

impl Stage {
    /// The shipped route: crossed, in Harset beside the real Marsh, connected.
    const MARSH: Self = Self {
        connect: true,
        cross: true,
        world: "Harset_CmdCenter",
        tag: "CmdCenter_Marsh",
    };
}

/// Stage the character at Marsh's click: the Cellblock grant, the Castle
/// crossing that persists 4038, and the Harset bind. Without `connect` the
/// base does not know the client, so a click is stale.
async fn stage_at_marsh(
    driver: &Driver,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    connect: bool,
) -> SpaceManager {
    stage(
        driver,
        tx,
        rx,
        Stage {
            connect,
            ..Stage::MARSH
        },
    )
    .await
}

async fn stage(
    driver: &Driver,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    opts: Stage,
) -> SpaceManager {
    // Cellblock: the seeded grant chain hands over the letter and accepts 1360.
    let mut cellblock = driver.hydrate("Castle_CellBlock").await;
    cellblock
        .get_entity_mut(PLAYER_EID)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(622, 2113, vec![]));
    fire_dialog_open(
        PLAYER_EID,
        driver.player_id,
        3995,
        &driver.engine,
        tx,
        &mut cellblock,
    )
    .await;
    driver.route(drain(rx)).await;
    assert_eq!(driver.letter_count().await, 1, "one letter from the grant");
    assert_eq!(driver.step_1360().await, Some(4037));
    drop(cellblock);

    if opts.cross {
        // Castle: the seeded bridge chain 1366 persists 4038 on the crossing.
        let mut castle = driver.hydrate("Castle").await;
        fire_stargate_crossed(
            PLAYER_EID,
            driver.player_id,
            "Harset",
            &driver.engine,
            tx,
            &mut castle,
        )
        .await;
        driver.route(drain(rx)).await;
        assert_eq!(
            driver.step_1360().await,
            Some(4038),
            "crossing persists 4038"
        );
        drop(castle);
    }
    enter(driver, tx, rx, opts).await.harset
}

/// A fresh hydrate in `opts.world` beside the tagged NPC, the world-entry
/// bind chains fired. A relog or a second entry is this call again.
async fn enter(
    driver: &Driver,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    opts: Stage,
) -> Entered {
    let mut harset = driver.hydrate(opts.world).await;
    let space = harset.get_entity(PLAYER_EID).unwrap().space_id.0 as u32;
    harset
        .create_entity_in_space(MARSH_EID, space, [1.0, 0.0, 0.0], [0.0; 3])
        .expect("Marsh must spawn beside the player");
    harset.connect_entity(PLAYER_EID);
    let marsh = harset.get_entity_mut(MARSH_EID).unwrap();
    marsh.tag = Some(opts.tag.into());
    marsh.template_id = Some(10);
    let session = if opts.connect {
        driver.connect()
    } else {
        LetterTurnInSession {
            player_id: driver.player_id,
            token: LetterTurnInToken::from_bytes([8; 16]),
            generation: Instant::now(),
            epoch: LetterTurnInEntryEpoch::new(1),
        }
    };
    let player = harset.get_entity_mut(PLAYER_EID).unwrap();
    player.last_interaction_target = Some(MARSH_EID);
    // The turn-in session the Marsh request is bound to (the same test-side
    // seat `stage_player` gives the executor tests).
    player.extensions.insert(session);
    fire_player_loaded(
        PLAYER_EID,
        driver.player_id,
        opts.world,
        &driver.engine,
        tx,
        &mut harset,
    )
    .await;
    let queued = drain(rx);
    let methods = frame_methods(&queued);
    let requests = request_count(&queued);
    let receipt_dialog = shows_receipt_dialog(&queued);
    driver.route(queued).await;
    Entered {
        harset,
        methods,
        requests,
        receipt_dialog,
    }
}

/// What a world entry left behind: the manager and the cell's queued frames.
struct Entered {
    harset: SpaceManager,
    /// Method indices the entry queued for the client, in order.
    methods: Vec<u16>,
    /// Turn-in requests the entry queued for the base.
    requests: usize,
    /// Whether the entry displayed the receipt dialog 4576.
    receipt_dialog: bool,
}

/// Whether `messages` display the receipt dialog 4576 (the success dialog).
fn shows_receipt_dialog(messages: &[CellToBaseMsg]) -> bool {
    messages.iter().any(|msg| {
        matches!(msg, CellToBaseMsg::EntityMethodCall { method_index, args, .. }
            if *method_index == ON_DIALOG_DISPLAY
                && args.windows(4).any(|w| w == 4576i32.to_le_bytes()))
    })
}

/// How many turn-in requests `messages` carry.
fn request_count(messages: &[CellToBaseMsg]) -> usize {
    messages
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::Plugin(_)))
        .count()
}

/// Whether `messages` carry the seal's processing feedback: the chat frame on
/// the feedback channel with the `Checking Frost's Letter...` line.
fn shows_processing_feedback(messages: &[CellToBaseMsg]) -> bool {
    let want = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, PROCESSING_FEEDBACK);
    messages.iter().any(|msg| {
        matches!(msg, CellToBaseMsg::EntityMethodCall { method_index, args, .. }
            if *method_index == ON_PLAYER_COMMUNICATION && *args == want)
    })
}

/// The journal frames in `messages` as (method, leading id), in order. Each of
/// `onMissionUpdate`, `onStepUpdate` and `onObjectiveUpdate` leads with its
/// INT32 id.
fn journal_ids(messages: &[CellToBaseMsg]) -> Vec<(u16, i32)> {
    messages
        .iter()
        .filter_map(|msg| match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } if MISSION_FRAMES.contains(method_index) => Some((
                *method_index,
                i32::from_le_bytes(args.get(..4)?.try_into().ok()?),
            )),
            _ => None,
        })
        .collect()
}

/// The real click: `CM_INTERACT` on Marsh through the cell's method router.
/// Returns what the cell queued for it, unrouted.
async fn click_marsh(
    driver: &Driver,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    harset: &mut SpaceManager,
) -> Vec<CellToBaseMsg> {
    dispatch_cell_method(
        PLAYER_EID,
        CM_INTERACT,
        &(MARSH_EID as i32).to_le_bytes(),
        tx,
        harset,
        &driver.engine,
        None,
    )
    .await;
    drain(rx)
}

/// One accepted click through the base: it commits and answers with the
/// correlated result, which the cell applies. Returns the frames the cell
/// queued for the client and the base, unrouted, in the order queued.
async fn deliver(
    driver: &mut Driver,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    harset: &mut SpaceManager,
) -> Vec<CellToBaseMsg> {
    let click = click_marsh(driver, tx, rx, harset).await;
    let requests = click
        .iter()
        .filter(|m| matches!(m, CellToBaseMsg::Plugin(_)))
        .count();
    assert_eq!(requests, 1, "the click queues exactly one turn-in request");
    driver.route(click).await;
    let result = driver.next_result().await;
    apply_letter_turn_in_result(result, tx, harset, &driver.engine).await;
    drain(rx)
}

/// The positive path, once: the click commits, the correlated result consumes
/// the one letter, completes 1360 with the exact terminal arrays and `repeats`
/// 1, writes the receipt row, and the cell opens the 1361 offer.
#[tokio::test]
async fn live_db_marsh_receipt_full_replay_delivers_the_letter_once() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = (TEST_BASE + 3, TEST_BASE + 4);
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let mut driver = Driver::production(pool.clone(), account_id, player_id).await;
    let (tx, mut rx) = mpsc::channel(128);
    let mut harset = stage_at_marsh(&driver, &tx, &mut rx, true).await;

    // The click is inline (not `deliver`) so its frames are kept: the processing
    // feedback must arrive before the result, with no success frame in front.
    let click = click_marsh(&driver, &tx, &mut rx, &mut harset).await;
    assert_eq!(
        request_count(&click),
        1,
        "the click queues one turn-in request"
    );
    assert!(
        shows_processing_feedback(&click),
        "the click shows 'Checking Frost's Letter...' before the result"
    );
    assert!(
        !frame_methods(&click)
            .iter()
            .any(|m| *m == ON_DIALOG_DISPLAY || MISSION_FRAMES.contains(m)),
        "no dialog or journal frame before the result: {:?}",
        frame_methods(&click)
    );
    driver.route(click).await;
    let result = driver.next_result().await;
    apply_letter_turn_in_result(result, &tx, &mut harset, &driver.engine).await;
    let frames = drain(&mut rx);

    // The cell's frames: 4576 dialog, then the mission frames in order.
    let methods = frame_methods(&frames);
    assert_eq!(
        &methods[..4],
        &[ON_DIALOG_DISPLAY, 82, 81, 80],
        "dialog 4576, then the 4651 and 4038 frames and the 1360 completion"
    );
    assert!(
        shows_receipt_dialog(&frames),
        "the first frame displays the receipt dialog 4576"
    );
    assert_eq!(
        journal_ids(&frames),
        [(82, 4651), (81, 4038), (80, 1360)],
        "objective 4651, step 4038, then mission 1360"
    );
    // The seal (section 7) says "then one 6511 offer" as if it were a frame. It
    // is not: chain 6511 is `add_dialog_set` 5254 on slot 10, a bind, and a bind
    // pushes only the `InteractionType` flags (no dialog frame, no 1361 accept).
    // So exactly one dialog display and exactly one mission-accept-class frame.
    assert_eq!(
        methods.len(),
        4,
        "the cell result path emits only the four frames"
    );
    let player = harset.get_entity(PLAYER_EID).unwrap();
    assert!(
        player.missions.get_mission(1361).is_none(),
        "the offer is a bind: 1361 is not accepted by the delivery"
    );
    let slot10 = player.available_interactions.get(&10);
    assert!(
        slot10.is_some_and(|e| e.iter().any(|&(dsm, _, _)| dsm == 5254)),
        "6511 binds the Praxis offer 5254 on Marsh's slot 10: {slot10:?}"
    );
    assert!(
        slot10.is_some_and(|e| e.iter().all(|&(dsm, _, _)| dsm != 120101)),
        "the letter bind 120101 is gone from slot 10"
    );

    // The database: one letter consumed, the terminal row, one receipt.
    assert_eq!(driver.letter_count().await, 0, "one owned 3730 consumed");
    let (status, step, completed_steps, completed_objectives, active, failed, repeats) =
        driver.row_1360().await;
    assert_eq!(status, i32::from(MISSION_COMPLETED));
    assert_eq!(step, None);
    assert_eq!(completed_steps, [4037, 4038]);
    assert_eq!(completed_objectives, [4650, 4651]);
    // The production postimage keeps 4651 active beside the completed pair
    // (`letter_turn_in.rs` pins `[4650, 4651]` / `[4651]`).
    assert_eq!(active, [4651]);
    assert!(failed.is_empty());
    assert_eq!(repeats, 1);
    assert_eq!(driver.receipt_rows().await, 1);
    // The DSM 120101 slot-10 removal is asserted by the end state of
    // `available_interactions` above. No client-side check is made for it: a
    // non-empty frame filter for the client would also pass on the bind and
    // feedback frames already sent to that address, so it proves nothing.

    driver.route(frames).await;
    cleanup(&pool, account_id, player_id).await;
}

mod negative;
mod reaccept;
