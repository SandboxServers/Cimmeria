//! `npc_ai_patrol` state-machine and waypoint-loop tests.
//!
//! Covers:
//! - Idle NPC with a non-empty `patrol_path` transitions to Patrol and
//!   queues the first waypoint into `nav_path` on the same tick.
//! - Empty `nav_path` + elapsed dwell → advance `patrol_next_index`
//!   (modulo path length) and queue the next waypoint.
//! - Threat preemption during Patrol → Fighting; `patrol_next_index`
//!   persists so the post-fight return resumes the route.
//! - Empty `patrol_path` mid-tick (defensive) → drop back to Idle.

use crate::cell::space_manager::SpaceManager;
use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::{AiState, MobMovementType};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

fn make_castle_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// Seed a patrol NPC at id=200 with the given 3-waypoint loop.
fn spawn_patrol_npc(mgr: &mut SpaceManager, npc_id: u32, spawn_pos: [f32; 3]) -> Vec<Vector3> {
    let path = vec![
        Vector3::new(10.0, 0.0, 0.0),
        Vector3::new(10.0, 0.0, 10.0),
        Vector3::new(0.0, 0.0, 10.0),
    ];
    mgr.spawn_npc(npc_id, "Castle", spawn_pos, [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.class_id = 0x04;
        npc.is_player = false;
        npc.patrol_path = path.clone();
        npc.patrol_point_delay_secs = 2.0;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    path
}

/// Idle NPC with a non-empty patrol path transitions to Patrol on the
/// very next AI tick and queues the first waypoint. Pin: the
/// snapshot filter must admit Idle-with-patrol, and the
/// Patrol-handler-on-same-tick fall-through must run so the NPC isn't
/// stuck Patrol-but-idle for one tick.
#[tokio::test]
async fn idle_npc_with_patrol_path_transitions_to_patrol_same_tick() {
    let mut mgr = make_castle_mgr();
    spawn_patrol_npc(&mut mgr, 200, [0.0; 3]);
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.ai_state,
        AiState::Patrol,
        "Idle + patrol_path → Patrol on same tick",
    );
    assert!(
        !npc.nav_path.is_empty(),
        "First waypoint must be queued into nav_path on the same tick (no wasted tick)",
    );
    assert_eq!(
        npc.last_movement_type,
        Some(MobMovementType::Patrol),
        "Patrol entry must broadcast MobMovementType::Patrol",
    );
}

/// Empty `nav_path` (NPC arrived at waypoint) + dwell deadline elapsed
/// → advance `patrol_next_index` modulo path length AND queue the next
/// waypoint in the same tick (no wasted tick at each waypoint).
#[tokio::test]
async fn patrol_advances_index_when_dwell_elapses_at_waypoint() {
    let mut mgr = make_castle_mgr();
    let path = spawn_patrol_npc(&mut mgr, 200, [0.0; 3]);
    // Pre-arrange: NPC is in Patrol, physically AT waypoint 0
    // (so `close == true`), dwell deadline in the past. The handler
    // should observe "elapsed dwell" and advance the index. The next
    // waypoint queue happens on the FOLLOWING tick, when `close`
    // becomes false against the new target (index 1).
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Patrol;
        npc.patrol_next_index = 0;
        npc.position = path[0]; // physically at the current target
        npc.patrol_dwell_until =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
        npc.nav_path.clear();
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.patrol_next_index, 1,
        "Index must advance from 0 → 1 after dwell elapses at waypoint 0",
    );
    assert!(
        npc.patrol_dwell_until.is_none(),
        "Dwell must clear on advance — next tick will queue movement to the new target, \
         and the tick AFTER that (when NPC arrives) will stamp a fresh dwell",
    );
    assert!(
        npc.nav_path.is_empty(),
        "No waypoint queued on the same tick — that happens on the next tick when \
         `close` is false against the new target index",
    );
}

