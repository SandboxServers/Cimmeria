//! Seed-vs-navmesh guard for the interior / story-room respawners (rows
//! 24-29 in `db/resources/Worlds/Seed/respawners.sql`), placed from map data
//! on 2026-09-27 because these worlds had no stargate, ring pad, spawn or
//! respawner at all.
//!
//! Same pairing as `harset_placement_tests`: the coordinate is read from the
//! seeded database and judged by the real `data/spaces/<world>.nav`, so the
//! test fails if either the row is dropped or it stops standing on the mesh.
//! The meshes are loaded by path (relative to `crates/cell-world`, the test
//! CWD) for the reason that module gives.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

use crate::cell::spawner::{load_respawners, load_stargates};
use crate::test_support::require_db_or_skip;

/// `(respawner_id, world, navmesh file)`.
const ROOM_RESPAWNERS: [(i32, &str, &str); 6] = [
    (24, "Agnos_Library", "agnos_library.nav"),
    (25, "Dakara_E1_StoryRm", "dakara_e1_storyrm.nav"),
    (26, "Omega_Site_CmdCenter", "omega_site_cmdcenter.nav"),
    (27, "Tollana_Curia", "tollana_curia.nav"),
    (28, "Sewer_Falls", "sewer_falls.nav"),
    (29, "SandBox", "sandbox.nav"),
];

fn load_mesh(file: &str) -> Option<NavMesh> {
    let path = std::path::PathBuf::from("../../data/spaces").join(file);
    if !path.exists() {
        return None;
    }
    Some(NavMesh::load(&path).unwrap_or_else(|e| panic!("{file} failed to load: {e}")))
}

/// Each room world has its respawner, it is not a `(0,0,0)` placeholder, and
/// it stands on that world's mesh. Without the row, `.gotolocation <world>`
/// refuses ("no known entry point") and a death there respawns in place.
#[tokio::test]
async fn room_respawners_exist_and_stand_on_their_worlds_mesh() {
    let pool = require_db_or_skip!();
    let rows = load_respawners(&pool).await.expect("load_respawners");

    for (id, world, nav) in ROOM_RESPAWNERS {
        let r = rows
            .iter()
            .find(|r| r.respawner_id == id)
            .unwrap_or_else(|| panic!("respawner {id} ({world}) is missing from the seed"));
        assert_eq!(r.world_name, world, "respawner {id} moved worlds");
        assert_ne!(r.pos, [0.0; 3], "respawner {id} ({world}) is a placeholder");

        if let Some(mesh) = load_mesh(nav) {
            let p = Vector3::new(r.pos[0], r.pos[1], r.pos[2]);
            assert!(
                mesh.is_point_valid(&p),
                "respawner {id} ({world}) {:?} is off {nav}",
                r.pos
            );
        }
    }
}

/// Gate 15 (Agnos) shipped at (0,0,0) with no prefab in the map to recover
/// it from; its arrival is pinned to the map's PlayerStart. The pin must be
/// seeded and stand on `agnos.nav`, or gate travel into Agnos lands at the
/// origin again.
#[tokio::test]
async fn agnos_gate_arrival_is_pinned_on_the_mesh() {
    let pool = require_db_or_skip!();
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates.get(&15).expect("stargate 15 (Agnos) is seeded");
    assert_eq!(gate.world_name, "Agnos");
    let (pos, _yaw) = gate
        .arrival
        .expect("gate 15 must carry its PlayerStart arrival pin");
    assert_ne!(pos, [0.0; 3]);
    if let Some(mesh) = load_mesh("agnos.nav") {
        assert!(
            mesh.is_point_valid(&Vector3::new(pos[0], pos[1], pos[2])),
            "gate 15's arrival {pos:?} is off agnos.nav"
        );
    }
}
