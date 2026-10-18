//! The `sgw_mission` round-trip tests that drive the cell's mission code on a
//! `SpaceManager` (and, for two of them, the base's cell-to-base dispatch)
//! together with the base's `handle_mission_update` and
//! `query_saved_missions`.
//!
//! They were in `base::world_entry::methods::missions::tests`, which moved to
//! `cimmeria-base-methods` in wave B2 of
//! docs/architecture/services-crate-split.md. These three need both the cell
//! code (`cimmeria-cell-content` since wave C3) and the base world-entry code
//! (`cimmeria-base-world-entry` since wave B3), and only the facade sits
//! above both. The sentinels and the two fixture helpers are
//! the originals', so the tests touch the same rows as before.

use std::sync::Arc;

use sqlx::PgPool;

use crate::base::world_entry::methods::missions::{handle_mission_update, query_saved_missions};
use crate::test_support::require_db_or_skip;

/// Sentinel base for the mission round trips. Was `0x7000_0400`, shared
/// with `missions/tests.rs` in `cimmeria-base-methods` (#800). Offsets run
/// to `+603`, so this owns `0x7000_8200..=0x7000_84FF`.
const TEST_PLAYER_BASE: i32 = 0x7000_8200;

/// sgw_mission has a FK to sgw_player. Cleanup deletes the account, which
/// cascades sgw_player rows; sgw_mission rows go away when the player rows
/// they reference are deleted (or via the FK rule). Mission rows are also
/// deleted explicitly first to avoid relying on cascade ordering.
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
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("mission-test-{account_id}"))
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
    .bind(format!("test-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// Cross-service triangle: cell completion → CellToBaseMsg::MissionUpdate
/// (routed through `cell_dispatch::handle_cell_message`, the actual
/// production bridge) → base UPSERT → re-login query. Routing through
/// `handle_cell_message` instead of calling `handle_mission_update`
/// directly catches a regression in the dispatch arm itself — e.g.
/// dropping `repeats` from the destructure or wiring one of the Vec
/// fields to the wrong handler argument.
///
/// What the test does NOT pin: the `current_step_id`, `completed_step_ids`,
/// `completed_objective_ids`, `active_objective_ids`, `failed_objective_ids`
/// fields are all empty / None on the wire because the executor
/// (cell/content/executor.rs::CompleteMission) hardcodes them empty
/// for completion — that's the production message shape, lossy by
/// design. The cell-side `MissionInstance` does carry the completed
/// step + objective in `completed_steps` / `completed_objectives`,
/// but those values are not transmitted to base today. Adding wire
/// fidelity for them is a separate concern; this test pins the
/// EXISTING contract (lossy completion) round-trips end-to-end.
#[tokio::test]
async fn live_db_mission_completion_cell_to_base_to_db_to_relogin_round_trip() {
    use crate::base::world_entry::handle_cell_message;
    use crate::cell::messages::CellToBaseMsg;
    use crate::cell::missions::{accept_mission, complete_mission_direct};
    use crate::cell::space_manager::SpaceManager;
    use crate::test_support::TestTransport;
    use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};
    use cimmeria_mercury::transport::Transport;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 400;
    let player_id = TEST_PLAYER_BASE + 401;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let db_pool = Some(Arc::new(pool.clone()));

    // ── 1. Cell-side setup ──────────────────────────────────────
    const MISSION_ID: i32 = 0x7000_0410;
    const STEP_ID: i32 = 0x7000_0411;
    const OBJ_ID: i32 = 0x7000_0412;
    const ENTITY_ID: u32 = 1;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(64);
    let objectives = vec![MissionObjective {
        objective_id: OBJ_ID,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }];
    accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives, &tx, &mut mgr).await;

    // ── 2. Cell-side completion ─────────────────────────────────
    complete_mission_direct(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await;
    let post_repeats = mgr
        .get_entity(ENTITY_ID)
        .and_then(|e| e.missions.get_mission(MISSION_ID))
        .map(|m| m.repeats)
        .unwrap();
    assert_eq!(
        post_repeats, 1,
        "MissionInstance::complete must bump repeats (Python parity)",
    );

    // ── 3. Build the EXACT message the executor emits ───────────
    // Field set mirrors crates/cell-content/src/cell/content/executor/mod.rs
    // CompleteMission arm. Empty Vecs / None are the executor's
    // intentional contract for a "completed" message.
    let msg = CellToBaseMsg::MissionUpdate {
        player_id,
        mission_id: MISSION_ID,
        status: 2,
        current_step_id: None,
        completed_step_ids: vec![],
        completed_objective_ids: vec![],
        active_objective_ids: vec![],
        failed_objective_ids: vec![],
        repeats: post_repeats,
    };

    // ── 4. Drive through the production cell_dispatch arm ──────
    // handle_cell_message is the actual cell→base bridge — a
    // regression in its MissionUpdate destructure (dropped repeats,
    // mis-bound Vec args) breaks the test here rather than silently
    // shipping. The MissionUpdate arm only touches db_pool; the
    // transport / connected / cell_tx / minigame fields aren't used,
    // so we wire the minimum stubs.
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx_opt: Option<mpsc::Sender<crate::cell::messages::BaseToCellMsg>> = None;
    let minigame_registry = None;
    handle_cell_message(
        msg,
        &transport,
        &connected,
        &entity_to_addr,
        &cell_tx_opt,
        &db_pool,
        &minigame_registry,
        "127.0.0.1",
        0,
    )
    .await;

    // ── 5. Re-login: query_saved_missions reads the row back ────
    let restored = query_saved_missions(&db_pool, player_id).await;
    let mission = restored
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("mission_id must round-trip across cell→base→DB");
    assert_eq!(mission.status, 2, "status round-trip");
    assert_eq!(
        mission.repeats, post_repeats,
        "repeats from cell-side bump must round-trip via the cell_dispatch bridge",
    );
    assert!(
        mission.current_step_id.is_none(),
        "completion clears current_step_id"
    );
    // Pin the executor's lossy contract end-to-end: the wire shape
    // sends empty Vecs, so the DB row + re-login query reflect that.
    assert!(mission.completed_step_ids.is_empty());
    assert!(mission.completed_objective_ids.is_empty());
    assert!(mission.active_objective_ids.is_empty());
    assert!(mission.failed_objective_ids.is_empty());

    cleanup(&pool, account_id, player_id).await;
}

/// Repeat-counter cross-service round-trip: complete the mission twice
/// across two cell→base→DB cycles. Pins that the post-bump `repeats`
/// from the SECOND completion is what the DB row carries — the
/// counter advances rather than stalling. A regression here would
/// silently re-strand repeatable missions even with the
/// `EXCLUDED.repeats` UPSERT in place, because the cell→base bridge
/// or the UPSERT could regress independently.
#[tokio::test]
async fn live_db_second_completion_advances_persisted_repeats_counter_end_to_end() {
    use crate::cell::missions::{accept_mission, complete_mission_direct};
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};
    use tokio::sync::mpsc;

    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 500;
    let player_id = TEST_PLAYER_BASE + 501;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;
    let db_pool = Some(Arc::new(pool.clone()));

    const MISSION_ID: i32 = 0x7000_0510;
    const STEP_ID: i32 = 0x7000_0511;
    const OBJ_ID: i32 = 0x7000_0512;
    const ENTITY_ID: u32 = 1;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(ENTITY_ID, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    // Repeatable def (num_repeats=2 → two completions stay under the
    // `repeats > num_repeats` cap) so the post-relog re-accept passes
    // the offer guard. Without a def the guard fails closed.
    mgr.mission_defs.insert(
        MISSION_ID,
        crate::cell::spawner::MissionDefEntry {
            step_id: STEP_ID,
            objectives: vec![],
            is_hidden: false,
            num_repeats: 2,
            can_repeat_on_fail: true,
        },
    );

    let (tx, _rx) = mpsc::channel::<crate::cell::messages::CellToBaseMsg>(64);
    let objectives = || {
        vec![MissionObjective {
            objective_id: OBJ_ID,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }]
    };

    // ── First completion: repeats 0 → 1 ─────────────────────────
    accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives(), &tx, &mut mgr).await;
    complete_mission_direct(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await;
    let after_first = mgr
        .get_entity(ENTITY_ID)
        .and_then(|e| e.missions.get_mission(MISSION_ID))
        .map(|m| m.repeats)
        .unwrap();
    assert_eq!(after_first, 1, "first completion bumps repeats to 1");
    handle_mission_update(
        player_id,
        MISSION_ID,
        2,
        None,
        &[],
        &[],
        &[],
        &[],
        after_first,
        &db_pool,
    )
    .await;

    // ── Simulate a relog: drop the cell-side mission and rebuild
    //    it from query_saved_missions, mirroring what
    //    cimmeria-cell's cell/service/base_messages/player_init does on
    //    InitPlayerState.
    //    This pins the production restore path as well as the
    //    write path — if `query_saved_missions` (or its column
    //    mapping) drops `repeats`, the seeded MissionInstance
    //    starts at 0 and the second cycle ends at 1 instead of 2.
    {
        let entity = mgr.get_entity_mut(ENTITY_ID).unwrap();
        // Wipe in-memory state: a fresh login starts with no missions.
        entity.missions = Default::default();
    }
    let saved = query_saved_missions(&db_pool, player_id).await;
    let saved_for_us = saved
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("relog must find the persisted mission");
    assert_eq!(
        saved_for_us.repeats, 1,
        "relog round-trip must read back the persisted repeats — \
         a 0 here means query_saved_missions's column mapping dropped the field"
    );
    // Seed cell-side from SavedMission, matching the production
    // path in cell::service::base_messages::InitPlayerState.
    {
        use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE};
        let entity = mgr.get_entity_mut(ENTITY_ID).unwrap();
        let objs: Vec<MissionObjective> = saved_for_us
            .active_objective_ids
            .iter()
            .map(|&oid| MissionObjective {
                objective_id: oid,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            })
            .collect();
        let mut mi = MissionInstance::new(
            saved_for_us.mission_id,
            saved_for_us.current_step_id.unwrap_or(0),
            objs,
        );
        mi.status = saved_for_us.status;
        mi.completed_steps = saved_for_us.completed_step_ids.clone();
        mi.completed_objectives = saved_for_us.completed_objective_ids.clone();
        // The load-bearing field — `accept_mission` will read
        // m.repeats off this seeded instance and carry it forward.
        mi.repeats = saved_for_us.repeats;
        entity.missions.add_mission(mi);
    }

    // ── Re-accept (now reads `repeats` off the DB-restored state)
    //    and complete again: the bump should go 1 → 2.
    accept_mission(ENTITY_ID, MISSION_ID, STEP_ID, objectives(), &tx, &mut mgr).await;
    complete_mission_direct(ENTITY_ID, MISSION_ID, &tx, &mut mgr).await;
    let after_second = mgr
        .get_entity(ENTITY_ID)
        .and_then(|e| e.missions.get_mission(MISSION_ID))
        .map(|m| m.repeats)
        .unwrap();
    assert_eq!(
        after_second, 2,
        "second completion advances repeats from the DB-restored value: \
         re-accept must carry forward what query_saved_missions returned"
    );
    handle_mission_update(
        player_id,
        MISSION_ID,
        2,
        None,
        &[],
        &[],
        &[],
        &[],
        after_second,
        &db_pool,
    )
    .await;

    // ── Final relog query: DB row carries the post-second-bump count ─
    let restored = query_saved_missions(&db_pool, player_id).await;
    let mission = restored
        .iter()
        .find(|m| m.mission_id == MISSION_ID)
        .expect("mission must round-trip");
    assert_eq!(
        mission.repeats, 2,
        "DB row must carry the cell's post-second-completion repeats — \
         a 1 here means the UPSERT regressed to dropping the column on conflict",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Fresh entities use the production world-entry mission hydrator; the base
/// inventory remains a separate query because carried bags live on the base.
async fn fresh_letter_entity(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    world: &str,
) -> crate::cell::space_manager::SpaceManager {
    use crate::cell::spawner::{
        load_item_containers, load_mission_defs, load_step_objectives, load_world_rows,
    };
    let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
    mgr.parse_spaces_xml(&format!(
        r#"<Spaces><Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#
    )).unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.stamp_world_rows(&load_world_rows(pool).await.unwrap());
    mgr.mission_defs = load_mission_defs(pool).await.unwrap();
    mgr.step_objectives = load_step_objectives(pool).await.unwrap();
    mgr.item_containers = load_item_containers(pool).await.unwrap();
    mgr.create_entity(1, world, [0.0; 3], [0.0; 3]).unwrap();
    let db_pool = Some(Arc::new(pool.clone()));
    let saved = query_saved_missions(&db_pool, player_id).await;
    let restored = cimmeria_cell::cell::build_restored_missions_for_test(&saved, &mgr);
    let load = crate::base::world_entry::methods::query_player_load_data(
        &db_pool,
        account_id as u32,
        player_id,
    )
    .await;
    assert_eq!(
        load.player_id, player_id,
        "production inventory/player hydration"
    );
    let entity = mgr.get_entity_mut(1).unwrap();
    entity.player_id = Some(player_id);
    entity.archetype_id = Some(load.archetype);
    entity.missions.mark_loaded();
    for mission in restored {
        entity.missions.add_mission(mission);
    }
    mgr
}

async fn letter_inventory(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
) -> Vec<(i32, i32, i32, i32)> {
    let load = crate::base::world_entry::methods::query_player_load_data(
        &Some(Arc::new(pool.clone())),
        account_id as u32,
        player_id,
    )
    .await;
    assert_eq!(load.player_id, player_id);
    load.items
        .into_iter()
        .map(|i| {
            assert_eq!(i.dbid, 3730, "only the real Cellblock letter was granted");
            (i.id, i.stack_size, i.container_id, i.slot_id)
        })
        .collect()
}

fn drain_letter_traffic(
    rx: &mut tokio::sync::mpsc::Receiver<crate::cell::messages::CellToBaseMsg>,
) -> Vec<crate::cell::messages::CellToBaseMsg> {
    let mut messages = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    messages
}

/// Route the inspected, actual messages through the production cell/base bridge.
async fn persist_letter_traffic(
    pool: &PgPool,
    messages: Vec<crate::cell::messages::CellToBaseMsg>,
) {
    use crate::test_support::TestTransport;
    use cimmeria_mercury::transport::Transport;
    use std::collections::HashMap;
    use std::sync::Mutex;
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    for msg in messages {
        crate::base::world_entry::handle_cell_message(
            msg,
            &transport,
            &connected,
            &entity_to_addr,
            &None,
            &Some(Arc::new(pool.clone())),
            &None,
            "127.0.0.1",
            0,
        )
        .await;
    }
}

/// CS-08a1: the seeded Cellblock grant/accept reaches the Castle crossing,
/// persists 4038, and hydrates in Harset with the same letter instance.
/// Removing bridge 1366 must strand this unchanged test at 4037.
#[tokio::test]
async fn live_db_frosts_letter_seeded_bridge_round_trip() {
    use crate::cell::content::{
        fire_dialog_open, fire_stargate_crossed, load_single_chain_for_test,
    };
    use crate::cell::messages::CellToBaseMsg;
    use cimmeria_content_engine::chain::ChainEngine;
    use cimmeria_entity::missions::{MissionInstance, MISSION_ACTIVE, STATUS_ACTIVE};
    use tokio::sync::mpsc;
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 600;
    let player_id = TEST_PLAYER_BASE + 601;
    let other_account = TEST_PLAYER_BASE + 602;
    let other_player = TEST_PLAYER_BASE + 603;
    for (a, p) in [(account_id, player_id), (other_account, other_player)] {
        cleanup(&pool, a, p).await;
        insert_account_and_player(&pool, a, p).await;
    }
    let mut engine = ChainEngine::new();
    for id in [1003, 1121, 1366] {
        // A missing bridge remains missing: reachability below must fail at
        // 4037, rather than substituting a handwritten advance action.
        if let Some(chain) = load_single_chain_for_test(&pool, id).await.unwrap() {
            engine.register_chain(chain);
        }
    }
    let mut cellblock = fresh_letter_entity(&pool, account_id, player_id, "Castle_CellBlock").await;
    cellblock
        .get_entity_mut(1)
        .unwrap()
        .missions
        .add_mission(MissionInstance::new(622, 2113, vec![]));
    let (tx, mut rx) = mpsc::channel(128);
    fire_dialog_open(1, player_id, 3995, &engine, &tx, &mut cellblock).await;
    let grant = drain_letter_traffic(&mut rx);
    assert_eq!(
        grant
            .iter()
            .filter(|m| matches!(
                m,
                CellToBaseMsg::GrantItem {
                    item_id: 3730,
                    container_id: 2,
                    count: 1,
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(
        grant
            .iter()
            .filter(|m| matches!(
                m,
                CellToBaseMsg::MissionUpdate {
                    mission_id: 1360,
                    current_step_id: Some(4037),
                    ..
                }
            ))
            .count(),
        1
    );
    persist_letter_traffic(&pool, grant).await;
    let letter = letter_inventory(&pool, account_id, player_id).await;
    assert_eq!(letter.len(), 1);
    assert_eq!(
        (letter[0].1, letter[0].2),
        (1, 2),
        "one letter in mission bag"
    );
    assert!(letter_inventory(&pool, other_account, other_player)
        .await
        .is_empty());
    drop(cellblock);

    let mut castle = fresh_letter_entity(&pool, account_id, player_id, "Castle").await;
    assert_eq!(castle.get_entity_world_id(1), Some(8));
    let accepted = castle
        .get_entity(1)
        .unwrap()
        .missions
        .get_mission(1360)
        .unwrap();
    assert_eq!(
        (accepted.status, accepted.current_step_id),
        (MISSION_ACTIVE, Some(4037))
    );
    assert_eq!(accepted.active_objectives[0].objective_id, 4650);
    fire_stargate_crossed(1, player_id, "Harset", &engine, &tx, &mut castle).await;
    assert_eq!(
        castle
            .get_entity(1)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap()
            .current_step_id,
        Some(4038),
        "real Castle crossing must reach delivery; missing bridge strands 4037"
    );
    let crossed = drain_letter_traffic(&mut rx);
    assert!(
        crossed.iter().all(|m| matches!(
            m,
            CellToBaseMsg::MissionUpdate { .. } | CellToBaseMsg::EntityMethodCall { .. }
        )),
        "Castle must emit no inventory/binding/transport mutation: {crossed:?}"
    );
    let updates: Vec<_> = crossed
        .iter()
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::MissionUpdate {
                    mission_id: 1360,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(updates.len(), 1);
    assert!(matches!(updates[0], CellToBaseMsg::MissionUpdate {
        status: MISSION_ACTIVE, current_step_id: Some(4038),
        completed_step_ids, completed_objective_ids, active_objective_ids,
        failed_objective_ids, repeats: 0, ..
    } if completed_step_ids == &[4037] && completed_objective_ids == &[4650]
        && active_objective_ids == &[4651] && failed_objective_ids.is_empty()));
    persist_letter_traffic(&pool, crossed).await;
    let db_pool = Some(Arc::new(pool.clone()));
    let saved = query_saved_missions(&db_pool, player_id).await;
    let delivery = saved.iter().find(|m| m.mission_id == 1360).unwrap();
    assert_eq!(delivery.current_step_id, Some(4038));
    assert_eq!(delivery.completed_step_ids, vec![4037]);
    assert_eq!(delivery.completed_objective_ids, vec![4650]);
    assert_eq!(delivery.active_objective_ids, vec![4651]);
    assert_eq!(letter_inventory(&pool, account_id, player_id).await, letter);
    drop(castle);

    // Travel and ordinary relog both rebuild from saved rows. Even a relog
    // back in Castle cannot re-credit the crossing once 4038 is persisted.
    for world in ["Harset", "Harset", "Castle"] {
        let mut fresh = fresh_letter_entity(&pool, account_id, player_id, world).await;
        let mission = fresh
            .get_entity(1)
            .unwrap()
            .missions
            .get_mission(1360)
            .unwrap();
        assert_eq!(
            (mission.status, mission.current_step_id),
            (MISSION_ACTIVE, Some(4038))
        );
        assert_eq!(mission.completed_steps, vec![4037]);
        assert_eq!(mission.completed_objectives, vec![4650]);
        assert_eq!(mission.active_objectives.len(), 1);
        assert_eq!(
            (
                mission.active_objectives[0].objective_id,
                mission.active_objectives[0].status
            ),
            (4651, STATUS_ACTIVE)
        );
        fire_stargate_crossed(1, player_id, "Harset", &engine, &tx, &mut fresh).await;
        assert!(drain_letter_traffic(&mut rx).is_empty());
        assert_eq!(letter_inventory(&pool, account_id, player_id).await, letter);
    }
    assert!(query_saved_missions(&db_pool, other_player)
        .await
        .is_empty());
    assert!(letter_inventory(&pool, other_account, other_player)
        .await
        .is_empty());
    for (a, p) in [(account_id, player_id), (other_account, other_player)] {
        cleanup(&pool, a, p).await;
    }
}
