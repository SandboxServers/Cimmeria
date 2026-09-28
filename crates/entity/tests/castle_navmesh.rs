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

/// Every seeded Castle (world 8) spawn stands on the mesh: `is_point_valid`
/// (what the `npc_off_mesh` detector asks) and `find_path`'s tight start box
/// both accept it. NA24 (UAT-1 D): `Castle_BravoOfficer3` at (970, 26, 478)
/// sat 1.57 u outside the walkable edge and logged an off-mesh WARN every
/// 30 s for as long as the zone was up. Revert proof: put the old row back
/// and this names spawn 244.
#[test]
fn every_castle_spawn_is_on_the_mesh() {
    let Some(mesh) = castle_nav() else { return };
    let sql = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../db/resources/Worlds/Seed/spawnlist.sql"),
    )
    .expect("spawnlist seed");
    let mut checked = 0;
    let mut off = Vec::new();
    for line in sql
        .lines()
        .filter(|l| l.starts_with("INSERT INTO spawnlist"))
    {
        // Every spawnlist INSERT leads with
        // `spawn_id, x, y, z, heading, world_id, template_id, tag`.
        let vals = line.split("VALUES (").nth(1).expect("VALUES list");
        let f: Vec<&str> = vals.split(", ").collect();
        if f[5].trim() != "8" {
            continue;
        }
        let num = |i: usize| f[i].trim().parse::<f32>().expect("numeric column");
        let p = v(num(1), num(2), num(3));
        checked += 1;
        if !mesh.is_point_valid(&p) || mesh.start_poly_snap(&p).is_none() {
            off.push(format!("spawn {} {} at {p:?}", f[0], f[7]));
        }
    }
    assert!(checked > 30, "parsed only {checked} world-8 spawns");
    assert!(off.is_empty(), "off castle.nav: {off:#?}");
}

/// Every World 8 patrol loop (`point_sets` type `Patrol`) is walkable:
/// each waypoint stands on the mesh and each leg, including the one that
/// closes the loop, routes all the way. A leg that ends at a component
/// boundary is still a "path" to Detour (`Partial`), and the patrol then
/// slides the NPC toward the waypoint across whatever lies between, which
/// in Castle's layered interior means through a floor. Revert proof: move
/// waypoint 2424 (the east end of `Castle.Patrol.ThroneApproach_A`) into the
/// Symbiote Chamber at (386, 55.38, 940), valid mesh in an isolated
/// component, and both of set 2091's legs report `partial`.
#[test]
fn every_castle_patrol_leg_is_routable() {
    let Some(mesh) = castle_nav() else { return };
    let seed = |file: &str| {
        std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../db/resources/Events/Seed")
                .join(file),
        )
        .expect("point set seed")
    };
    let values = |line: &str| -> Vec<String> {
        let vals = line.split("VALUES (").nth(1).expect("VALUES list");
        vals.trim_end_matches(';')
            .trim_end_matches(')')
            .split(", ")
            .map(|s| s.trim().to_string())
            .collect()
    };
    let sets: Vec<i32> = seed("point_sets.sql")
        .lines()
        .filter(|l| l.starts_with("INSERT INTO point_sets"))
        .map(values)
        .filter(|f| f[2] == "'Patrol'" && f[3] == "8")
        .map(|f| f[0].parse().expect("set_id"))
        .collect();
    assert!(
        sets.len() >= 8,
        "found only {} Castle patrol sets",
        sets.len()
    );

    let points = seed("point_set_points.sql");
    let mut failures = Vec::new();
    for set in &sets {
        // (point_id, position); the loader orders waypoints by point_id.
        let mut wps: Vec<(i32, Vector3)> = points
            .lines()
            .filter(|l| l.starts_with("INSERT INTO point_set_points"))
            .map(values)
            .filter(|f| f[0] == set.to_string())
            .map(|f| {
                let n = |i: usize| f[i].parse::<f32>().expect("numeric column");
                (f[1].parse().expect("point_id"), v(n(2), n(3), n(4)))
            })
            .collect();
        wps.sort_by_key(|(id, _)| *id);
        assert!(
            wps.len() >= 2,
            "patrol set {set} has {} waypoints",
            wps.len()
        );
        for (id, p) in &wps {
            if !mesh.is_point_valid(p) || mesh.start_poly_snap(p).is_none() {
                failures.push(format!("set {set} waypoint {id} at {p:?} is off the mesh"));
            }
        }
        for i in 0..wps.len() {
            let (a_id, a) = wps[i];
            let (b_id, b) = wps[(i + 1) % wps.len()];
            let outcome = mesh.find_path(&a, &b);
            let status = outcome.status.label();
            let arrived = outcome
                .into_waypoints()
                .and_then(|w| w.last().map(|e| e.distance_to(&b)))
                .is_some_and(|d| d < 2.0);
            if status != "ok" || !arrived {
                failures.push(format!(
                    "set {set} leg {a_id} -> {b_id} ({a:?} -> {b:?}): {status}, arrived={arrived}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "unroutable patrol legs: {failures:#?}");
}
