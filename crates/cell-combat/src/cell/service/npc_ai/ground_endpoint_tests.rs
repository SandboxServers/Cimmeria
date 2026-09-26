//! NA11 guards for the raw-endpoint movers (audit M5): the min-range backup,
//! patrol and investigate all end on the navmesh floor, on the real
//! `castle_cellblock.nav`.

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use tokio::sync::mpsc;

use super::ability_select::{backup_waypoint_on_mesh, compute_backup_waypoint};
use crate::cell::space_manager::SpaceManager;

const FIXTURE: &str = "../../data/spaces/castle_cellblock.nav";
const NPC: u32 = 101;

/// A point on the flat floor of the guard room, on the route the guards
/// chase along, with open floor back toward their spawn.
const GUARD_ROOM_FLOOR: Vector3 = Vector3 {
    x: -295.94,
    y: 68.6,
    z: -165.54,
};

/// A navmesh-backed Castle_CellBlock space with NPC [`NPC`] standing on the
/// floor at `at` (Y replaced by the floor), or `None` without the fixture.
fn cellblock_npc_on_floor(at: Vector3) -> Option<(SpaceManager, Vector3)> {
    let path = std::path::Path::new(FIXTURE);
    if !path.exists() {
        return None;
    }
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr
        .spawn_npc(NPC, "Castle_CellBlock", [at.x, at.y, at.z], [0.0; 3])
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh =
        Some(NavMesh::load(path).expect("load castle_cellblock.nav"));
    let floor = mgr
        .get_navmesh_height(NPC, at.x, at.y, at.z)
        .expect("fixture point must be over a floor");
    let pos = Vector3::new(at.x, floor, at.z);
    mgr.update_position_preserving_facing(NPC, [pos.x, pos.y, pos.z], [0.0; 3]);
    Some((mgr, pos))
}

/// **M5 regression guard.** A player 5 u above the NPC and inside its
/// ability's `min_range`: the backup waypoint lands on the NPC's floor and
/// on the mesh. The old 3D extrapolation kept the vertical component of the
/// player-to-NPC vector and put the point ~3 u under the floor.
#[test]
fn a_backup_from_a_player_above_ends_on_the_floor() {
    let Some((mgr, npc_pos)) = cellblock_npc_on_floor(GUARD_ROOM_FLOOR) else {
        return;
    };
    // 2.6 u away horizontally, 5 u up: 5.6 u, inside a min_range of 8.
    let player = Vector3::new(npc_pos.x - 1.56, npc_pos.y + 5.0, npc_pos.z - 2.06);
    let min_range = 8.0;

    let backup = backup_waypoint_on_mesh(&mgr, NPC, npc_pos, player, min_range)
        .expect("a non-degenerate backup");

    let floor = mgr
        .get_navmesh_height(NPC, backup.x, npc_pos.y, backup.z)
        .unwrap_or_else(|| panic!("no floor under the backup point {backup:?}"));
    assert!(
        (backup.y - floor).abs() <= 0.3,
        "the backup point {backup:?} must stand on the floor at {floor:.2}"
    );
    assert!(
        mgr.is_position_valid(NPC, &backup),
        "the backup point {backup:?} must be on the navmesh"
    );
    // It still backs away far enough to fire: open floor behind the NPC.
    let (dx, dz) = (backup.x - player.x, backup.z - player.z);
    assert!(
        (dx * dx + dz * dz).sqrt() > min_range,
        "the backup {backup:?} must clear min_range from the player {player:?}"
    );
}

/// Without a navmesh the raw point is used, so the raw point itself must
/// not inherit the player's height: it is at the NPC's own Y.
#[test]
fn a_raw_backup_point_keeps_the_npc_height() {
    let npc = Vector3::new(3.0, 0.0, 0.0);
    let player = Vector3::new(0.0, 5.0, 0.0);
    let backup = compute_backup_waypoint(npc, player, 8.0).unwrap();
    assert_eq!(backup.y, 0.0, "{backup:?}");
    assert!((backup.x - 9.0).abs() < 1e-4, "{backup:?}");
    // Straight overhead has no horizontal direction to back away along.
    assert!(compute_backup_waypoint(npc, Vector3::new(3.0, 5.0, 0.0), 8.0).is_none());
}

/// **M5 regression guard.** A patrol point authored 1.5 u above the floor
/// the NPC is standing on reads as arrived. Unsnapped, the 1.5 u gap kept
/// the arrival check false forever and the NPC re-routed to its own spot
/// on every tick instead of dwelling.
#[tokio::test]
async fn a_patrol_point_authored_above_the_floor_is_reached() {
    let Some((mut mgr, npc_pos)) = cellblock_npc_on_floor(GUARD_ROOM_FLOOR) else {
        return;
    };
    let authored = Vector3::new(npc_pos.x, npc_pos.y + 1.5, npc_pos.z);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    super::force_ai_state(npc, AiState::Patrol);
    npc.patrol_path = vec![authored, Vector3::new(-289.9, 68.6, -157.6)];
    npc.patrol_next_index = 0;

    let (tx, _rx) = mpsc::channel(8);
    super::patrol::npc_ai_patrol(NPC, &tx, &mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(
        npc.patrol_dwell_until.is_some() && npc.nav_path.is_empty(),
        "an NPC on the floor under its patrol point has arrived and dwells, \
         got dwell {:?} and route {:?}",
        npc.patrol_dwell_until,
        npc.nav_path
    );
}

/// **M5 regression guard.** The same for a content-authored investigate
/// POI.
#[tokio::test]
async fn an_investigate_poi_authored_above_the_floor_is_reached() {
    let Some((mut mgr, npc_pos)) = cellblock_npc_on_floor(GUARD_ROOM_FLOOR) else {
        return;
    };
    let npc = mgr.get_entity_mut(NPC).unwrap();
    super::force_ai_state(npc, AiState::Investigating);
    npc.poi = Some(Vector3::new(npc_pos.x, npc_pos.y + 1.5, npc_pos.z));
    npc.investigate_until = None;

    let (tx, _rx) = mpsc::channel(8);
    super::investigate::npc_ai_investigate(NPC, &tx, &mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(
        npc.investigate_until.is_some() && npc.nav_path.is_empty(),
        "an NPC on the floor under its POI has arrived and dwells, got \
         dwell {:?} and route {:?}",
        npc.investigate_until,
        npc.nav_path
    );
}
