//! Worlds 1301 `CimmeriaLab` and 1302 `MapperDebug`
//! (docs/analysis/custom-debug-map/): each loads its own navmesh, built from
//! its locally installed client package set (`data/spaces/README.md`).

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

/// Both custom-map worlds come up from `cell_spaces.xml` with a mesh of
/// their own, and the documented arrival point stands on it. Fails if
/// `cimmerialab.nav` or `mapperdebug.nav` is removed: neither client map
/// (`Cimmeria_Lab1`, `Debug`) has a file to fall back to.
#[test]
fn custom_map_worlds_load_their_own_navmesh() {
    let mut mgr = SpaceManager::new(1);
    mgr.space_data_dir = repo_file("data/spaces");
    mgr.parse_spaces_xml(&entities_file("spaces.xml")).unwrap();
    mgr.create_startup_spaces(&entities_file("cell_spaces.xml"))
        .unwrap();

    // `.gotolocation CimmeriaLab 0 2 0` (client-load-test.md) and the
    // MapperDebug gate apron (mapper-debug-import.md). nav_inspect: 1.06 m and
    // 0.10 m from the nearest polygon.
    for (world, [x, y, z]) in [
        ("CimmeriaLab", [0.0, 2.0, 0.0]),
        ("MapperDebug", [40.0, 2.0, 177.0]),
    ] {
        let id = mgr
            .default_space_for_world(world)
            .unwrap_or_else(|| panic!("{world} has no startup space"));
        let mesh = mgr.spaces[&id]
            .navmesh
            .as_ref()
            .unwrap_or_else(|| panic!("{world} has no navmesh"));
        let p = cimmeria_common::Vector3::new(x, y, z);
        let (_, nearest) = mesh
            .find_nearest_poly(&p)
            .unwrap_or_else(|| panic!("{world}: no polygon near the arrival point"));
        let off = ((nearest.x - p.x).powi(2) + (nearest.z - p.z).powi(2)).sqrt();
        assert!(
            off <= 1.5 && (nearest.y - p.y).abs() <= 3.0,
            "{world}: arrival point is {off:.2} m from the mesh, nearest {nearest:?}"
        );
    }
}
