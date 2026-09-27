//! `cover_extract` — world-space cover nodes from cooked `.umap` chunks to
//! the `resources.cover_sets` / `resources.cover_nodes` seeds.
//!
//! ```text
//! cover_extract --map <world_id>=<world_name>=<map_dir> [--map ...]
//!               --sets-out <cover_sets.sql> --nodes-out <cover_nodes.sql>
//!               [--client-build <label>]
//! ```
//!
//! `=` separates the fields because a Windows map directory contains `:`.
//! Maps are written in the order given. See
//! `docs/engine/cover-extraction.md` for the full procedure.

use std::path::PathBuf;
use std::process::ExitCode;

use cimmeria_navmesh_extractor::cover::{self, sql};

struct MapArg {
    world_id: i32,
    world_name: String,
    dir: PathBuf,
}

fn parse_map(v: &str) -> Result<MapArg, String> {
    let mut parts = v.splitn(3, '=');
    let (Some(id), Some(name), Some(dir)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!(
            "--map expects <world_id>=<world_name>=<dir>, got {v:?}"
        ));
    };
    let world_id = id
        .parse::<i32>()
        .map_err(|e| format!("--map world id {id:?}: {e}"))?;
    if world_id <= 0 || world_id > i32::MAX / cover::SET_ID_WORLD_STRIDE - 1 {
        return Err(format!("--map world id {world_id} out of range"));
    }
    Ok(MapArg {
        world_id,
        world_name: name.to_string(),
        dir: PathBuf::from(dir),
    })
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: cover_extract --map <world_id>=<world_name>=<map_dir> [--map ...] \
         --sets-out <file> --nodes-out <file> [--client-build <label>]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    // No tracing subscriber, same as `extract_map`: every walker warning
    // is also a counter in the per-map summary line printed below.
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut maps = Vec::new();
    let (mut sets_out, mut nodes_out) = (None, None);
    let mut client_build = "unspecified".to_string();
    let mut i = 0;
    while i < args.len() {
        let value = args.get(i + 1).cloned();
        match (args[i].as_str(), value) {
            ("--map", Some(v)) => match parse_map(&v) {
                Ok(m) => maps.push(m),
                Err(e) => {
                    eprintln!("{e}");
                    return usage();
                }
            },
            ("--sets-out", Some(v)) => sets_out = Some(PathBuf::from(v)),
            ("--nodes-out", Some(v)) => nodes_out = Some(PathBuf::from(v)),
            ("--client-build", Some(v)) => client_build = v,
            _ => return usage(),
        }
        i += 2;
    }
    let (Some(sets_out), Some(nodes_out)) = (sets_out, nodes_out) else {
        return usage();
    };
    if maps.is_empty() {
        return usage();
    }

    let mut all_sets = Vec::new();
    let mut map_lines = Vec::new();
    for m in &maps {
        let extraction = match cover::extract_map_cover(&m.dir) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("{}: {e}", m.dir.display());
                return ExitCode::FAILURE;
            }
        };
        let st = &extraction.stats;
        if !st.is_balanced() {
            eprintln!(
                "{}: component accounting does not balance: {st:?}",
                m.world_name
            );
            return ExitCode::FAILURE;
        }
        let sets = cover::group_into_sets(m.world_id, &m.world_name, extraction.nodes);
        let nodes: usize = sets.iter().map(|s| s.nodes.len()).sum();
        let line = format!(
            "{} (world {}): {nodes} nodes in {} sets from {} of {} chunks; \
             {} SGWSpecCoverNode, {} CoverNodeArray ({} composed, {} unlisted); \
             defaults applied: height {}, quality {}, width {}; quality out of range {}; \
             skipped {}",
            m.world_name,
            m.world_id,
            sets.len(),
            extraction.chunks_with_cover,
            extraction.chunks_scanned,
            st.spec_nodes,
            st.array_nodes,
            st.array_nodes_composed,
            st.array_nodes_unlisted,
            st.height_defaulted,
            st.quality_defaulted,
            st.width_defaulted,
            st.quality_out_of_range,
            st.skipped(),
        );
        println!("{line}");
        map_lines.push(line);
        all_sets.extend(sets);
    }

    let command = format!(
        "cargo run --release -p cimmeria-navmesh-extractor --bin cover_extract -- {}",
        maps.iter()
            .map(|m| format!(
                "--map {}={}=<CookedPC>/Maps/{}",
                m.world_id,
                m.world_name,
                m.dir.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            ))
            .chain([
                "--sets-out db/resources/AI/Seed/cover_sets.sql".to_string(),
                "--nodes-out db/resources/AI/Seed/cover_nodes.sql".to_string(),
                format!("--client-build \"{client_build}\""),
            ])
            .collect::<Vec<_>>()
            .join(" ")
    );
    let prov = sql::Provenance {
        command: &command,
        client_build: &client_build,
        map_lines: &map_lines,
    };
    for (path, body) in [
        (&sets_out, sql::render_cover_sets(&all_sets, &prov)),
        (&nodes_out, sql::render_cover_nodes(&all_sets, &prov)),
    ] {
        if let Err(e) = std::fs::write(path, body) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
