//! `npc_ai.tick`: one row per ticked NPC per AI tick, emitted after the handler,
//! carrying where the NPC is, where it is going, the facing clients are sent,
//! and what it is fighting or following -- plus the handler's terminal outcome,
//! including `fight.rs` outcomes that used to bypass the shared vocabulary.

use super::{make_aggression_fixture, make_ai_fixture};
use crate::cell::combat::HOSTILE_FACTION;
use crate::test_support::LogCapture;
use cimmeria_common::Vector3;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

#[tokio::test]
async fn ai_tick_row_reports_state_target_facing_and_a_fight_outcome() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.create_entity(101, "Castle", [-10.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(101, 10.0);
        npc.direction = Vector3::new(0.0, 0.0, 0.0);
    }

    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;

    let row = logs
        .find_message(tracing::Level::DEBUG, "NPC AI tick")
        .expect("every ticked NPC must produce a tick row");
    assert_eq!(row.target, "npc_ai.tick");
    assert!(row.has_field("npc_id", "200"));
    assert!(row.has_field("target_id", "101"));
    assert!(row.has_field("state_before", "Fighting"));
    // Target is due west: the handler re-faced to -PI/2, which must go out as
    // byte 192 (not the saturated 0 of the old pack_angle).
    assert!(
        row.has_field("yaw_byte", "192"),
        "got {:?}",
        row.fields.get("yaw_byte")
    );
    assert!(row.has_field("world", "Castle"), "{row:?}");
    let outcome = row
        .fields
        .get("decision_outcome")
        .cloned()
        .unwrap_or_default();
    assert!(
        !outcome.is_empty() && outcome != "\"\"",
        "a fight.rs decision must reach the tick row; got {outcome:?}"
    );
}

