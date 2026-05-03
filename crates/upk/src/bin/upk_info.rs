//! CLI tool to dump UE3 package information.
//!
//! Usage: upk-info <file.upk|file.umap> [--classes] [--exports N]

use cimmeria_upk::Package;
use std::collections::HashMap;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: upk-info <file.upk|file.umap> [--classes] [--exports N]");
        process::exit(1);
    }

    let filepath = &args[1];
    let show_classes = args.contains(&"--classes".to_string());
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

    if export_limit > 0 {
        println!("--- Exports (first {}) ---", export_limit);
        for (i, export) in pkg.exports.iter().enumerate().take(export_limit) {
            let cls = pkg.export_class_name(export);
            let path = pkg.export_full_path(export);
            println!(
                "  [{:4}] {:<30} {} ({}b @ 0x{:08X})",
                i, cls, path, export.serial_size, export.serial_offset
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
