//! Mission 1521 (CS-03c2): the symbiote tank is routable from Ogilvie in the
//! Infirmary. CS-03S placed it by `nav_inspect` component equality
//! (both on component 250); this runs the actual `find_path`. Coordinates are
//! BigWorld (x, y-up, z), read from the spawn seed so a moved row is caught.

use std::path::PathBuf;

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

fn spawn_xyz(sql: &str, spawn_id: i32) -> (f32, f32, f32) {
    let needle = format!("VALUES ({spawn_id}, ");
    let line = sql
        .lines()
        .find(|l| l.contains(&needle))
        .unwrap_or_else(|| panic!("spawn {spawn_id} must be seeded"));
    let rest = &line[line.find(&needle).unwrap() + needle.len()..];
    let n: Vec<f32> = rest
        .split(',')
        .take(3)
        .map(|s| s.trim().parse().expect("coordinate"))
        .collect();
    (n[0], n[1], n[2])
}

#[test]
fn the_symbiote_tank_is_routable_from_ogilvie() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let nav = root.join("data/spaces/castle.nav");
    if !nav.exists() {
        eprintln!("SKIPPED - {} not present", nav.display());
        return;
    }
    let mesh = NavMesh::load(&nav).expect("castle.nav must load");
    let sql = std::fs::read_to_string(root.join("db/resources/Worlds/Seed/spawnlist.sql"))
        .expect("spawnlist seed");
    let (ox, oy, oz) = spawn_xyz(&sql, 197);
    let (tx, ty, tz) = spawn_xyz(&sql, 13802);
    let from = Vector3::new(ox, oy, oz);
    let to = Vector3::new(tx, ty, tz);

    let path = mesh
        .find_path(&from, &to)
        .into_waypoints()
        .expect("Ogilvie -> symbiote tank must be one connected region");
    let end = path.last().expect("a path has an end");
    assert!(
        end.distance_to(&to) < 2.0,
        "path must arrive (a partial path stops at the component boundary); \
         ended {:.1} m short at {end:?}",
        end.distance_to(&to)
    );
}
