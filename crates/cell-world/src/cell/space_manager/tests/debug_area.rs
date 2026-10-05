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
