//! CS-04c1c2: the Marsh click of the return_q1 side story (mission 10320,
//! chain 10132) behind the `interact` range gate. Marsh is the real seeded
//! NPC (tag `Castle_ColMarsh`), built by the startup spawn path, and the click
//! goes through the player dispatcher from the wire's method index with the
//! whole seeded chain engine, so chain 10132 is live.
//!
//! A click from 30 u is refused by the range gate in `interact.rs` before any
//! chain sees it: one `onPlayerCommunication` line, one
//! `interaction.out_of_range` row, no chain resolved, and 10320 untouched at
//! step A. The control from 1 u reaches chain 10132, so the far case cannot
//! pass because the chain was dead.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
};

use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::{require_db_or_skip, LogCapture};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 4244;
const MARSH_TAG: &str = "Castle_ColMarsh";
const REPORT_CHAIN: &str = "10132";
const MISSION: i32 = 10320;
const STEP_A: i32 = 10372;
const OBJ_A: i32 = 10492;
const PREREQ: i32 = 708;
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// Marsh spawned from his seeded record, and the player `dist` metres from
/// him holding a completed 708 and 10320 at step A. Evaluates to the manager,
/// Marsh's entity id and the production chain engine. A macro because this
/// crate has no `sqlx` dependency to name the pool type with.
macro_rules! stage {
    ($pool:expr, $dist:expr) => {{
        let pool = &$pool;
        let mut mgr = SpaceManager::new(1);
        let records: Vec<_> = spawner::load_spawns_from_db(pool)
            .await
            .expect("spawns must load")
            .into_iter()
            .filter(|r| r.tag.as_deref() == Some(MARSH_TAG))
            .collect();
        assert_eq!(records.len(), 1, "one spawn carries {MARSH_TAG}: {records:?}");
        let record = &records[0];
        let world = record.world_name.clone();
        mgr.parse_spaces_xml(&format!(
            r#"<Spaces><Space WorldName="{world}" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#
        ))
        .unwrap();
        mgr.create_startup_spaces(&format!(r#"<Spaces><Space WorldName="{world}" /></Spaces>"#))
            .unwrap();
        mgr.stamp_world_rows(&spawner::load_world_rows(pool).await.expect("world rows"));
        mgr.mission_defs = spawner::load_mission_defs(pool).await.expect("mission defs");
        mgr.step_objectives = spawner::load_step_objectives(pool)
            .await
            .expect("step objectives");
        mgr.item_containers = spawner::load_item_containers(pool)
            .await
            .expect("item containers");
        mgr.dialog_set_maps = spawner::load_dialog_set_maps(pool)
            .await
            .expect("dialog set maps");
        mgr.monologue_dialog_ids = spawner::load_monologue_dialog_ids(pool)
            .await
            .expect("monologue ids");
        let marsh = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(marsh, record)
            .expect("Marsh must spawn from his record");

        mgr.create_entity(
            PLAYER,
            &world,
            [record.x + $dist, record.y, record.z],
            [0.0; 3],
        )
        .expect("the player must stage next to Marsh");
        mgr.connect_entity(PLAYER);
        let player = mgr.get_entity_mut(PLAYER).unwrap();
        player.is_player = true;
        player.player_id = Some(PLAYER_ID);
        player.archetype_id = Some(1);
        let mut done = MissionInstance::new(PREREQ, 0, Vec::new());
        done.status = MISSION_COMPLETED;
        done.current_step_id = None;
        player.missions.add_mission(done);
        let objectives = vec![MissionObjective {
            objective_id: OBJ_A,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }];
        let mut report = MissionInstance::new(MISSION, STEP_A, objectives);
        report.current_step_id = Some(STEP_A);
        player.missions.add_mission(report);

        let engine = crate::cell::content::build_engine(Some(pool)).await;
        (mgr, marsh, engine)
    }};
}

/// Right-click Marsh through the wire dispatcher; everything it sent.
async fn click(
    mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
    marsh: u32,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(marsh as i32).to_le_bytes(),
            &tx,
            mgr,
            engine
        )
        .await
    );
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// 10320's current step, `None` once it has left step A.
fn step_of(mgr: &SpaceManager) -> Option<i32> {
    mgr.get_entity(PLAYER)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .and_then(|m| m.current_step_id)
}

/// **A Marsh click from 30 u reaches no chain; the same click from 1 u does.**
/// Far: exactly one `onPlayerCommunication`, exactly one INFO
/// `interaction.out_of_range` row, no log row of any kind for chain 10132
/// (neither a match nor a `condition_failed`), and 10320 still at step A.
/// Near (control): chain 10132 resolves and moves 10320 off step A, and no
/// out-of-range row is logged. Revert proof: delete the
/// `interact_target_in_range` gate in `interact.rs` and the far click resolves
/// chain 10132, so the far case fails.
#[tokio::test]
async fn live_db_castle_return_marsh_click_out_of_range_reaches_no_chain() {
    let pool = require_db_or_skip!();

    // Far.
    let (mut mgr, marsh, engine) = stage!(pool, 30.0);
    let capture = LogCapture::install();
    let out = click(&mut mgr, &engine, marsh).await;
    let calls: Vec<u16> = out
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => Some(*method_index),
            _ => None,
        })
        .collect();
    assert_eq!(
        calls,
        vec![ON_PLAYER_COMMUNICATION],
        "a too-far click gets exactly one chat line and nothing else: {out:?}"
    );
    let all = capture.all();
    let refused: Vec<_> = all
        .iter()
        .filter(|c| c.level == Level::INFO && c.has_field("event", "interaction.out_of_range"))
        .collect();
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(refused[0].has_field("reason", "too_far"), "{refused:#?}");
    let chain_rows: Vec<_> = all
        .iter()
        .filter(|c| c.has_field("chain_id", REPORT_CHAIN))
        .collect();
    assert!(
        chain_rows.is_empty(),
        "a refused click must not reach chain {REPORT_CHAIN}: {chain_rows:#?}"
    );
    assert_eq!(
        step_of(&mgr),
        Some(STEP_A),
        "a refused click must leave {MISSION} at step A"
    );
    drop(capture);

    // Control: one unit away.
    let (mut mgr, marsh, engine) = stage!(pool, 1.0);
    let capture = LogCapture::install();
    click(&mut mgr, &engine, marsh).await;
    let all = capture.all();
    assert!(
        !all.iter()
            .any(|c| c.has_field("event", "interaction.out_of_range")),
        "the in-range click must not be refused for range"
    );
    assert!(
        all.iter().any(|c| c.has_field("chain_id", REPORT_CHAIN)),
        "the in-range click must reach chain {REPORT_CHAIN} (fired or condition_failed)"
    );
    assert_ne!(
        step_of(&mgr),
        Some(STEP_A),
        "the in-range click must complete step A of {MISSION}"
    );
}
