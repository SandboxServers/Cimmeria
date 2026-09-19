//! TEMPORARY exploration bin — kDOP presence per mesh. Deleted before push.

use cimmeria_upk::Package;
use cimmeria_upk_objects::{deserialize_static_mesh, PackageIndex};

fn main() {
    let index_path = std::env::var("CIMMERIA_PACKAGE_INDEX").unwrap();
    let index = PackageIndex::load(std::path::Path::new(&index_path)).unwrap();
    let list = std::env::args().nth(1).unwrap();
    let text = std::fs::read_to_string(&list).unwrap();
    for arg in text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()) {
        let (p, o) = arg.split_once(':').unwrap();
        let Some(loc) = index.find(p, o) else {
            println!("{arg}\tNOT IN INDEX");
            continue;
        };
        let pkg = Package::open(&loc.file_path).unwrap();
        let e = &pkg.exports[loc.export_index];
        let d = pkg.read_export_data(e).unwrap();
        match deserialize_static_mesh(&d, &pkg.names) {
            Ok(m) => {
                let lod = m.lod_models.first();
                println!(
                    "{arg}\tkdop={}\tlod0_verts={}\tlod0_idx={}\ttris={}\tsource={}",
                    m.kdop_triangles.len(),
                    lod.map(|l| l.vertices.len()).unwrap_or(0),
                    lod.map(|l| l.indices.len()).unwrap_or(0),
                    m.collision_triangles().len(),
                    if m.kdop_triangles.is_empty() { "LOD0" } else { "kDOP" },
                );
            }
            Err(err) => println!("{arg}\tDECODE FAIL {err}"),
        }
    }
}
