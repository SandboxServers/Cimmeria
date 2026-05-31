//! `npc_ai_wander` state-machine + waypoint sampling tests.

use crate::cell::space_manager::SpaceManager;
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

fn spawn_wander_npc(mgr: &mut SpaceManager, npc_id: u32, radius: f32) {
    mgr.spawn_npc(npc_id, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.class_id = 0x04;
        npc.is_player = false;
        npc.wander_radius = radius;
        npc.wander_min_dwell_secs = 3.0;
        npc.wander_max_dwell_secs = 8.0;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
}

/// Idle NPC with `wander_radius > 0` and no patrol path transitions
/// into Wander on the next AI tick AND queues a fresh waypoint /
/// dwell deadline in the same tick.
#[tokio::test]
async fn idle_npc_with_wander_radius_transitions_to_wander_same_tick() {
    let mut mgr = make_castle_mgr();
    spawn_wander_npc(&mut mgr, 200, 10.0);
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.ai_state,
        AiState::Wander,
        "Idle + wander_radius > 0 → Wander on same tick",
    );
    assert!(
        npc.wander_next_at.is_some(),
        "Wander entry must stamp wander_next_at",
    );
    assert_eq!(
        npc.last_movement_type,
        Some(MobMovementType::Wander),
        "Wander entry must broadcast MobMovementType::Wander",
    );
}

/// Wander NPC with a future `wander_next_at` is paused — no fresh
/// waypoint queued.
#[tokio::test]
async fn wander_with_future_deadline_is_a_no_op() {
    let mut mgr = make_castle_mgr();
    spawn_wander_npc(&mut mgr, 200, 10.0);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Wander;
        npc.wander_next_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
        npc.nav_path.clear();
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "Paused wander must not queue a fresh waypoint",
    );
    assert!(
        npc.wander_next_at.unwrap() > std::time::Instant::now(),
        "Pause deadline must persist",
    );
}

/// `wander_radius = 0` zeros mid-tick → handler drops back to Idle
/// rather than panicking on the angle sample.
#[tokio::test]
async fn wander_with_zero_radius_drops_to_idle() {
    let mut mgr = make_castle_mgr();
    spawn_wander_npc(&mut mgr, 200, 10.0);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Wander;
        npc.wander_radius = 0.0; // content-action wipe simulated
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle, "zero radius → Idle");
    assert_eq!(
        npc.last_movement_type, None,
        "movement-type cache must clear on Wander → Idle drop",
    );
}

/// Wander NPC with non-empty `nav_path` is mid-walk — handler must
/// not re-queue a fresh waypoint and must not advance the deadline.
#[tokio::test]
async fn wander_with_active_nav_path_is_a_no_op() {
    let mut mgr = make_castle_mgr();
    spawn_wander_npc(&mut mgr, 200, 10.0);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Wander;
        npc.nav_path
            .push_back(cimmeria_common::Vector3::new(5.0, 0.0, 5.0));
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.nav_path.len(),
        1,
        "Active nav_path must not be re-queued",
    );
}

/// Priority: an Idle NPC with BOTH a patrol path AND a wander radius
/// transitions to Patrol (patrol beats wander). Pin the dispatch
/// priority so a refactor that re-orders the if/else can't silently
/// promote wander over patrol.
#[tokio::test]
async fn idle_with_patrol_and_wander_picks_patrol() {
    let mut mgr = make_castle_mgr();
    spawn_wander_npc(&mut mgr, 200, 10.0);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.patrol_path = vec![cimmeria_common::Vector3::new(5.0, 0.0, 5.0)];
        npc.patrol_point_delay_secs = 2.0;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(
        npc.ai_state,
        AiState::Patrol,
        "patrol_path takes priority over wander_radius",
    );
}
