//! `archetype_census` — what is in the prefab-archetype gap?
//!
//! `extract_map` reports how many `StaticMeshActor`s resolved and why
//! the rest didn't. That answers "how big is the gap"; it does not
//! answer "does the gap matter", which for a navmesh means: is the
//! missing geometry stairs, ramps and floor plates, or is it wall
//! lights and signage?
//!
//! This binary walks a map's chunks, resolves every archetype-stub
//! component through `staticmesh::archetype` — the same code path
//! `extract_map` uses — and reports, per resolved mesh:
//!
//! - instance count, and which chunks they sit in;
//! - collision triangles per instance and in total;
//! - world footprint (XZ projected area, BigWorld m²) and how much of
//!   it is near-horizontal **walkable-facing** under NavBuilder's
//!   convention (`floor_probe::recast_up` ≥ cos 45°, i.e. the UE3
//!   right-hand normal of the emitted winding points *down*);
//! - the BigWorld position of every instance, with `--positions`.
//!
//! It also re-measures two inheritance questions the extractor's
//! correctness rests on, so a future map that behaves differently
//! surfaces here rather than silently shifting geometry:
//!
//! - do instance components carry `Translation` / `Rotation` / `Scale`
//!   / `Scale3D` (which the actor-transform-only path would ignore)?
//! - do instance actors *omit* `Location` / `Rotation` / `DrawScale` /
//!   `DrawScale3D` while their actor archetype supplies one?
//!
//! ```bash
//! CIMMERIA_PACKAGE_INDEX=/path/package_index.bin \
//! cargo run -p cimmeria-navmesh-extractor --release --bin archetype_census -- \
//!   <cooked-root> <MapName> [--positions <TSV>] [--meshes <TSV>]
//! ```
//!
//! Exit codes: `0` ok, `1` usage or I/O error.
//!
//! # Layout
//!
//! - [`geometry`] — area and axis arithmetic.
//! - [`census`] — the per-chunk walk and the tallies it fills.
//! - [`report`] — the formatting, into any `Write`.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cimmeria_navmesh_extractor::staticmesh::archetype::ArchetypeCache;
use cimmeria_navmesh_extractor::umap::enumerate_chunks;
use cimmeria_upk::Package;
use cimmeria_upk_objects::PackageIndex;

// A bin target's file IS the crate root, so a bare `mod census;` would
// look for `src/bin/census.rs` and collide with every other bin. Point
// at the per-binary subdirectory explicitly.
#[path = "archetype_census/census.rs"]
mod census;
#[path = "archetype_census/geometry.rs"]
mod geometry;
#[path = "archetype_census/report.rs"]
mod report;

use census::{Census, MeshCache};

const EXIT_USAGE: u8 = 1;

const USAGE: &str =
    "usage: archetype_census <cooked-root> <MapName> [--positions TSV] [--meshes TSV]";

struct Args {
    cooked: PathBuf,
    map: String,
    positions_out: Option<PathBuf>,
    meshes_out: Option<PathBuf>,
}

fn parse_args_from(argv: &[String]) -> Result<Args, String> {
    let mut it = argv.iter().cloned();
    let cooked = PathBuf::from(it.next().ok_or_else(|| USAGE.to_string())?);
    let map = it.next().ok_or_else(|| USAGE.to_string())?;
    if map.starts_with('-') {
        return Err(format!(
            "{USAGE}\n(got a flag, {map:?}, where the map name goes)"
        ));
    }
    let mut positions_out = None;
    let mut meshes_out = None;
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--positions" => {
                positions_out = Some(PathBuf::from(it.next().ok_or("--positions needs a path")?))
            }
            "--meshes" => {
                meshes_out = Some(PathBuf::from(it.next().ok_or("--meshes needs a path")?))
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            other => return Err(format!("unknown flag {other:?}")),
        }
    }
    Ok(Args {
        cooked,
        map,
        positions_out,
        meshes_out,
    })
}

/// Walk the map and write the report. `out` is stdout in production and
/// a `Vec<u8>` under test.
fn run(out: &mut impl Write, args: &Args, index: &PackageIndex) -> Result<u8, String> {
    let map_dir = args.cooked.join("Maps").join(&args.map);
    let chunks = enumerate_chunks(&map_dir).map_err(|e| format!("{}: {e}", map_dir.display()))?;
    eprintln!("{}: {} chunks", args.map, chunks.len());

    let mut cache = ArchetypeCache::default();
    let mut mesh_cache = MeshCache::new();
    let mut census = Census::default();

    for chunk_path in &chunks {
        let chunk_name = chunk_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        let pkg = match Package::open(chunk_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("open {}: {e}", chunk_path.display());
                continue;
            }
        };
        census.add_chunk(&chunk_name, &pkg, index, &mut cache, &mut mesh_cache);
    }

    let io_err = |e: io::Error| format!("write failed: {e}");
    report::write_report(
        out,
        &args.map,
        chunks.len(),
        &census,
        cache.stats(),
        cache.len(),
    )
    .map_err(io_err)?;

    if let Some(path) = &args.meshes_out {
        let mut w = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        report::write_meshes_tsv(&mut w, &census).map_err(io_err)?;
        eprintln!("wrote {}", path.display());
    }
    if let Some(path) = &args.positions_out {
        let mut w = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        report::write_positions_tsv(&mut w, &census).map_err(io_err)?;
        eprintln!("wrote {}", path.display());
    }
    Ok(0)
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args_from(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("archetype_census: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    let index_path = match std::env::var("CIMMERIA_PACKAGE_INDEX") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("archetype_census: set CIMMERIA_PACKAGE_INDEX to a package_index.bin");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let index = match PackageIndex::load(Path::new(&index_path)) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("archetype_census: {index_path}: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    eprintln!(
        "index: {} packages, {} exports",
        index.package_count, index.export_count
    );

    let mut out = io::BufWriter::new(io::stdout().lock());
    let code = match run(&mut out, &args, &index) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("archetype_census: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    if let Err(e) = out.flush() {
        eprintln!("archetype_census: write failed: {e}");
        return ExitCode::from(EXIT_USAGE);
    }
    ExitCode::from(code)
}

#[cfg(test)]
#[path = "archetype_census/tests.rs"]
mod tests;
