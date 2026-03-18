//! CLI tool to extract Kismet sequences from .umap files.
//!
//! Usage: extract-kismet <file.umap|zone_dir> [--summary] [--graph]

use cimmeria_upk::Package;
use cimmeria_upk::objects::kismet::{self, KismetNode};
use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: extract-kismet <file.umap|zone_dir> [--summary] [--graph]");
        std::process::exit(1);
    }

    let path = Path::new(&args[1]);
    let show_graph = args.contains(&"--graph".to_string());

    let mut all_nodes: HashMap<String, KismetNode> = HashMap::new();
    let mut sequences: HashMap<String, KismetNode> = HashMap::new();
    let mut tile_count = 0u32;
    let mut error_count = 0u32;

    let files: Vec<std::path::PathBuf> = if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .expect("Failed to read directory")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("umap"))
            })
            .map(|e| e.path())
            .collect();
        entries.sort();
        entries
    } else {
        vec![path.to_path_buf()]
    };

    for file_path in &files {
        let tile_name = file_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        tile_count += 1;

        match Package::open(file_path.to_str().unwrap()) {
            Ok(pkg) => {
                let nodes = kismet::extract_kismet_nodes(&pkg, &tile_name);
                for node in nodes {
                    if node.class_name == "Sequence" {
                        sequences.insert(node.node_key.clone(), node.clone());
                    }
                    all_nodes.insert(node.node_key.clone(), node);
                }
            }
            Err(e) => {
                error_count += 1;
                eprintln!("  ERROR {}: {}", tile_name, e);
            }
        }
    }

    // Class distribution
    let mut class_counts: HashMap<&str, usize> = HashMap::new();
    for node in all_nodes.values() {
        *class_counts.entry(&node.class_name).or_default() += 1;
    }
    let mut sorted: Vec<_> = class_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    // Count content chains (events with output connections)
    let chain_count: usize = all_nodes
        .values()
        .filter(|n| {
            n.class_name.starts_with("SeqEvent_")
                && n.output_links.iter().any(|l| !l.connections.is_empty())
        })
        .count();

    println!("=== Kismet Extraction Summary ===");
    println!("Tiles:       {}", tile_count);
    println!("Parse errors: {}", error_count);
    println!("Total nodes:  {}", all_nodes.len());
    println!("Sequences:    {}", sequences.len());
    println!("Content chains (events with outputs): {}", chain_count);

    println!("\nClass distribution:");
    for (cls, count) in &sorted {
        let marker = if cls.starts_with("SeqEvent_") {
            " ** EVENT"
        } else if cls.starts_with("SeqAct_") {
            " ** ACTION"
        } else if cls.starts_with("SeqCond_") {
            " * CONDITION"
        } else if cls.starts_with("SeqVar_") {
            " . VAR"
        } else {
            ""
        };
        println!("  {:<45} {:6}{}", cls, count, marker);
    }

    if show_graph {
        println!("\n--- Sequence Hierarchy ---");
        // Find root sequences
        let mut root_seqs: Vec<_> = sequences
            .values()
            .filter(|s| {
                s.parent_sequence_key.is_empty()
                    || s.parent_name == "None"
                    || !sequences.contains_key(&s.parent_sequence_key)
            })
            .collect();
        root_seqs.sort_by_key(|s| &s.full_path);

        for seq in root_seqs {
            print_sequence_tree(&all_nodes, &sequences, seq, 2, &mut std::collections::HashSet::new());
        }

        // Show wired event chains
        let mut events: Vec<_> = all_nodes
            .values()
            .filter(|n| {
                n.class_name.starts_with("SeqEvent_")
                    && n.output_links.iter().any(|l| !l.connections.is_empty())
            })
            .collect();
        events.sort_by_key(|e| &e.class_name);

        if !events.is_empty() {
            println!("\n--- Event Chains ({}) ---", events.len());
            for event in events.iter().take(100) {
                let comment = if event.obj_comment.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", event.obj_comment)
                };
                let total_conns: usize = event.output_links.iter().map(|l| l.connections.len()).sum();
                println!(
                    "  {} [{}] -> {} outputs{}",
                    event.class_name, event.tile, total_conns, comment
                );
                for link in &event.output_links {
                    for conn in &link.connections {
                        let target = all_nodes.get(&conn.target_key);
                        let target_desc = target
                            .map(|t| format!("{} '{}'", t.class_name, t.object_name))
                            .unwrap_or_else(|| format!("?{}", conn.target_key));
                        println!("    -> {} [input {}] {}", link.desc, conn.input_idx, target_desc);
                    }
                }
            }
            if events.len() > 100 {
                println!("  ... and {} more", events.len() - 100);
            }
        }
    }
}

fn print_sequence_tree(
    all_nodes: &HashMap<String, KismetNode>,
    sequences: &HashMap<String, KismetNode>,
    seq: &KismetNode,
    indent: usize,
    visited: &mut std::collections::HashSet<String>,
) {
    if visited.contains(&seq.node_key) {
        return;
    }
    visited.insert(seq.node_key.clone());

    let pad: String = " ".repeat(indent);
    let comment = if seq.obj_comment.is_empty() {
        String::new()
    } else {
        format!(" ({})", seq.obj_comment)
    };
    println!(
        "{}{} [{} children]{}",
        pad,
        seq.object_name,
        seq.sequence_objects.len(),
        comment
    );

    for obj_key in &seq.sequence_objects {
        if let Some(child_seq) = sequences.get(obj_key) {
            print_sequence_tree(all_nodes, sequences, child_seq, indent + 2, visited);
        }
    }
}
