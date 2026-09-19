//! TEMPORARY exploration bin — dump the actor-archetype tagged props for
//! a list of dotted paths (`Pkg.Prefab.Actor`). Deleted before push.

use cimmeria_navmesh_extractor::staticmesh::archetype::export_outer_path;
use cimmeria_upk::Package;
use cimmeria_upk_objects::PackageIndex;

fn main() {
    let index = PackageIndex::load(std::path::Path::new(
        &std::env::var("CIMMERIA_PACKAGE_INDEX").unwrap(),
    ))
    .unwrap();
    let list = std::env::args().nth(1).unwrap();
    for line in std::fs::read_to_string(&list).unwrap().lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('.').collect();
        let (root, want) = (parts[0], parts[1..].join("."));
        let Some(loc) = index.find(root, parts[1]) else {
            println!("{line}\tPKG NOT FOUND");
            continue;
        };
        let pkg = Package::open(&loc.file_path).unwrap();
        let Some(idx) = (0..pkg.exports.len()).find(|i| export_outer_path(&pkg, *i) == want) else {
            println!("{line}\tEXPORT NOT FOUND");
            continue;
        };
        let e = &pkg.exports[idx];
        let d = pkg.read_export_data(e).unwrap();
        let props = cimmeria_upk::parse_tagged_properties(&d, 32, &pkg.names);
        println!(
            "{line}\tclass={}\tserial={}\tprops={:?}",
            pkg.export_class_name(e),
            e.serial_size,
            props
                .iter()
                .map(|p| format!("{}={}", p.name, short(&p.value)))
                .collect::<Vec<_>>()
        );
    }
}

fn short(v: &cimmeria_upk::PropValue) -> String {
    use cimmeria_upk::PropValue::*;
    match v {
        Int(i) => format!("{i}"),
        Float(f) => format!("{f}"),
        Bool(b) => format!("{b}"),
        Name(n) => n.clone(),
        Object(o) => format!("Obj{o}"),
        Vector { x, y, z } => format!("({x},{y},{z})"),
        Rotator { pitch, yaw, roll } => format!("R({pitch},{yaw},{roll})"),
        other => format!("{other:?}").chars().take(40).collect(),
    }
}
