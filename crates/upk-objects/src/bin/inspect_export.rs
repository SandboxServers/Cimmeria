//! Inspect the raw serial data of a specific export in a package.
//!
//! Usage: inspect-export <file.upk> <export_index> [--hex] [--props] [--prop-offset N]
//!
//! Useful for reverse-engineering object formats by examining raw bytes and
//! tagged properties.

use cimmeria_upk::Package;
use std::env;

/// Candidate byte offsets where an export's tagged-property block can start.
///
/// The prefix ahead of the property block is class-kind dependent, not version
/// dependent: Actor subclasses carry a ~32-byte prefix, StaticMesh 4, and
/// ActorComponent subclasses 8 (UComponent::Serialize writes TemplateOwnerClass
/// as an i32 plus a 4-byte pad before the script properties). Probing a fixed
/// list and taking the FIRST non-empty parse mis-reads components, because a
/// garbage parse at offset 0 happens to resolve one bogus FName pair (the
/// notorious "TabName") and then stops. Score instead: the correct offset is
/// the one that yields the most properties.
const PROP_OFFSET_CANDIDATES: [usize; 5] = [0, 4, 8, 12, 32];

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: inspect-export <file.upk> <export_index> [--hex] [--props] [--prop-offset N]"
        );
        std::process::exit(1);
    }

    let file = &args[1];
    let idx: usize = args[2].parse().expect("export_index must be a number");
    let show_hex = args.contains(&"--hex".to_string());
    let show_props = args.contains(&"--props".to_string());
    let forced_prop_offset: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--prop-offset")
        .and_then(|w| w[1].parse().ok());

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
        let best = best_prop_offset(&data, &pkg, forced_prop_offset);

        match best {
            Some((offset, props, end_offset)) => {
                println!(
                    "\n--- Properties (offset {}, end at {}) ---",
                    offset, end_offset
                );
                for p in &props {
                    let type_str = prop_type_name(&p.value);
                    // Object refs are useless as bare integers — resolve them so
                    // e.g. StaticMeshComponent.StaticMesh names the actual asset.
                    if let cimmeria_upk::PropValue::Object(r) = p.value {
                        println!(
                            "  {} ({}) = {} -> {}",
                            p.name,
                            type_str,
                            r,
                            pkg.resolve_object_path(r)
                        );
                    } else {
                        println!("  {} ({}) = {:?}", p.name, type_str, p.value);
                    }
                }
                let remaining = data.len().saturating_sub(end_offset);
                println!(
                    "\n--- Post-property binary (offset 0x{:X}, {} bytes remaining) ---",
                    end_offset, remaining
                );

                if show_hex && remaining > 0 {
                    let dump_len = remaining.min(256);
                    println!(
                        "\n--- Post-property hex (offset 0x{:X}, first {} of {} bytes) ---",
                        end_offset, dump_len, remaining
                    );
                    hex_dump(&data[end_offset..end_offset + dump_len], end_offset);
                }
            }
            None => println!("\n--- Properties: no candidate offset parsed cleanly ---"),
        }
    }

    if show_hex {
        let dump_len = data.len().min(512);
        println!("\n--- Hex dump (first {} bytes) ---", dump_len);
        hex_dump(&data[..dump_len], 0);
    }
}

/// Pick the tagged-property start offset for this export.
///
/// With `--prop-offset` the caller wins outright. Otherwise probe
/// [`PROP_OFFSET_CANDIDATES`] and keep the parse that produced the most
/// properties — a wrong offset typically yields 0 or 1 garbage property before
/// the FName validation in the parser bails out.
fn best_prop_offset(
    data: &[u8],
    pkg: &Package,
    forced: Option<usize>,
) -> Option<(usize, Vec<cimmeria_upk::TaggedProperty>, usize)> {
    let candidates: Vec<usize> = match forced {
        Some(o) => vec![o],
        None => PROP_OFFSET_CANDIDATES.to_vec(),
    };

    let mut best: Option<(usize, Vec<cimmeria_upk::TaggedProperty>, usize)> = None;
    for offset in candidates {
        if offset >= data.len() {
            continue;
        }
        let (props, end_offset) =
            cimmeria_upk::parse_tagged_properties_with_end(data, offset, &pkg.names);
        if forced.is_some() {
            return Some((offset, props, end_offset));
        }
        if props.is_empty() {
            continue;
        }
        let better = match &best {
            Some((_, best_props, _)) => props.len() > best_props.len(),
            None => true,
        };
        if better {
            best = Some((offset, props, end_offset));
        }
    }
    best
}

fn hex_dump(bytes: &[u8], base: usize) {
    for (i, chunk) in bytes.chunks(16).enumerate() {
        print!("{:04x}: ", base + i * 16);
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