/// Castle space with `n` hostile Idle guards (ticked every AI tick since
/// NA13) spread 30 u apart.
fn idle_guards(n: u32) -> (crate::cell::space_manager::SpaceManager, Vec<u32>) {
    use cimmeria_entity::cell_entity::AiState;
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.destroy_entity(200);
    let mut ids = Vec::new();
    for i in 0..n {
        let id = mgr.allocate_npc_id();
        mgr.spawn_npc(id, "Castle", [30.0 * i as f32, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc = mgr.get_entity_mut(id).unwrap();
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Idle);
        ids.push(id);
    }
    (mgr, ids)
}

/// Run `ticks` AI ticks; the `npc_id` of every `npc_ai.tick` row written.
async fn tick_rows(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    ticks: usize,
) -> Vec<String> {
    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(64);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    for _ in 0..ticks {
        crate::cell::service::npc_ai::npc_ai_tick(
            &tx,
            mgr,
            &crate::cell::content::EngineEvents(&engine),
        )
        .await;
    }
    logs.all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.tick" && c.message_contains("NPC AI tick"))
        .map(|c| c.fields.get("npc_id").cloned().unwrap_or_default())
        .collect()
}

/// NA24 (UAT-1 E): an empty server wrote ~14 `npc_ai.tick` rows a second,
/// one per hostile Idle guard per 2 s tick. Measured here: 10 idle guards
/// nobody can see, 6 ticks -- 60 rows before, 10 after (the first row per
/// NPC, then one per 60 s). Revert proof: make `admit_ai_tick_row` always
/// return `Some(0)` and the count is 60.
#[tokio::test]
async fn idle_unwitnessed_npcs_sample_their_tick_row() {
    let (mut mgr, ids) = idle_guards(10);
    let rows = tick_rows(&mut mgr, 6).await;
    assert_eq!(
        rows.len(),
        ids.len(),
        "one row per idle unwitnessed NPC, got {rows:?}"
    );
}

/// Sampling never touches what a player can see: an Idle guard inside a
/// player's AoI (50 u away, outside its 18 u aggro radius) writes a row on
/// every tick.
#[tokio::test]
async fn a_witnessed_idle_npc_writes_every_tick_row() {
    let (mut mgr, ids) = idle_guards(1);
    mgr.create_entity(101, "Castle", [0.0, 0.0, 50.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        p.player_id = Some(101);
    }
    mgr.connect_entity(101);
    let _ = mgr.compute_aoi_changes();
    assert_eq!(
        mgr.get_witnesses_of(ids[0]),
        vec![101],
        "fixture: the player sees the guard"
    );

    let rows = tick_rows(&mut mgr, 6).await;
    assert_eq!(rows.len(), 6, "{rows:?}");
}

/// Put guard `npc_id` on a three-waypoint loop in the Patrol state. The path
/// keeps the NPC walking, so its state holds for the ticks that follow (a
/// Patrol NPC with no route falls back to Idle, which is a state change).
fn force_patrol(mgr: &mut crate::cell::space_manager::SpaceManager, npc_id: u32) {
    use cimmeria_entity::cell_entity::AiState;
    let npc = mgr.get_entity_mut(npc_id).unwrap();
    npc.patrol_path = vec![
        Vector3::new(10.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 10.0),
        Vector3::new(0.0, 0.0, 10.0),
    ];
    npc.patrol_point_delay_secs = 2.0;
    crate::cell::service::npc_ai::force_ai_state(npc, AiState::Patrol);
}

/// TG-NPC-01: an unwitnessed Patrol NPC that keeps its state is sampled like
/// an Idle guard -- five ticks write one row. A witnessed patroller still
/// writes every tick.
#[tokio::test]
async fn unwitnessed_patrollers_sample_their_tick_row() {
    let (mut mgr, ids) = idle_guards(1);
    force_patrol(&mut mgr, ids[0]);

    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(64);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    for _ in 0..5 {
        crate::cell::service::npc_ai::npc_ai_tick(
            &tx,
            &mut mgr,
            &crate::cell::content::EngineEvents(&engine),
        )
        .await;
    }
    let suppressed: Vec<String> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.tick" && c.message_contains("NPC AI tick"))
        .map(|c| c.fields.get("suppressed").cloned().unwrap_or_default())
        .collect();
    // The window's first row goes out immediately with nothing swallowed
    // before it, so `suppressed` is 0; the four skipped ticks show up on the
    // next row, one window later.
    assert_eq!(
        suppressed,
        vec!["0".to_string()],
        "one row, first of its window"
    );

    let (mut wmgr, wids) = idle_guards(1);
    force_patrol(&mut wmgr, wids[0]);
    wmgr.create_entity(101, "Castle", [0.0, 0.0, 50.0], [0.0; 3])
        .unwrap();
    if let Some(p) = wmgr.get_entity_mut(101) {
        p.is_player = true;
        p.player_id = Some(101);
    }
    wmgr.connect_entity(101);
    let _ = wmgr.compute_aoi_changes();
    assert_eq!(wmgr.get_witnesses_of(wids[0]), vec![101], "fixture");
    let rows = tick_rows(&mut wmgr, 5).await;
    assert_eq!(
        rows.len(),
        5,
        "a witnessed patroller writes every tick: {rows:?}"
    );
}

/// TG-NPC-11: the tick on which a hostile Idle guard engages a player names
/// the world, and its outcome is `aggro_acquired` (it used to read `""`).
/// Reverting either change fails this test.
#[tokio::test]
async fn engaging_tick_row_names_the_world_and_the_aggro_outcome() {
    const NPC: u32 = 200_001;
    let mut mgr = make_aggression_fixture(NPC, HOSTILE_FACTION, 1, [5.0, 0.0, 0.0]);

    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;

    let row = logs
        .all()
        .into_iter()
        .find(|c| {
            c.target == "npc_ai.tick"
                && c.message_contains("NPC AI tick")
                && c.has_field("npc_id", "200001")
        })
        .expect("the engaging tick must write a tick row");
    assert!(row.has_field("world", "Castle"), "{row:?}");
    assert!(
        row.has_field("decision_outcome", "aggro_acquired"),
        "{row:?}"
    );
}
