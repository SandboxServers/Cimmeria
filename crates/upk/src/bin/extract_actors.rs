//! CLI tool to extract actors from a .umap file or directory of tiles.
//!
//! Usage: extract-actors <file.umap|zone_dir> [--json] [--summary]

use cimmeria_upk::{Package, extract_actors};
use std::env;
use std::path::Path;
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: extract-actors <file.umap|zone_dir> [--json] [--summary]");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    let as_json = args.contains(&"--json".to_string());
    let summary_only = args.contains(&"--summary".to_string());

    let mut all_actors = Vec::new();
    let mut file_count = 0u32;
    let mut error_count = 0u32;

    if path.is_dir() {
        // Process all .umap files in directory
        let entries: Vec<_> = std::fs::read_dir(path)
            .expect("Failed to read directory")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("umap"))
            })
            .collect();

        for entry in &entries {
            file_count += 1;
            match Package::open(entry.path().to_str().unwrap()) {
                Ok(pkg) => {
                    let actors = extract_actors(&pkg);
                    if !summary_only && !as_json {
                        if !actors.is_empty() {
                            println!(
                                "  {} — {} actors",
                                entry.path().file_name().unwrap().to_string_lossy(),
                                actors.len()
                            );
                        }
                    }
                    all_actors.extend(actors);
                }
                Err(e) => {
                    error_count += 1;
                    eprintln!(
                        "  ERROR {}: {}",
                        entry.path().file_name().unwrap().to_string_lossy(),
                        e
                    );
                }
            }
        }
    } else {
        // Single file
        file_count = 1;
        match Package::open(path.to_str().unwrap()) {
            Ok(pkg) => {
                all_actors = extract_actors(&pkg);
            }
            Err(e) => {
                eprintln!("ERROR: {}", e);
                std::process::exit(1);
            }
        }
    }

    if as_json {
        println!("[");
        for (i, actor) in all_actors.iter().enumerate() {
            let comma = if i + 1 < all_actors.len() { "," } else { "" };
            println!(
                "  {{\"class\": \"{}\", \"name\": \"{}\", \"path\": \"{}\", \"location\": [{}, {}, {}], \"rotation\": [{}, {}, {}], \"draw_scale\": {}}}{}",
                actor.class_name,
                actor.object_name,
                actor.full_path,
                actor.location[0], actor.location[1], actor.location[2],
                actor.rotation_deg[0], actor.rotation_deg[1], actor.rotation_deg[2],
                actor.draw_scale,
                comma
            );
        }
        println!("]");
    } else {
        // Class distribution summary
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for actor in &all_actors {
            *counts.entry(&actor.class_name).or_default() += 1;
        }
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        println!("\n=== Actor Extraction Summary ===");
        println!("Files processed: {}", file_count);
        println!("Parse errors:    {}", error_count);
        println!("Total actors:    {}", all_actors.len());
        println!("\nClass distribution:");
        for (cls, count) in &sorted {
            println!("  {:<35} {:6}", cls, count);
        }

        if !summary_only && all_actors.len() <= 50 {
            println!("\n--- Actors ---");
            for actor in &all_actors {
                println!(
                    "  {} \"{}\" @ ({:.1}, {:.1}, {:.1}) rot({:.1}, {:.1}, {:.1}) scale={:.2}",
                    actor.class_name,
                    actor.object_name,
                    actor.location[0], actor.location[1], actor.location[2],
                    actor.rotation_deg[0], actor.rotation_deg[1], actor.rotation_deg[2],
                    actor.draw_scale
                );
            }
        }
    }
}
