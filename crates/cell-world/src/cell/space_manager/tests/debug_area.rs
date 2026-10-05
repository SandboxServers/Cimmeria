//! World 1300 `DebugArea` (docs/analysis/debug-area/README.md): shared and
//! always loaded (D-DA3), and running on the Ihpet_Crater_Light map's
//! navmesh and occluder (D-DA5).

use super::super::*;

fn repo_file(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn entities_file(name: &str) -> String {
    let path = repo_file(&format!("entities/{name}"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn debug_area_is_a_shared_startup_space() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&entities_file("spaces.xml")).unwrap();
    mgr.create_startup_spaces(&entities_file("cell_spaces.xml"))
        .unwrap();

    assert!(
        !mgr.is_world_instanced("DebugArea"),
        "DebugArea must be Instanced=\"false\" (D-DA3)"
    );
    assert_eq!(mgr.canonical_world_name("debugarea"), Some("DebugArea"));
    assert!(
        mgr.has_space_for_world("DebugArea"),
        "DebugArea must be listed in cell_spaces.xml so it is always loaded"
    );
    let first = mgr.find_or_create_space("DebugArea").unwrap();
    let second = mgr.find_or_create_space("DebugArea").unwrap();
    assert_eq!(first, second, "every arrival shares the one space");
    assert_eq!(mgr.default_space_for_world("DebugArea"), Some(first));
    assert_ne!(
        Some(first),
        mgr.default_space_for_world("Ihpet_Crater_Light"),
        "a world of its own, not the stock world 73's space"
    );
}

/// D-DA5 end to end: with the shipped `data/spaces/`, DebugArea's space gets
/// the Ihpet_Crater_Light mesh and shares world 73's occluder (one load, one
/// residency set). Fails if the client-map fallback is removed: there is no
/// `debugarea.nav` / `.occ`.
#[test]
fn debug_area_space_runs_on_the_ihpet_crater_light_nav_and_occ() {
    // The files are committed (not LFS), so a missing one is a failure.
    let dir = repo_file("data/spaces");
    let mut mgr = SpaceManager::new(1);
    mgr.space_data_dir = dir;
    mgr.parse_spaces_xml(&entities_file("spaces.xml")).unwrap();
    mgr.create_startup_spaces(
        r#"<Spaces><Space WorldName="Ihpet_Crater_Light" /><Space WorldName="DebugArea" /></Spaces>"#,
    )
    .unwrap();

    let space = |world: &str| {
        let id = mgr.default_space_for_world(world).unwrap();
        &mgr.spaces[&id]
    };
    let debug = space("DebugArea");
    let light = space("Ihpet_Crater_Light");
    let debug_mesh = debug.navmesh.as_ref().expect("DebugArea has a navmesh");
    let light_mesh = light.navmesh.as_ref().expect("world 73 has a navmesh");
    assert_eq!(debug_mesh.short_hash(), light_mesh.short_hash());

    let debug_occ = debug.occluder.as_ref().expect("DebugArea has an occluder");
    let light_occ = light.occluder.as_ref().expect("world 73 has an occluder");
    assert!(
        std::sync::Arc::ptr_eq(debug_occ, light_occ),
        "one occluder per file, shared by both worlds"
    );
    assert_eq!(mgr.occluders.len(), 1, "{:?}", mgr.occluders.keys());
    assert_eq!(
        mgr.occluder_files.get("debugarea").map(String::as_str),
        Some("ihpet_crater_light")
    );

    // Respawners 130 (Z1 arrival) and 131 (Z9) stand on the mesh the cell
    // loaded for the world (nav_inspect: both component 17, dy +0.04 / -0.70).
    for (id, [x, y, z]) in [(130, [251.0, 8.0, -962.0]), (131, [438.0, 10.4, -916.0])] {
        assert!(
            debug_mesh.is_point_valid(&cimmeria_common::Vector3::new(x, y, z)),
            "respawner {id} must be on the Ihpet_Crater_Light mesh"
        );
    }
}

/// `(x, y, z)` of a `spawnlist` row in the DA-02 plaza seed.
fn plaza_spawn(spawn_id: i32) -> [f32; 3] {
    let path = repo_file("db/resources/Worlds/Seed/spawnlist_debug_area_plaza.sql");
    let seed = std::fs::read_to_string(&path).unwrap();
    let key = format!("VALUES ({spawn_id}, ");
    let row = seed
        .lines()
        .find(|l| l.contains(&key))
        .unwrap_or_else(|| panic!("no spawnlist row {spawn_id}"));
    let values: Vec<f32> = row[row.find(&key).unwrap() + key.len()..]
        .split(", ")
        .take(3)
        .map(|v| v.parse().unwrap())
        .collect();
    [values[0], values[1], values[2]]
}

/// DA-F7: the friendly training dummy (spawn 13104) is a heal target with a
/// 5 m range (Health Heal 1646, `max_range` 500), so a healer must be able
/// to walk up to it. Low walls split the dummy line into bays (navmesh gaps
/// at x 240.7-244.3 and 259.4-261 on z -872). At its first spot, the east
/// end (264), the wall at 259.4-261 left the nearest point a healer reached
/// more than 5 m away (DA-06). Now it shares the open west bay with the L1
/// dummy (spawn 13100): the walk from L1 to it must stay on navmesh
/// polygons the whole way, and a healer 4 m out on that line must see it
/// past the occluder. Moving it back to x 264 fails the walk at the first
/// wall it crosses (about x 241).
#[test]
fn the_friendly_dummy_is_reachable_and_visible_from_the_dummy_line() {
    use cimmeria_common::Vector3;
    use cimmeria_occluder::Sight;

    let mut mgr = SpaceManager::new(1);
    mgr.space_data_dir = repo_file("data/spaces");
    mgr.parse_spaces_xml(&entities_file("spaces.xml")).unwrap();
    mgr.create_startup_spaces(r#"<Spaces><Space WorldName="DebugArea" /></Spaces>"#)
        .unwrap();
    let space = &mgr.spaces[&mgr.default_space_for_world("DebugArea").unwrap()];
    let mesh = space.navmesh.as_ref().expect("DebugArea has a navmesh");
    let occ = space.occluder.as_ref().expect("DebugArea has an occluder");

    let friendly = plaza_spawn(13104);
    let l1 = plaza_spawn(13100);
    // The mesh is eroded by the agent radius (0.6 m), so a point within that
    // of a polygon is one a body can stand against; small dents in the
    // eroded edge read up to about 0.5 m. A wall footprint reads 0.7-1.4 m
    // in its middle.
    const MAX_OFF_MESH: f32 = 0.6;
    let steps = ((friendly[0] - l1[0]).abs() / 0.25).ceil() as usize;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let p = Vector3::new(
            l1[0] + (friendly[0] - l1[0]) * t,
            l1[1] + (friendly[1] - l1[1]) * t,
            l1[2] + (friendly[2] - l1[2]) * t,
        );
        let (_, nearest) = mesh
            .find_nearest_poly(&p)
            .unwrap_or_else(|| panic!("no polygon near {p:?}"));
        let off = ((nearest.x - p.x).powi(2) + (nearest.z - p.z).powi(2)).sqrt();
        assert!(
            off <= MAX_OFF_MESH,
            "the walk from L1 to the friendly dummy leaves the navmesh at x {:.2} ({off:.2} m off)",
            p.x
        );
    }

    // A healer 4 m from the dummy, on the line's side, eye to chest.
    let toward_line = (l1[0] - friendly[0]).signum();
    let healer = [
        friendly[0] + 4.0 * toward_line,
        friendly[1] + 1.5,
        friendly[2],
    ];
    let target = [friendly[0], friendly[1] + 1.2, friendly[2]];
    assert_eq!(
        occ.sight(healer, target),
        Sight::Clear,
        "a healer 4 m out must have line of sight to the friendly dummy"
    );
}
