//! CS-04a1c4: the Castle Q1 medic (mission 10300) behind the `interact`
//! range gate. The medic is the real seeded NPC (spawn 198, tag
//! `Castle_Pop_Infirmary_Medic1`), built by the startup spawn path, and the
//! click goes through the player dispatcher from the wire's method index with
//! the whole seeded chain engine, so chain 10003 is live.
//!
//! A click from 30 u is refused by the range gate in `interact.rs` before any
//! chain sees it: one `onPlayerCommunication` line, one
//! `interaction.out_of_range` row, no chain resolved. The control from 1 u
//! reaches chain 10003 and accepts the mission, so the far case cannot pass
//! because the chain was dead.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::missions::{MissionInstance, MISSION_COMPLETED};

use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::{require_db_or_skip, LogCapture};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 4243;
const MEDIC_TAG: &str = "Castle_Pop_Infirmary_Medic1";
const ACCEPT_CHAIN: &str = "10003";
const MISSION: i32 = 10300;
const PREREQ: i32 = 704;
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// The Castle with the seeded medic spawned from its record, and the player
/// `dist` metres from it holding a completed 704. Evaluates to the manager,
/// the medic's entity id and the production chain engine. A macro because this
/// crate has no `sqlx` dependency to name the pool type with.
macro_rules! stage {
    ($pool:expr, $dist:expr) => {{
        let pool = &$pool;
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<Spaces><Space WorldName="Castle" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(r#"<Spaces><Space WorldName="Castle" /></Spaces>"#)
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

        let records: Vec<_> = spawner::load_spawns_from_db(pool)
            .await
            .expect("spawns must load")
            .into_iter()
            .filter(|r| r.tag.as_deref() == Some(MEDIC_TAG))
            .collect();
        assert_eq!(records.len(), 1, "one spawn carries {MEDIC_TAG}: {records:?}");
        let record = &records[0];
        assert_eq!(record.world_name, "Castle");
        let medic = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(medic, record)
            .expect("the medic must spawn from its record");

        mgr.create_entity(
            PLAYER,
            "Castle",
            [record.x + $dist, record.y, record.z],
            [0.0; 3],
        )
        .expect("the player must stage in the Castle");
        mgr.connect_entity(PLAYER);
        let player = mgr.get_entity_mut(PLAYER).unwrap();
        player.is_player = true;
        player.player_id = Some(PLAYER_ID);
        player.archetype_id = Some(1);
        let mut done = MissionInstance::new(PREREQ, 0, Vec::new());
        done.status = MISSION_COMPLETED;
        done.current_step_id = None;
        player.missions.add_mission(done);

        let engine = crate::cell::content::build_engine(Some(pool)).await;
        (mgr, medic, engine)
    }};
}
/// Right-click the medic through the wire dispatcher; everything it sent.
async fn click(
    mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
    medic: u32,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(medic as i32).to_le_bytes(),
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

fn started(mgr: &SpaceManager) -> bool {
    mgr.get_entity(PLAYER)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .is_some()
}

/// **A medic click from 30 u reaches no chain; the same click from 1 u does.**
/// Far: exactly one `onPlayerCommunication`, exactly one INFO
/// `interaction.out_of_range` row, no log row of any kind for chain 10003
/// (neither a match nor a `condition_failed`), and 10300 not started. Near
/// (control): chain 10003 resolves and accepts 10300, and no out-of-range
/// row is logged. Revert proof: delete the `interact_target_in_range` gate in
/// `interact.rs` and the far click resolves chain 10003, so the far case
/// fails.
#[tokio::test]
async fn live_db_castle_medic_click_out_of_range_reaches_no_chain() {
    let pool = require_db_or_skip!();

    // Far.
    let (mut mgr, medic, engine) = stage!(pool, 30.0);
    let capture = LogCapture::install();
    let out = click(&mut mgr, &engine, medic).await;
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
        .filter(|c| c.has_field("chain_id", ACCEPT_CHAIN))
        .collect();
    assert!(
        chain_rows.is_empty(),
        "a refused click must not reach chain {ACCEPT_CHAIN}: {chain_rows:#?}"
    );
    assert!(!started(&mgr), "a refused click must not start {MISSION}");
    drop(capture);

    // Control: one unit away.
    let (mut mgr, medic, engine) = stage!(pool, 1.0);
    let capture = LogCapture::install();
    click(&mut mgr, &engine, medic).await;
    let all = capture.all();
    assert!(
        !all.iter()
            .any(|c| c.has_field("event", "interaction.out_of_range")),
        "the in-range click must not be refused for range"
    );
    assert!(
        all.iter().any(|c| c.has_field("chain_id", ACCEPT_CHAIN)),
        "the in-range click must reach chain {ACCEPT_CHAIN} (fired or condition_failed)"
    );
    assert!(started(&mgr), "the in-range click must accept {MISSION}");
}
