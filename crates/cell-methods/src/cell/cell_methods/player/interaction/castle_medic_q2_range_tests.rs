//! CS-04a2c2: the Castle Q2 prisoner and medic (mission 10301) behind the
//! `interact` range gate. Both are the real seeded NPCs (spawns 13806 and
//! 198, tags `Castle_Pop_Prisoner5` and `Castle_Pop_Infirmary_Medic1`), built
//! by the spawn path, and the click goes through the player dispatcher from
//! the wire's method index with the whole seeded chain engine, so chains
//! 10013 (accept) and 10015 (turn-in) are live.
//!
//! A click from 30 u is refused by the range gate in `interact.rs` before any
//! chain sees it: one `onPlayerCommunication` line, one
//! `interaction.out_of_range` row, no row for the chain. The control click
//! from 1 u reaches the chain and moves the mission, so the far case cannot
//! pass because the chain was dead. Sibling of `castle_medic_range_tests`.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};

use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::{require_db_or_skip, Captured, LogCapture};

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 4244;
const PRISONER_TAG: &str = "Castle_Pop_Prisoner5";
const MEDIC_TAG: &str = "Castle_Pop_Infirmary_Medic1";
const ACCEPT_CHAIN: &str = "10013";
const TURN_IN_CHAIN: &str = "10015";
const Q1: i32 = 10300;
const MISSION: i32 = 10301;
const STEP_B: i32 = 10343;
const OBJ_B: i32 = 10463;
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// Q1 completed (the prerequisite), and with `at_b` also Q2 active at step B.
fn missions(at_b: bool) -> Vec<MissionInstance> {
    let mut q1 = MissionInstance::new(Q1, 0, Vec::new());
    q1.status = MISSION_COMPLETED;
    q1.current_step_id = None;
    let mut out = vec![q1];
    if at_b {
        let objective = MissionObjective {
            objective_id: OBJ_B,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        };
        let mut q2 = MissionInstance::new(MISSION, STEP_B, vec![objective]);
        q2.current_step_id = Some(STEP_B);
        out.push(q2);
    }
    out
}

/// The Castle with the NPC tagged `$tag` spawned from its seeded record, and
/// the player `$dist` metres from it holding `$missions`. Evaluates to the
/// manager, the NPC's entity id and the production chain engine. A macro
/// because this crate has no `sqlx` dependency to name the pool type with.
macro_rules! stage {
    ($pool:expr, $tag:expr, $dist:expr, $missions:expr) => {{
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
            .filter(|r| r.tag.as_deref() == Some($tag))
            .collect();
        assert_eq!(records.len(), 1, "one spawn carries {}: {records:?}", $tag);
        let record = &records[0];
        assert_eq!(record.world_name, "Castle");
        let npc = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(npc, record)
            .expect("the NPC must spawn from its record");

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
        for mission in $missions {
            player.missions.add_mission(mission);
        }

        let engine = crate::cell::content::build_engine(Some(pool)).await;
        (mgr, npc, engine)
    }};
}

/// Right-click `npc` through the wire dispatcher; everything it sent.
async fn click(
    mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
    npc: u32,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(npc as i32).to_le_bytes(),
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

/// The Q2 mission's status, or `None` when the player does not hold it.
fn status(mgr: &SpaceManager) -> Option<i8> {
    mgr.get_entity(PLAYER)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .map(|m| m.status)
}

/// Assert the far click was refused by the range gate alone: one chat line,
/// one INFO `interaction.out_of_range` row (`too_far`), and no log row of any
/// kind for `chain`.
fn assert_refused_for_range(out: &[CellToBaseMsg], all: &[Captured], chain: &str) {
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
    let refused: Vec<_> = all
        .iter()
        .filter(|c| c.level == Level::INFO && c.has_field("event", "interaction.out_of_range"))
        .collect();
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(refused[0].has_field("reason", "too_far"), "{refused:#?}");
    let chain_rows: Vec<_> = all
        .iter()
        .filter(|c| c.has_field("chain_id", chain))
        .collect();
    assert!(
        chain_rows.is_empty(),
        "a refused click must not reach chain {chain}: {chain_rows:#?}"
    );
}

/// **A far prisoner or medic click reaches no chain; the near click does.**
/// Prisoner at 30 u with 10300 completed: refused for range, 10301 not
/// started; at 1 u chain 10013 accepts 10301. Medic at 30 u with 10301 active
/// at B: refused for range, 10301 still active; at 1 u chain 10015 completes
/// it. Revert proof: delete the `interact_target_in_range` gate in
/// `interact.rs` and each far click resolves its chain, so the far cases fail.
#[tokio::test]
async fn live_db_castle_side_medical_q2_out_of_range_click_refused() {
    let pool = require_db_or_skip!();

    // Far prisoner.
    let (mut mgr, npc, engine) = stage!(pool, PRISONER_TAG, 30.0, missions(false));
    let capture = LogCapture::install();
    let out = click(&mut mgr, &engine, npc).await;
    assert_refused_for_range(&out, &capture.all(), ACCEPT_CHAIN);
    assert_eq!(
        status(&mgr),
        None,
        "a refused click must not start {MISSION}"
    );
    drop(capture);

    // Near prisoner (control).
    let (mut mgr, npc, engine) = stage!(pool, PRISONER_TAG, 1.0, missions(false));
    let capture = LogCapture::install();
    click(&mut mgr, &engine, npc).await;
    let all = capture.all();
    assert!(
        !all.iter()
            .any(|c| c.has_field("event", "interaction.out_of_range")),
        "the in-range prisoner click must not be refused for range"
    );
    assert_eq!(
        status(&mgr),
        Some(MISSION_ACTIVE),
        "the in-range prisoner click must accept {MISSION}"
    );
    drop(capture);

    // Far medic at B.
    let (mut mgr, npc, engine) = stage!(pool, MEDIC_TAG, 30.0, missions(true));
    let capture = LogCapture::install();
    let out = click(&mut mgr, &engine, npc).await;
    assert_refused_for_range(&out, &capture.all(), TURN_IN_CHAIN);
    assert_eq!(
        status(&mgr),
        Some(MISSION_ACTIVE),
        "a refused click must not complete {MISSION}"
    );
    drop(capture);

    // Near medic at B (control).
    let (mut mgr, npc, engine) = stage!(pool, MEDIC_TAG, 1.0, missions(true));
    let capture = LogCapture::install();
    click(&mut mgr, &engine, npc).await;
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "interaction.out_of_range")),
        "the in-range medic click must not be refused for range"
    );
    assert_eq!(
        status(&mgr),
        Some(MISSION_COMPLETED),
        "the in-range medic click must complete {MISSION}"
    );
}
