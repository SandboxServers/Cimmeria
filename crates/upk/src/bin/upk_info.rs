//! CLI tool to dump UE3 package information.
//!
//! Usage: upk-info <file.upk|file.umap> [--classes] [--exports N] [--imports] [--names] [--properties INDEX]

use cimmeria_upk::{parse_tagged_properties, Package, PropValue};
use std::collections::HashMap;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: upk-info <file.upk|file.umap> [--classes] [--exports N] [--imports] [--names]"
        );
        process::exit(1);
    }

    let filepath = &args[1];
    let show_classes = args.contains(&"--classes".to_string());
    let show_imports = args.contains(&"--imports".to_string());
    let show_names = args.contains(&"--names".to_string());
    let show_mesh_actors = args.contains(&"--mesh-actors".to_string());
    let property_index: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--properties")
        .and_then(|w| w[1].parse().ok());
    let export_limit: usize = args
        .windows(2)
        .find(|w| w[0] == "--exports")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(0);

    let pkg = match Package::open(filepath) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: {}", e);
            process::exit(1);
        }
    };
    if show_mesh_actors {
        for (index, export) in pkg.exports.iter().enumerate() {
            if pkg.export_class_name(export) != "StaticMeshActor" {
                continue;
            }
            let Ok(actor_data) = pkg.read_export_data(export) else {
                continue;
            };
            let props = parse_tagged_properties(&actor_data, 32, &pkg.names);
            let component_ref = props.iter().find_map(|prop| {
                (prop.name == "StaticMeshComponent")
                    .then_some(&prop.value)
                    .and_then(|value| match value {
                        PropValue::Object(reference) => Some(*reference),
                        _ => None,
                    })
            });
            let location = props.iter().find_map(|prop| {
                (prop.name == "Location")
                    .then_some(&prop.value)
                    .and_then(|value| match value {
                        PropValue::Vector { x, y, z } => Some([*x, *y, *z]),
                        _ => None,
                    })
            });
            let mut mesh = String::new();
            if let Some(reference) = component_ref {
                if reference > 0 {
                    if let Some(component) = pkg.exports.get(reference as usize - 1) {
                        if let Ok(data) = pkg.read_export_data(component) {
                            for prop in parse_tagged_properties(&data, 8, &pkg.names) {
                                if prop.name == "StaticMesh" {
                                    if let PropValue::Object(mesh_ref) = prop.value {
                                        mesh = pkg.resolve_object_path(mesh_ref);
                                    }
                                }
                            }
                        }
                        if mesh.is_empty() {
                            mesh = format!(
                                "archetype:{}",
                                pkg.resolve_object_path(component.archetype)
                            );
                        }
                    }
                }
            }
            println!(
                "{index}\t{}\t{:?}\t{mesh}",
                pkg.export_full_path(export),
                location
            );
        }
        return;
    }

    if let Some(index) = property_index {
        let export = pkg.exports.get(index).unwrap_or_else(|| {
            eprintln!("ERROR: export {index} out of range");
            process::exit(1);
        });
        let class = pkg.export_class_name(export);
        let start = if class.ends_with("Component") {
            8
        } else if cimmeria_upk::objects::actor::is_actor_class(class) {
            32
        } else {
            4
        };
        let data = pkg.read_export_data(export).unwrap_or_else(|e| {
            eprintln!("ERROR: {e}");
            process::exit(1);
        });
        println!(
            "=== Properties of export {index}: {} ({class}) ===",
            pkg.export_full_path(export)
        );
        println!(
            "archetype = {} ({})",
            pkg.resolve_object_path(export.archetype),
            export.archetype
        );
        for prop in parse_tagged_properties(&data, start, &pkg.names) {
            if let PropValue::Object(reference) = &prop.value {
                println!(
                    "{} = {} ({reference})",
                    prop.name,
                    pkg.resolve_object_path(*reference)
                );
            } else {
                println!("{} = {:?}", prop.name, prop.value);
            }
        }
        return;
    }

    let h = &pkg.header;
    println!("=== Package: {} ===", filepath);
    println!("  Epic Version:     {}", h.epic_version);
    println!("  Licensee Version: {}", h.licensee_version);
    println!("  Header Size:      {}", h.total_header_size);
    println!("  Package Flags:    0x{:08X}", h.package_flags);
    println!(
        "  GUID:             {:08X}-{:08X}-{:08X}-{:08X}",
        h.guid[0], h.guid[1], h.guid[2], h.guid[3]
    );
    println!("  Engine Version:   {}", h.engine_version);
    println!("  Cooker Version:   {}", h.cooker_version);
    println!(
        "  Compression:      0x{:08X} ({} chunks)",
        h.compression_flags,
        h.compressed_chunks.len()
    );
    println!(
        "  Names:   {:6} (offset 0x{:08X})",
        h.name_count, h.name_offset
    );
    println!(
        "  Exports: {:6} (offset 0x{:08X})",
        h.export_count, h.export_offset
    );
    println!(
        "  Imports: {:6} (offset 0x{:08X})",
        h.import_count, h.import_offset
    );
    println!();

    if show_classes {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for export in &pkg.exports {
            let cls = pkg.export_class_name(export);
            *counts.entry(cls).or_default() += 1;
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by_key(|&(_, n)| std::cmp::Reverse(n));

        println!("--- Class Distribution ({} classes) ---", sorted.len());
        for (cls, count) in &sorted {
            println!("  {:<40} {:6}", cls, count);
        }
        println!();
    }

    if show_imports {
        println!("--- Imports ({}) ---", pkg.imports.len());
        for (i, import) in pkg.imports.iter().enumerate() {
            // ObjectProperty values reference imports as a negated 1-based index;
            // print it alongside so hex-dumped refs can be looked up directly.
            println!(
                "  [{:4}] (ref {:5}) {:<28} {}",
                i,
                -(i as i32) - 1,
                format!("{}.{}", import.class_package, import.class_name),
                pkg.import_full_path(import)
            );
        }
        println!();
    }

    if show_names {
        println!("--- Names ({}) ---", pkg.names.len());
        for (i, name) in pkg.names.iter().enumerate() {
            println!("  [{:4}] 0x{:016X}  {}", i, name.flags, name.name);
        }
        println!();
    }

    if export_limit > 0 {
        println!("--- Exports (first {}) ---", export_limit);
        for (i, export) in pkg.exports.iter().enumerate().take(export_limit) {
            let cls = pkg.export_class_name(export);
            let path = pkg.export_full_path(export);
            // Archetype matters for prefab instances: a cooked component instance
            // often carries no StaticMesh property of its own and inherits it from
            // the archetype, so the ref has to be printed to follow the chain.
            println!(
                "  [{:4}] (ref {:5}) {:<30} {} ({}b @ 0x{:08X}) archetype={} outer={}",
                i,
                i as i32 + 1,
                cls,
                path,
                export.serial_size,
                export.serial_offset,
                export.archetype,
                export.package_index
            );
        }
        println!();
    }

    println!(
        "Parsed: {} names, {} imports, {} exports",
        pkg.names.len(),
        pkg.imports.len(),
        pkg.exports.len()
    );
}
