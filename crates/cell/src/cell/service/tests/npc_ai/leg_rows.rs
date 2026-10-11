//! TG-NPC-09: an unwitnessed patroller's leg rows are sampled, and its path
//! failures are never sampled.

use super::make_ai_fixture;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;
use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tokio::sync::mpsc;

/// One unwitnessed Patrol guard at the origin on `path`, with a 2 s dwell.
fn patroller(path: Vec<Vector3>) -> (SpaceManager, u32) {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.destroy_entity(200);
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc(id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let npc = mgr.get_entity_mut(id).unwrap();
    npc.faction = crate::cell::combat::HOSTILE_FACTION;
    npc.patrol_path = path;
    npc.patrol_point_delay_secs = 2.0;
    crate::cell::service::npc_ai::force_ai_state(npc, AiState::Patrol);
    (mgr, id)
}

/// Run one AI tick over every NPC in `mgr`.
async fn tick_once(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>) {
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    crate::cell::service::npc_ai::npc_ai_tick(
        tx,
        mgr,
        &crate::cell::content::EngineEvents(&engine),
    )
    .await;
}

/// Three legs of a loop, each ending on its waypoint. Three `patrol_arrived`
/// rows happen, but the window samples them to one row (`suppressed = 0`).
/// Revert proof: make `admit_leg_row` return `Some(None)` and the count is 3.
#[tokio::test]
async fn unwitnessed_patroller_samples_leg_rows() {
    let (mut mgr, id) = patroller(vec![
        Vector3::new(10.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 10.0),
        Vector3::new(0.0, 0.0, 10.0),
    ]);
    let logs = LogCapture::install();
    let (tx, _rx) = mpsc::channel(64);
    for leg in 0..3 {
        let waypoint = mgr.get_entity(id).unwrap().patrol_path[leg];
        if let Some(npc) = mgr.get_entity_mut(id) {
            npc.position = waypoint;
            npc.nav_path.clear();
            npc.patrol_dwell_until = None;
        }
        // Arrival: stamps the dwell deadline and writes `patrol_arrived`.
        tick_once(&mut mgr, &tx).await;
        // Dwell over: the next tick advances to the following waypoint.
        if let Some(npc) = mgr.get_entity_mut(id) {
            npc.patrol_dwell_until = Some(std::time::Instant::now());
        }
        tick_once(&mut mgr, &tx).await;
    }
    let arrived: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai" && c.message_contains("patrol → arrived"))
        .collect();
    assert_eq!(
        arrived.len(),
        1,
        "three legs of an unwitnessed patroller give one arrival row: {arrived:?}"
    );

    // A path failure is never sampled: an unreachable leg still writes its
    // `npc_ai.path_fail` row on the first tick.
    let (mut fmgr, _) = patroller(vec![Vector3::new(500.0, 0.0, 500.0)]);
    let flogs = LogCapture::install();
    tick_once(&mut fmgr, &tx).await;
    let fails = flogs
        .all()
        .into_iter()
        .filter(|c| c.target == "npc_ai.path_fail")
        .count();
    assert!(fails >= 1, "a path_fail is still written: {fails}");
}
