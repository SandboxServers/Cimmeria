//! Inspect the raw serial data of a specific export in a package.
//!
//! Usage: inspect-export <file.upk> <export_index> [--hex] [--props]
//!
//! Useful for reverse-engineering object formats by examining raw bytes and
//! tagged properties.

use cimmeria_upk::Package;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: inspect-export <file.upk> <export_index> [--hex] [--props]");
        std::process::exit(1);
    }

    let file = &args[1];
    let idx: usize = args[2].parse().expect("export_index must be a number");
    let show_hex = args.contains(&"--hex".to_string());
    let show_props = args.contains(&"--props".to_string());

    let pkg = Package::open(file).expect("Failed to open package");

    if idx >= pkg.exports.len() {
        eprintln!(
            "Export index {} out of range (max {})",
            idx,
            pkg.exports.len() - 1
        );
        std::process::exit(1);
    }

    let export = &pkg.exports[idx];
    let class_name = pkg.export_class_name(export);

    println!("=== Export {} ===", idx);
    println!("Class:  {}", class_name);
    println!("Name:   {}", export.object_name);
    println!("Path:   {}", pkg.export_full_path(export));
    println!(
        "Serial: offset={}, size={}",
        export.serial_offset, export.serial_size
    );

    let data = pkg
        .read_export_data(export)
        .expect("Failed to read export data");
    println!("Data:   {} bytes", data.len());

    // Try class-specific deserialization
    if class_name == "Texture2D" {
        match cimmeria_upk_objects::deserialize_texture2d(&data, &pkg.names) {
            Ok(tex) => {
                println!("\n--- Texture2D ---");
                println!("  Size:   {}x{}", tex.size_x, tex.size_y);
                println!("  Format: {:?}", tex.format);
                println!("  Mips:   {} (declared {})", tex.mips.len(), tex.num_mips);
                for (i, mip) in tex.mips.iter().enumerate() {
                    println!(
                        "    [{}] {}x{} — {} bytes{}",
                        i,
                        mip.size_x,
                        mip.size_y,
                        mip.data.len(),
                        if mip.is_bulk_compressed {
                            " (LZO/ZLIB compressed)"
                        } else if mip.data.is_empty() {
                            " (external/empty)"
                        } else {
                            ""
                        }
                    );
                }
            }
            Err(e) => {
                println!("\n--- Texture2D FAILED: {} ---", e);
            }
        }
    }

    if class_name == "StaticMesh" {
        match cimmeria_upk_objects::deserialize_static_mesh(&data, &pkg.names) {
            Ok(mesh) => {
                println!("\n--- StaticMesh ---");
                println!(
                    "  Bounds: origin=({:.1}, {:.1}, {:.1}) extent=({:.1}, {:.1}, {:.1}) r={:.1}",
                    mesh.bounds.origin[0],
                    mesh.bounds.origin[1],
                    mesh.bounds.origin[2],
                    mesh.bounds.extent[0],
                    mesh.bounds.extent[1],
                    mesh.bounds.extent[2],
                    mesh.bounds.sphere_radius
                );
                println!("  LODs:   {}", mesh.lod_models.len());
                for (i, lod) in mesh.lod_models.iter().enumerate() {
                    println!(
                        "    [{}] {} verts, {} tris, {} sections, {} indices",
                        i,
                        lod.num_vertices,
                        lod.num_triangles,
                        lod.sections.len(),
                        lod.indices.len()
                    );
                    if let Some(v) = lod.vertices.first() {
                        println!(
                            "         first vert: ({:.1}, {:.1}, {:.1})",
                            v.position[0], v.position[1], v.position[2]
                        );
                    }
                }
            }
            Err(e) => {
                println!("\n--- StaticMesh FAILED: {} ---", e);
            }
        }
    }

    if show_props {
        // Try parsing tagged properties at common offsets.
        // Offset 0 is typical for non-Actor objects.
        // Offset 4 may occur if there's a NetIndex prefix.
        // Offset 32 is used by some Actor subclasses.
        for &offset in &[0usize, 4, 32] {
            if offset >= data.len() {
                continue;
            }
            let (props, end_offset) =
                cimmeria_upk::parse_tagged_properties_with_end(&data, offset, &pkg.names);
            if !props.is_empty() {
                println!(
                    "\n--- Properties (offset {}, end at {}) ---",
                    offset, end_offset
                );
                for p in &props {
                    let type_str = prop_type_name(&p.value);
                    println!("  {} ({}) = {:?}", p.name, type_str, p.value);
                }
                println!(
                    "\n--- Post-property binary (offset 0x{:X}, {} bytes remaining) ---",
                    end_offset,
                    data.len().saturating_sub(end_offset)
                );
                break;
            }
        }
    }

    // Also dump post-property binary if --props was used
    if show_props && show_hex {
        // Find post-property binary offset
        for &offset in &[0usize, 4, 32] {
            if offset >= data.len() {
                continue;
            }
            let (props, end_offset) =
                cimmeria_upk::parse_tagged_properties_with_end(&data, offset, &pkg.names);
            if !props.is_empty() {
                let remaining = data.len().saturating_sub(end_offset);
                let dump_len = remaining.min(256);
                println!(
                    "\n--- Post-property hex (offset 0x{:X}, first {} of {} bytes) ---",
                    end_offset, dump_len, remaining
                );
                for (i, chunk) in data[end_offset..end_offset + dump_len]
                    .chunks(16)
                    .enumerate()
                {
                    print!("{:04x}: ", end_offset + i * 16);
                    for b in chunk {
                        print!("{:02x} ", b);
                    }
                    for _ in chunk.len()..16 {
                        print!("   ");
                    }
                    print!(" |");
                    for b in chunk {
                        let c = if *b >= 0x20 && *b < 0x7f {
                            *b as char
                        } else {
                            '.'
                        };
                        print!("{}", c);
                    }
                    println!("|");
                }
                break;
            }
        }
    }

    if show_hex {
        let dump_len = data.len().min(512);
        println!("\n--- Hex dump (first {} bytes) ---", dump_len);
        for (i, chunk) in data[..dump_len].chunks(16).enumerate() {
            print!("{:04x}: ", i * 16);
            for b in chunk {
                print!("{:02x} ", b);
            }
            // Pad short lines
            for _ in chunk.len()..16 {
                print!("   ");
            }
            // ASCII representation
            print!(" |");
            for b in chunk {
                let c = if *b >= 0x20 && *b < 0x7f {
                    *b as char
                } else {
                    '.'
                };
                print!("{}", c);
            }
            println!("|");
        }
    }
}

/// Extract a human-readable type name from a PropValue variant.
fn prop_type_name(value: &cimmeria_upk::PropValue) -> &'static str {
    match value {
        cimmeria_upk::PropValue::Int(_) => "IntProperty",
        cimmeria_upk::PropValue::Float(_) => "FloatProperty",
        cimmeria_upk::PropValue::Bool(_) => "BoolProperty",
        cimmeria_upk::PropValue::Str(_) => "StrProperty",
        cimmeria_upk::PropValue::Name(_) => "NameProperty",
        cimmeria_upk::PropValue::Object(_) => "ObjectProperty",
        cimmeria_upk::PropValue::Vector { .. } => "StructProperty:Vector",
        cimmeria_upk::PropValue::Rotator { .. } => "StructProperty:Rotator",
        cimmeria_upk::PropValue::Color { .. } => "StructProperty:Color",
        cimmeria_upk::PropValue::LinearColor { .. } => "StructProperty:LinearColor",
        cimmeria_upk::PropValue::Array(_) => "ArrayProperty",
        cimmeria_upk::PropValue::Struct { .. } => "StructProperty",
        cimmeria_upk::PropValue::Byte(_) => "ByteProperty",
        cimmeria_upk::PropValue::Raw { .. } => "Raw",
    }
}
