//! Query a cached [`PackageIndex`] by class name (and optional package-name
//! substring), for archaeology sessions that need "which files hold an
//! export of class X" without re-scanning the whole `CookedPC` tree. Also
//! supports a `scan` mode that opens one package directly and enumerates
//! every export of a class by export index, with its outer chain — the
//! `PackageIndex` cannot do this on its own (see the module note below).
//!
//! Usage:
//!   `query-index <index.bin> --class <ClassName> [--package-contains <substr>] [--count-only]`
//!   `query-index scan <file.upk|umap> <ClassName> [--limit N]`
//!   `query-index cover <file.umap>`
//!
//! # Why `scan` exists: the `(package, object_name)` key collides
//!
//! [`PackageIndex`] keys `exports`/`by_class` by `(package_name,
//! object_name)`. UE3 object names are only unique **within their Outer**
//! (an `FName` is really `(name_index, instance_number)`, and
//! `ExportEntry::object_name` in this crate carries just the base string,
//! dropping `object_name_num`). A component class instanced once per owning
//! actor — which is exactly what `SGWCoverNodeComponent` is, per
//! `docs/reverse-engineering/findings/cover-system.md` — produces many
//! exports that all share the literal object name `"SGWCoverNodeComponent"`
//! in the same package, distinguished only by their outer actor. Measured on
//! `Castle_CellBlock-fffefffe.umap`: `by_class` reports 113 occurrences, but
//! `exports` holds a single overwritten entry for the collided key, so
//! `PackageIndex::find` cannot recover the other 112. `scan` opens the
//! package directly and walks `Package::exports` by index instead, so every
//! occurrence is distinguishable by its outer.

use cimmeria_upk::{extract_actors, Package, PropValue};
use cimmeria_upk_objects::PackageIndex;
use std::env;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: query-index <index.bin> --class <ClassName> [--package-contains <substr>] [--count-only]\n       query-index scan <file.upk|umap> <ClassName> [--limit N]\n       query-index cover <file.umap>"
        );
        std::process::exit(1);
    }

    if args[1] == "scan" {
        return run_scan(&args[2..]);
    }
    if args[1] == "cover" {
        return run_cover(&args[2..]);
    }

    let index_path = PathBuf::from(&args[1]);
    let class = args
        .iter()
        .position(|a| a == "--class")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let package_contains = args
        .iter()
        .position(|a| a == "--package-contains")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let count_only = args.iter().any(|a| a == "--count-only");

    let Some(class) = class else {
        eprintln!("--class is required");
        std::process::exit(1);
    };

    eprintln!("loading index from {}...", index_path.display());
    let index = PackageIndex::load(&index_path).expect("failed to load package index");
    eprintln!(
        "loaded {} exports across {} packages",
        index.export_count, index.package_count
    );

    let matches = matching_entries(&index, &class, package_contains.as_deref());

    if count_only {
        println!("{} matches for class {:?}", matches.len(), class);
        let mut per_package: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (pkg, _obj) in &matches {
            *per_package.entry(pkg.as_str()).or_insert(0) += 1;
        }
        let mut per_package: Vec<_> = per_package.into_iter().collect();
        per_package.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        for (pkg, n) in per_package {
            println!("  {pkg:<40} {n}");
        }
        return;
    }

    for (pkg, obj) in &matches {
        let loc = index.find(pkg, obj).expect("matched entry must resolve");
        println!(
            "{pkg}.{obj}\tfile={}\texport_index={}\tsize={}",
            loc.file_path.display(),
            loc.export_index,
            loc.serial_size
        );
    }
    println!("{} matches for class {:?}", matches.len(), class);
}

/// All `(package, object)` keys of the given class, optionally restricted to
/// packages whose name contains `package_contains`.
fn matching_entries(
    index: &PackageIndex,
    class: &str,
    package_contains: Option<&str>,
) -> Vec<(String, String)> {
    index
        .find_by_class(class)
        .iter()
        .filter(|(pkg, _)| match package_contains {
            Some(sub) => pkg.contains(sub),
            None => true,
        })
        .cloned()
        .collect()
}

