//! `npc_ai_follow` distance-band maintenance + target-tracking.

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

fn spawn_follower_at(mgr: &mut SpaceManager, npc_id: u32, pos: [f32; 3]) {
    mgr.spawn_npc(npc_id, "Castle", pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.class_id = 0x04;
        npc.is_player = false;
        npc.follow_min_distance = 2.0;
        npc.follow_max_distance = 5.0;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
}

fn spawn_player_at(mgr: &mut SpaceManager, player_id: u32, pos: [f32; 3]) {
    mgr.create_entity(player_id, "Castle", pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.is_player = true;
    }
}

/// Target outside the max-distance band → pathfind toward target.
#[tokio::test]
async fn follow_out_of_band_pathfinds_toward_target() {
    let mut mgr = make_castle_mgr();
    spawn_follower_at(&mut mgr, 200, [0.0; 3]);
    spawn_player_at(&mut mgr, 1, [20.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(1);
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.nav_path.is_empty(),
        "Out-of-band follow must queue waypoints toward target",
    );
    assert_eq!(
        npc.last_movement_type,
        Some(MobMovementType::Follow),
        "Follow entry must broadcast MobMovementType::Follow",
    );
}

/// Target inside the band → hold position (no nav_path).
#[tokio::test]
async fn follow_in_band_holds_position() {
    let mut mgr = make_castle_mgr();
    spawn_follower_at(&mut mgr, 200, [0.0; 3]);
    spawn_player_at(&mut mgr, 1, [3.0, 0.0, 0.0]); // distance 3, inside [2, 5]
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(1);
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "In-band follow must not queue movement",
    );
}

/// Target closer than min_distance → still hold (NPCs don't back away).
#[tokio::test]
async fn follow_below_min_distance_holds_position() {
    let mut mgr = make_castle_mgr();
    spawn_follower_at(&mut mgr, 200, [0.0; 3]);
    spawn_player_at(&mut mgr, 1, [1.0, 0.0, 0.0]); // distance 1, below min=2
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(1);
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "Below-min-distance follow must hold (NPCs don't back away)",
    );
}

/// Target despawned → clear follow_target_id, drop to Idle.
#[tokio::test]
async fn follow_with_gone_target_drops_to_idle() {
    let mut mgr = make_castle_mgr();
    spawn_follower_at(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(999); // never existed
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
    assert_eq!(npc.follow_target_id, None);
    assert_eq!(npc.last_movement_type, None);
}

/// `follow_target_id = None` mid-Follow → drop to Idle.
#[tokio::test]
async fn follow_with_no_target_drops_to_idle() {
    let mut mgr = make_castle_mgr();
    spawn_follower_at(&mut mgr, 200, [0.0; 3]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = None;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
}

/// Threat preemption preserves the follow target — content authors
/// who want a continued follow post-fight can inspect the field.
#[tokio::test]
async fn follow_preempted_by_threat_clears_nav_keeps_target() {
    let mut mgr = make_castle_mgr();
    spawn_follower_at(&mut mgr, 200, [0.0; 3]);
    spawn_player_at(&mut mgr, 1, [20.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Follow;
        npc.follow_target_id = Some(1);
        npc.nav_path.push_back(Vector3::new(10.0, 0.0, 0.0));
    }

    let _ = crate::cell::combat::generate_threat(&mut mgr, 1, 200, 50.0);

    let npc = mgr.get_entity(200).unwrap();
    assert_eq!(npc.ai_state, AiState::Fighting);
    assert!(npc.nav_path.is_empty());
    assert_eq!(
        npc.follow_target_id,
        Some(1),
        "Follow target persists across preemption",
    );
}
