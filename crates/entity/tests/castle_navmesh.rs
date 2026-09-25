//! `data/spaces/castle.nav` against the Detour loader the server uses.
//!
//! Castle (world 8) had no navmesh until 2026-09-19, so NPCs there pathed
//! in raw straight lines through walls and floors. This mesh is rebuilt
//! from the cooked client maps; these tests pin what it is *for*: the
//! places players demonstrably stood are on it, and the routes content
//! depends on are routable.
//!
//! Coordinates are BigWorld (x, y-up, z). HIGH = live playtest telemetry,
//! MEDIUM = reconstructed from map data (see
//! `docs/analysis/castle-rebuild/worknotes/ca05.md`).

use std::path::PathBuf;

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

fn castle_nav() -> Option<NavMesh> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join("spaces")
        .join("castle.nav");
    if !path.exists() {
        eprintln!("SKIPPED — {} not present", path.display());
        return None;
    }
    Some(NavMesh::load(&path).expect("castle.nav must load through the Detour loader"))
}

fn v(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3::new(x, y, z)
}

// HIGH confidence: a player or an escorted NPC stood here on 2026-09-18.
const ZURITSKA_CELL: (f32, f32, f32) = (268.0, 66.79, 1042.59);
const ROMNEY_CORRIDOR: (f32, f32, f32) = (244.0, 66.79, 1036.0);
const COMMS_ROOM: (f32, f32, f32) = (271.7, 55.2, 858.0);
// MEDIUM confidence: reconstructed placements.
const NID_GUARD_116: (f32, f32, f32) = (294.16, 55.39, 894.44);
const GATE_ROOM_DHD: (f32, f32, f32) = (806.27, 55.10, 517.24);
const BUNKER_MUELBACH: (f32, f32, f32) = (1008.0, 48.0, 414.0);

#[test]
fn the_mesh_is_a_whole_map_build_with_a_humanoid_agent() {
    let Some(mesh) = castle_nav() else { return };
    // A cropped or truncated build would still load. The whole map is
    // ~20k polygons; Recast's unchecked 16-bit edge limit caps it near 36k.
    assert!(
        mesh.poly_count() > 15_000,
        "castle.nav has {} polygons — expected the whole-map build",
        mesh.poly_count()
    );
    // The radius drives `is_point_valid`'s gates; the 2013 meshes used a
    // 0.6-high agent, which let NPCs path under waist-high obstacles.
    assert_eq!(mesh.agent_radius, 0.6);
    assert_eq!(mesh.agent_height, 1.8);
}

#[test]
fn places_players_stood_are_valid_positions() {
    let Some(mesh) = castle_nav() else { return };
    for (name, p) in [
        ("zuritska_cell", ZURITSKA_CELL),
        ("romney_corridor", ROMNEY_CORRIDOR),
        ("comms_room", COMMS_ROOM),
        ("nid_guard_116", NID_GUARD_116),
        ("gate_room_dhd", GATE_ROOM_DHD),
        ("bunker_muelbach", BUNKER_MUELBACH),
    ] {
        assert!(
            mesh.is_point_valid(&v(p.0, p.1, p.2)),
            "{name} {p:?} must be on the navmesh: interior floors are BSP and \
             outdoor ground is Terrain, and losing either extraction drops it"
        );
    }
}

/// Mission 704 escorts Zuritska from his cell to the Level-5
/// Communications room: ~190 m and an 11.6 m descent through the
/// Interrogation Block. Before this mesh the follower walked it as a
/// straight line through walls and floors.
#[test]
fn the_704_escort_route_is_routable() {
    let Some(mesh) = castle_nav() else { return };
    let from = v(ZURITSKA_CELL.0, ZURITSKA_CELL.1, ZURITSKA_CELL.2);
    let to = v(COMMS_ROOM.0, COMMS_ROOM.1, COMMS_ROOM.2);
    let path = mesh
        .find_path(&from, &to)
        .into_waypoints()
        .expect("cell -> comms room must be one connected region");
    assert!(
        path.len() > 2,
        "a {:.0} m route through corridors cannot be a single straight leg; got {} waypoints",
        from.distance_to(&to),
        path.len()
    );
    let end = path.last().unwrap();
    assert!(
        end.distance_to(&to) < 2.0,
        "path must actually arrive (a partial path ends at the component boundary); \
         ended {:.1} m short at {end:?}",
        end.distance_to(&to)
    );
}

/// The exterior is its own region: the gate room and the bunker above
/// Checkpoint Bravo (mission 708) are joined across open ground.
#[test]
fn the_gate_room_reaches_the_bravo_bunker() {
    let Some(mesh) = castle_nav() else { return };
    let from = v(GATE_ROOM_DHD.0, GATE_ROOM_DHD.1, GATE_ROOM_DHD.2);
    let to = v(BUNKER_MUELBACH.0, BUNKER_MUELBACH.1, BUNKER_MUELBACH.2);
    let path = mesh
        .find_path(&from, &to)
        .into_waypoints()
        .expect("gate room -> bunker must be one connected region");
    let end = path.last().unwrap();
    assert!(
        end.distance_to(&to) < 2.0,
        "ended {:.1} m short at {end:?}",
        end.distance_to(&to)
    );
}