/// `query-index scan <file> <class> [--limit N]`: enumerate every export of
/// `class` in `file` by export index (see the module doc for why the cached
/// index cannot do this).
fn run_scan(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: query-index scan <file.upk|umap> <ClassName> [--limit N]");
        std::process::exit(1);
    }
    let file = PathBuf::from(&args[0]);
    let class = &args[1];
    let limit = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<usize>().ok());

    let pkg = Package::open(&file).expect("failed to open package");
    let mut shown = 0usize;
    let mut total = 0usize;
    for (idx, export) in pkg.exports.iter().enumerate() {
        if pkg.export_class_name(export) != class {
            continue;
        }
        total += 1;
        if let Some(l) = limit {
            if shown >= l {
                continue;
            }
        }
        shown += 1;
        let owner = describe_owner(&pkg, export.package_index);
        println!(
            "export[{idx}] name={:?} num={} archetype={} size={} owner={owner}",
            export.object_name, export.object_name_num, export.archetype, export.serial_size,
        );
    }
    println!("{total} export(s) of class {class:?} in {}", file.display());
}

/// The nearest outer export's index, name and class — the owning actor for a
/// component export, one hop up `ExportEntry::package_index` (the outer
/// field; the field name is a historical UE3-ism, not a package reference).
fn describe_owner(pkg: &Package, outer: i32) -> String {
    if outer <= 0 {
        return "<none>".to_string();
    }
    match pkg.exports.get((outer - 1) as usize) {
        Some(e) => format!(
            "export[{}] {:?} ({})",
            outer - 1,
            e.object_name,
            pkg.export_class_name(e)
        ),
        None => format!("<outer_oob:{outer}>"),
    }
}

/// `query-index cover <file.umap>`: list every `SGWSpecCoverNode` actor with
/// its world-space UE3 transform, the BigWorld conversion from
/// `docs/engine/navmesh-build-pipeline.md` §1 (`bw = (ue.y/100, ue.z/100,
/// ue.x/100)`), and its `SGWCoverNodeComponent`'s `CoverHeight`/
/// `CoverQuality`/`CoverWidth`.
fn run_cover(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: query-index cover <file.umap>");
        std::process::exit(1);
    }
    let file = PathBuf::from(&args[0]);
    let pkg = Package::open(&file).expect("failed to open package");

    let actors = extract_actors(&pkg);
    let mut n = 0usize;
    for actor in actors.iter().filter(|a| a.class_name == "SGWSpecCoverNode") {
        n += 1;
        let [ue_x, ue_y, ue_z] = actor.location;
        let bw = (ue_y / 100.0, ue_z / 100.0, ue_x / 100.0);
        let scale3d = find_vector3(&actor.properties, "DrawScale3D");
        let cover = find_object_ref(&actor.properties, "CoverNodeComponent")
            .and_then(|obj| read_cover_component(&pkg, obj))
            .map(|c| c.to_string())
            .unwrap_or_else(|| "<none>".to_string());
        println!(
            "{}\tue=({ue_x:.3},{ue_y:.3},{ue_z:.3})\trot_deg=({:.2},{:.2},{:.2})\tscale3d={:?}\tbw=({:.3},{:.3},{:.3})\tcover={cover}",
            actor.full_path,
            actor.rotation_deg[0],
            actor.rotation_deg[1],
            actor.rotation_deg[2],
            scale3d,
            bw.0,
            bw.1,
            bw.2,
        );
    }
    println!("{n} SGWSpecCoverNode actor(s) in {}", file.display());
}

struct CoverProps {
    height: Option<u8>,
    quality: Option<u8>,
    width: Option<f32>,
}

