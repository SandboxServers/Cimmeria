//! Build a cross-package export index from a CookedPC directory.
//!
//! Usage: build-package-index <cooked_pc_path> [--output index.bin]
//!
//! Scans all .upk and .umap files, builds an index of every export, and saves
//! it as a bincode file for fast loading by the scene editor.

use cimmeria_upk_objects::PackageIndex;
use std::env;
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: build-package-index <cooked_pc_path> [--output index.bin]");
        std::process::exit(1);
    }

    let cooked_pc = Path::new(&args[1]);
    let output = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("package_index.bin");

    if !cooked_pc.is_dir() {
        eprintln!("Error: {} is not a directory", cooked_pc.display());
        std::process::exit(1);
    }

    println!("Building package index from: {}", cooked_pc.display());
    let start = Instant::now();

    let index = PackageIndex::build(cooked_pc).expect("Failed to build index");

    println!(
        "Indexed {} exports across {} packages in {:.1}s",
        index.export_count,
        index.package_count,
        start.elapsed().as_secs_f32()
    );

    // Show class distribution
    let mut class_counts: Vec<_> = index
        .by_class
        .iter()
        .map(|(k, v)| (k.as_str(), v.len()))
        .collect();
    class_counts.sort_by_key(|&(_, n)| std::cmp::Reverse(n));

    println!("\nTop 20 classes:");
    for (cls, count) in class_counts.iter().take(20) {
        println!("  {:<40} {:>8}", cls, count);
    }

    // Highlight rendering-relevant classes
    let mesh_count = index
        .by_class
        .get("StaticMesh")
        .map(|v| v.len())
        .unwrap_or(0);
    let tex_count = index
        .by_class
        .get("Texture2D")
        .map(|v| v.len())
        .unwrap_or(0);
    let mat_count = index
        .by_class
        .get("MaterialInstanceConstant")
        .map(|v| v.len())
        .unwrap_or(0);
    println!(
        "\nRendering assets: {} meshes, {} textures, {} materials",
        mesh_count, tex_count, mat_count
    );

    // Save
    index.save(Path::new(output)).expect("Failed to save index");
    println!("Saved to {}", output);
}
