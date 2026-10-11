//! TG-DB-12 ordering through `handle_base_message`: the cell holds mission
//! changes from `CreateEntity` until `InitPlayerState` says how the saved
//! missions loaded; `MissionStateUnknown` before `InitPlayerState` keeps a
//! `player_loaded` mission chain from running on the empty tracker; and
//! `SavedMissionsRecovered` replays only that dropped chain.

use super::super::super::handle_base_message;
use super::super::*;
use crate::cell::messages::BaseToCellMsg;
use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::Chain;
use cimmeria_content_engine::triggers::Trigger;
use cimmeria_entity::missions::MISSION_ACTIVE;

const WORLD: &str = "Castle_CellBlock";
const ENTITY: u32 = 7001;
const PLAYER_ID: i32 = 4100;
const MISSION: i32 = 622;
const ACCEPT_CHAIN: i64 = 9101;
const COUNTER_CHAIN: i64 = 9102;

fn manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.mission_defs.insert(
        MISSION,
        crate::cell::spawner::MissionDefEntry {
            step_id: 2113,
            objectives: vec![],
            is_hidden: false,
            num_repeats: 1,
            can_repeat_on_fail: true,
        },
    );
    mgr
}

/// A `player_loaded` chain that accepts the mission, and one that only
/// bumps a counter (it must run once, never twice).
fn engine() -> ChainEngine {
    let mut engine = ChainEngine::new();
    for (id, action) in [
        (
            ACCEPT_CHAIN,
            Action::AcceptMission {
                mission_id: MISSION,
            },
        ),
        (
            COUNTER_CHAIN,
            Action::IncrementCounter {
                counter_name: "entered".into(),
                amount: 1,
            },
        ),
    ] {
        engine.register_chain(Chain {
            action_delays: Vec::new(),
            id,
            name: format!("tg-db-12 order {id}"),
            enabled: true,
            trigger: Trigger::OnPlayerLoaded { world_name: None },
            conditions: vec![],
            actions: vec![action],
            priority: 0,
            once: false,
        });
    }
    engine
}

async fn send(
    msg: BaseToCellMsg,
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    handle_base_message(msg, tx, mgr, engine, &[]).await;
}

async fn create(mgr: &mut SpaceManager, engine: &ChainEngine, tx: &mpsc::Sender<CellToBaseMsg>) {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    send(
        BaseToCellMsg::CreateEntity {
            entity_id: ENTITY,
            world_name: WORLD.into(),
            position: [0.0; 3],
            rotation: [0.0; 3],
            destination_space_id: None,
            account_id: Some(6),
            player_id: Some(PLAYER_ID),
            account_name: Some("acct".into()),
            player_name: Some("Daniel".into()),
            reply_tx,
        },
        mgr,
        engine,
        tx,
    )
    .await;
    reply_rx.await.expect("cell must ack the create");
}

fn init_player_state() -> BaseToCellMsg {
    BaseToCellMsg::InitPlayerState {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
        account_id: 6,
        world_name: WORLD.into(),
        archetype_id: 1,
        saved_missions: vec![],
        abilities: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
        access_level: 0,
        known_stargates: vec![],
        tree_progress: Default::default(),
        level: 1,
        character_name: Some("Daniel".into()),
        body_set: None,
        looted_containers: Vec::new(),
        shown_tutorials: Vec::new(),
    }
}

fn mission_status(mgr: &SpaceManager) -> Option<i8> {
    mgr.get_entity(ENTITY)?
        .missions
        .get_mission(MISSION)
        .map(|m| m.status)
}

fn counter(mgr: &SpaceManager) -> i32 {
    mgr.get_entity(ENTITY)
        .and_then(|e| e.counters.get("entered").copied())
        .unwrap_or(0)
}

fn mission_updates(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> usize {
    let mut n = 0;
    while let Ok(m) = rx.try_recv() {
        if matches!(m, CellToBaseMsg::MissionUpdate { mission_id, .. } if mission_id == MISSION) {
            n += 1;
        }
    }
    n
}

/// `MissionStateUnknown`, then `InitPlayerState`: the `player_loaded`
/// accept is refused (no mission, no save) while the counter chain runs.
/// Recovery replays only the accept chain, so the counter stays at 1.
#[tokio::test]
async fn unknown_before_init_refuses_player_loaded_chain_then_recovery_replays_it() {
    let mut mgr = manager();
    let engine = engine();
    let (tx, mut rx) = mpsc::channel(512);

    create(&mut mgr, &engine, &tx).await;
    // The pre-lock window: the load outcome is not known yet.
    let objectives = vec![];
    assert!(
        !crate::cell::missions::accept_mission(ENTITY, MISSION, 2113, objectives, &tx, &mut mgr)
            .await,
        "a mission change before InitPlayerState must wait"
    );

    send(
        BaseToCellMsg::MissionStateUnknown { entity_id: ENTITY },
        &mut mgr,
        &engine,
        &tx,
    )
    .await;
    send(init_player_state(), &mut mgr, &engine, &tx).await;

    assert_eq!(mission_status(&mgr), None, "the locked accept must not run");
    assert_eq!(counter(&mgr), 1, "the non-mission chain still runs");
    assert_eq!(mission_updates(&mut rx), 0, "nothing saved while locked");

    send(
        BaseToCellMsg::SavedMissionsRecovered {
            entity_id: ENTITY,
            saved_missions: vec![],
        },
        &mut mgr,
        &engine,
        &tx,
    )
    .await;

    assert_eq!(
        mission_status(&mgr),
        Some(MISSION_ACTIVE),
        "recovery must replay the dropped player_loaded accept"
    );
    assert_eq!(counter(&mgr), 1, "a chain that ran unlocked is not re-run");
    assert!(
        mission_updates(&mut rx) >= 1,
        "the replayed accept is saved"
    );
}

/// On a successful load the `CreateEntity` hold is lifted before
/// `player_loaded` fires, so its accept runs normally.
#[tokio::test]
async fn successful_load_lifts_the_hold_before_player_loaded() {
    let mut mgr = manager();
    let engine = engine();
    let (tx, _rx) = mpsc::channel(512);

    create(&mut mgr, &engine, &tx).await;
    send(init_player_state(), &mut mgr, &engine, &tx).await;

    assert!(mgr.get_entity(ENTITY).unwrap().saved_missions_loaded);
    assert_eq!(mission_status(&mgr), Some(MISSION_ACTIVE));
    assert_eq!(counter(&mgr), 1);
}