/// Arrival-based dwell: when the NPC reaches the target waypoint
/// (close + nav_path empty + no dwell stamp), the handler stamps the
/// dwell deadline. Pre-fix bug shape: stamping at queue time meant the
/// effective dwell was `max(0, delay - travel_time)`; for any hop
/// longer than `delay_secs` to walk, the dwell was 0.
#[tokio::test]
async fn patrol_arrival_stamps_dwell_deadline() {
    let mut mgr = make_castle_mgr();
    let path = spawn_patrol_npc(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Patrol;
        npc.patrol_next_index = 0;
        npc.position = path[0]; // just arrived
        npc.patrol_dwell_until = None; // no dwell yet
        npc.nav_path.clear();
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.patrol_dwell_until.is_some(),
        "Arrival at waypoint with no existing dwell must stamp a fresh deadline",
    );
    assert!(
        npc.patrol_dwell_until.unwrap() > std::time::Instant::now(),
        "Dwell deadline must be in the future",
    );
    assert_eq!(
        npc.patrol_next_index, 0,
        "Index must not advance on arrival"
    );
}

/// Threat preemption: a Patrol NPC that takes damage transitions to
/// Fighting and clears its in-flight `nav_path` (so the fight handler
/// doesn't continue walking the patrol route). The `patrol_next_index`
/// persists across the preemption so the post-fight return to Patrol
/// can resume the route from the correct waypoint.
#[tokio::test]
async fn patrol_preempted_by_threat_clears_nav_but_keeps_patrol_index() {
    let mut mgr = make_castle_mgr();
    spawn_patrol_npc(&mut mgr, 200, [0.0; 3]);
    // Player attacker at the same world.
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
    }

    // Pre-arrange: NPC is mid-route, in Patrol, with nav_path holding
    // the current waypoint.
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Patrol;
        npc.patrol_next_index = 2;
        npc.nav_path.push_back(Vector3::new(0.0, 0.0, 10.0));
    }

    let _ = crate::cell::combat::generate_threat(&mut mgr, 1, 200, 50.0);

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.ai_state,
        AiState::Fighting,
        "Patrol + damage → Fighting (preemption)",
    );
    assert!(
        npc.nav_path.is_empty(),
        "Preemption must clear in-flight nav_path so the fight handler can re-pathfind",
    );
    assert_eq!(
        npc.patrol_next_index, 2,
        "patrol_next_index must persist across preemption — post-fight Patrol resumes the route",
    );
}

/// Defensive: if a Patrol NPC's `patrol_path` is wiped mid-tick (e.g.,
/// by a content action), the handler must drop the NPC back to Idle
/// rather than panicking on an empty-path lookup.
#[tokio::test]
async fn patrol_with_empty_path_drops_to_idle() {
    let mut mgr = make_castle_mgr();
    spawn_patrol_npc(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Patrol;
        npc.patrol_path.clear(); // simulate content-action wipe
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle, "empty path → Idle");
    assert_eq!(
        npc.last_movement_type, None,
        "movement-type cache must clear on Patrol → Idle drop",
    );
}

/// Future dwell deadline → no work this tick. Pin the "still
/// dwelling, do nothing" branch so a refactor that drops the
/// deadline check would tip into a tight push-waypoint loop.
#[tokio::test]
async fn patrol_with_future_dwell_deadline_is_a_no_op() {
    let mut mgr = make_castle_mgr();
    let path = spawn_patrol_npc(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Patrol;
        npc.patrol_next_index = 1;
        // NPC physically AT waypoint 1 (close to target). With a
        // future dwell deadline, the handler must observe "still
        // dwelling" and not touch anything.
        npc.position = path[1];
        npc.patrol_dwell_until =
            Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
        npc.nav_path.clear();
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.patrol_next_index, 1,
        "Index must not advance while dwelling",
    );
    assert!(
        npc.nav_path.is_empty(),
        "No waypoint must be queued while dwelling",
    );
    assert!(
        npc.patrol_dwell_until.unwrap() > std::time::Instant::now(),
        "Dwell deadline must persist unchanged",
    );
}
