//! Tests for the `interaction_hostiles` row and [`nearby_hostiles`]: the three
//! nearest living hostiles, the exclusions, and no row in an empty area.

use super::{capture_interaction_pose, emit_nearby_hostiles, nearby_hostiles, InteractionPose};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, LogCapture};
use cimmeria_wire::state_field::BSF_DEAD;
use tracing::Level;

/// Faction of a hostile NPC (the `HOSTILE_FACTION` sentinel).
const HOSTILE: u8 = 10;

/// Spawns an NPC at `(x, 0, z)` with `faction` and returns its id.
fn spawn(mgr: &mut SpaceManager, x: f32, z: f32, faction: u8) -> u32 {
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc(id, "HostileProbe", [x, 0.0, z], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(id).unwrap().faction = faction;
    id
}

/// Builds a player `id` at the origin facing +x, plus the player's pose at the
/// target `target` (an NPC placed by the caller).
fn pose_for(mgr: &mut SpaceManager, id: u32, target: u32) -> InteractionPose {
    mgr.create_entity(id, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(id).unwrap().is_player = true;
    capture_interaction_pose(mgr, id, target).unwrap()
}

/// Hostiles at 3, 6, 9 and 12 m, a non-hostile at 4 m, a dead hostile at 5 m,
/// and the interaction target (a hostile at 2 m to the south).
fn crowded_scene(id: u32) -> (SpaceManager, InteractionPose, [u32; 4]) {
    let mut mgr = make_space_manager();
    let target = spawn(&mut mgr, 0.0, -2.0, HOSTILE);
    let h3 = spawn(&mut mgr, 3.0, 0.0, HOSTILE);
    let h6 = spawn(&mut mgr, 6.0, 0.0, HOSTILE);
    let h9 = spawn(&mut mgr, 9.0, 0.0, HOSTILE);
    let h12 = spawn(&mut mgr, 12.0, 0.0, HOSTILE);
    spawn(&mut mgr, 4.0, 0.0, 1);
    let dead = spawn(&mut mgr, 5.0, 0.0, HOSTILE);
    mgr.get_entity_mut(dead).unwrap().state_field |= BSF_DEAD;
    let pose = pose_for(&mut mgr, id, target);
    (mgr, pose, [h3, h6, h9, h12])
}

/// Pins that the three nearest living hostiles come back nearest first, and
/// that the target, the dead hostile, the non-hostile and the fourth hostile are left out.
#[test]
fn nearby_hostiles_returns_three_nearest_living_in_order() {
    let (mgr, pose, [h3, h6, h9, _h12]) = crowded_scene(9221);
    let found = nearby_hostiles(&mgr, &pose);
    let ids: Vec<u32> = found.iter().map(|h| h.entity_id).collect();
    assert_eq!(ids, vec![h3, h6, h9]);
    assert!((found[0].dist_m - 3.0).abs() < 0.01, "{}", found[0].dist_m);
    assert!((found[2].dist_m - 9.0).abs() < 0.01, "{}", found[2].dist_m);
}

/// Pins that the row is written once, with `count = 3` and the nearest hostile
/// first, and that it carries the pose row's `pose_seq`.
#[test]
fn emit_writes_one_interaction_hostiles_row() {
    let id = 9222;
    let (mgr, pose, [h3, _h6, h9, _h12]) = crowded_scene(id);
    let guard = LogCapture::install();
    emit_nearby_hostiles(&mgr, &pose, 77);
    let rows: Vec<_> = guard
        .all()
        .into_iter()
        .filter(|c| {
            c.target == "interaction.pose"
                && c.fields.get("event").map(|v| v.as_str()) == Some("interaction_hostiles")
        })
        .collect();
    assert_eq!(rows.len(), 1, "one interaction_hostiles row");
    let row = &rows[0];
    assert_eq!(row.level, Level::INFO);
    assert!(row.has_field("count", "3"));
    assert!(row.has_field("pose_seq", "77"));
    assert!(row.has_field("hostile_1_id", &h3.to_string()));
    assert!(row.has_field("hostile_3_id", &h9.to_string()));
}

/// Pins that an empty area writes no `interaction_hostiles` row at all.
#[test]
fn empty_area_writes_no_hostiles_row() {
    let id = 9223;
    let mut mgr = make_space_manager();
    let target = spawn(&mut mgr, 0.0, -2.0, 1);
    spawn(&mut mgr, 4.0, 0.0, 1);
    let pose = pose_for(&mut mgr, id, target);
    let guard = LogCapture::install();
    emit_nearby_hostiles(&mgr, &pose, 5);
    assert!(nearby_hostiles(&mgr, &pose).is_empty());
    let rows = guard
        .all()
        .into_iter()
        .filter(|c| c.fields.get("event").map(|v| v.as_str()) == Some("interaction_hostiles"))
        .count();
    assert_eq!(rows, 0);
}