impl std::fmt::Display for CoverProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "height={:?} quality={:?} width={:?}",
            self.height, self.quality, self.width
        )
    }
}

fn read_cover_component(pkg: &Package, obj_index: i32) -> Option<CoverProps> {
    if obj_index <= 0 {
        return None;
    }
    let export = pkg.exports.get((obj_index - 1) as usize)?;
    let data = pkg.read_export_data(export).ok()?;
    let props = cimmeria_upk::parse_tagged_properties(&data, 8, &pkg.names);
    let byte_prop = |name: &str| -> Option<u8> {
        props.iter().find(|p| p.name == name).and_then(|p| {
            if let PropValue::Byte(b) = &p.value {
                b.first().copied()
            } else {
                None
            }
        })
    };
    let float_prop = |name: &str| -> Option<f32> {
        props.iter().find(|p| p.name == name).and_then(|p| {
            if let PropValue::Float(f) = p.value {
                Some(f)
            } else {
                None
            }
        })
    };
    Some(CoverProps {
        height: byte_prop("CoverHeight"),
        quality: byte_prop("CoverQuality"),
        width: float_prop("CoverWidth"),
    })
}

fn find_vector3(props: &[cimmeria_upk::TaggedProperty], name: &str) -> Option<[f32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Vector { x, y, z } = p.value {
            Some([x, y, z])
        } else {
            None
        }
    })
}

fn find_object_ref(props: &[cimmeria_upk::TaggedProperty], name: &str) -> Option<i32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Object(r) = p.value {
            Some(r)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_upk_objects::package_index::ExportLocation;
    use std::collections::HashMap;

    fn fixture_index() -> PackageIndex {
        let mut exports = HashMap::new();
        let mut by_class: HashMap<String, Vec<(String, String)>> = HashMap::new();

        let entries = [
            (
                "CA-Prebuilt",
                "CoverNodeComp0",
                "SGWCoverNodeComponent",
                "/cooked/Packages/CA-Prebuilt.upk",
            ),
            (
                "GA-Arch",
                "CoverNodeComp1",
                "SGWCoverNodeComponent",
                "/cooked/Packages/GA-Arch.upk",
            ),
            (
                "Castle_CellBlock-fffefffd",
                "StaticMeshActor_12",
                "StaticMeshActor",
                "/cooked/Maps/Castle_CellBlock/Castle_CellBlock-fffefffd.umap",
            ),
        ];

        for (pkg, obj, class, file) in entries {
            let key = (pkg.to_string(), obj.to_string());
            exports.insert(
                key.clone(),
                ExportLocation {
                    file_path: PathBuf::from(file),
                    export_index: 0,
                    class_name: class.to_string(),
                    serial_size: 64,
                },
            );
            by_class.entry(class.to_string()).or_default().push(key);
        }

        PackageIndex {
            exports,
            by_class,
            package_count: 3,
            export_count: 3,
        }
    }

    #[test]
    fn finds_all_entries_of_a_class_across_packages() {
        let index = fixture_index();
        let matches = matching_entries(&index, "SGWCoverNodeComponent", None);
        assert_eq!(matches.len(), 2);
        assert!(matches.contains(&("CA-Prebuilt".to_string(), "CoverNodeComp0".to_string())));
        assert!(matches.contains(&("GA-Arch".to_string(), "CoverNodeComp1".to_string())));
    }

    #[test]
    fn package_contains_filter_narrows_the_result() {
        let index = fixture_index();
        let matches = matching_entries(&index, "SGWCoverNodeComponent", Some("CA-"));
        assert_eq!(
            matches,
            vec![("CA-Prebuilt".to_string(), "CoverNodeComp0".to_string())]
        );
    }

    #[test]
    fn a_class_with_no_matches_returns_empty() {
        let index = fixture_index();
        let matches = matching_entries(&index, "NoSuchClass", None);
        assert!(matches.is_empty());
    }
}
