//! `occluder_extract` — build and measure a world's collision-geometry
//! occluder (`data/spaces/<world>.occ`, NPC AI restoration NA27, #784).
//!
//! ```text
//! occluder_extract build   --cooked-root DIR --map NAME --index CACHE --out FILE.occ
//!                          [build knobs]
//! occluder_extract measure --cooked-root DIR --map NAME --index CACHE
//!                          [--cells 0.25,0.5,1.0] [build knobs]
//!                          [--nav FILE.nav [--pairs 4000] [--seed N] [--eye 1.5]
//!                           [--max-dy 4] [--max-component-area M2|none]
//!                           [--clearances 0.3] [--pairs-out FILE.tsv]]
//!                          [--report FILE.tsv] [--write-dir DIR]
//! occluder_extract probe   --occ FILE.occ --from X,Y,Z --to X,Y,Z [--eye 1.5]
//!
//! build knobs: [--cell 0.5] [--terrain-pitch 1.0|none] [--margin 2.0|none]
//!              [--merge-gap 0.1] [--y-step 0.1]
//! ```
//!
//! `--map` is the client map directory name (`Castle_CellBlock`). The file
//! name the server loads is the world name lower-cased with spaces as
//! underscores, the `.nav` rule in `space_manager/lifecycle.rs`:
//! `data/spaces/castle_cellblock.occ`.
//!
//! `probe` prints one segment's verdict (eye height added to both points,
//! which are standing positions) and the columns under both ends.
//!
//! Exit codes: 0 ok, 1 usage or I/O error, 2 the build produced nothing.

mod args;
mod measure;

use std::path::Path;
use std::process::ExitCode;

use cimmeria_navmesh_extractor::occluder::for_each_chunk;
use cimmeria_occluder::{Occluder, OccluderBuilder, Source};
use cimmeria_upk_objects::PackageIndex;

use args::Flags;

const BUILD_KNOBS: [&str; 5] = ["cell", "terrain-pitch", "margin", "merge-gap", "y-step"];

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(mode) = argv.first() else {
        eprintln!("usage: occluder_extract <build|measure|probe> ...  (see the module docs)");
        return ExitCode::from(1);
    };
    let rest = &argv[1..];
    let result = match mode.as_str() {
        "build" => build(rest),
        "measure" => measure::run(rest),
        "probe" => probe(rest),
        other => Err(format!(
            "unknown mode {other:?}; expected build, measure or probe"
        )),
    };
    match result {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("occluder_extract: {e}");
            ExitCode::from(1)
        }
    }
}

/// Load the package index cache (`extract_map` builds it).
pub(crate) fn load_index(path: &Path) -> Result<PackageIndex, String> {
    PackageIndex::load(path).map_err(|e| {
        format!(
            "cannot load package index {}: {e} (run extract_map once to build it)",
            path.display()
        )
    })
}

/// Feed a `.nav`'s polygons to `b` as its coverage mask.
pub(crate) fn add_nav_coverage(b: &mut OccluderBuilder, nav: &Path) -> Result<(), String> {
    use cimmeria_navmesh_extractor::nav_components::NavGraph;
    use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;
    let mut file = std::fs::File::open(nav).map_err(|e| format!("{}: {e}", nav.display()))?;
    let graph = NavGraph::from_nav(&XrcNav::read(&mut file).map_err(|e| e.to_string())?);
    for poly in &graph.polys {
        let v: Vec<[f32; 3]> = poly
            .verts
            .iter()
            .map(|&i| graph.verts[i as usize])
            .collect();
        for k in 1..v.len().saturating_sub(1) {
            b.add_coverage_triangle(&[v[0], v[k], v[k + 1]]);
        }
    }
    Ok(())
}

fn build(rest: &[String]) -> Result<u8, String> {
    let mut allowed = vec!["cooked-root", "map", "index", "out", "coverage-nav"];
    allowed.extend(BUILD_KNOBS);
    let f = Flags::parse("build", rest, &allowed)?;
    let cooked = f.path("cooked-root")?;
    let map = f.req("map")?.to_string();
    let out = f.path("out")?;
    let index = load_index(&f.path("index")?)?;
    let params = f.build_params()?;
    let label = format!(
        "{map} cell={} terrain_pitch={:?} margin={:?} merge_gap={} y_step={}",
        params.cell, params.terrain_pitch, params.margin, params.merge_gap, params.y_step
    );
    let mut builder = OccluderBuilder::new(params, label).map_err(|e| e.to_string())?;
    if let Some(nav) = f.opt("coverage-nav") {
        add_nav_coverage(&mut builder, Path::new(nav))?;
    }
    let started = std::time::Instant::now();
    let stats = for_each_chunk(&cooked.join("Maps").join(&map), Some(&index), |c| {
        for t in &c.geometry {
            builder.add_triangle(t, Source::Geometry);
        }
        for t in &c.terrain {
            builder.add_triangle(t, Source::Terrain);
        }
    })
    .map_err(|e| e.to_string())?;
    if stats.terrain_parse_failures > 0 || stats.bsp_models_failed > 0 {
        eprintln!(
            "warning: {} terrain and {} BSP decode failures; the occluder is missing that geometry",
            stats.terrain_parse_failures, stats.bsp_models_failed
        );
    }
    let occ = match builder.finish() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("occluder_extract: {map}: {e}");
            return Ok(2);
        }
    };
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    occ.save(&out).map_err(|e| e.to_string())?;
    let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    println!(
        "{map}: {} triangles ({} chunks) -> {} ({bytes} bytes, {} bytes RAM, hash {}) in {:.1}s",
        stats.triangles(),
        stats.chunks,
        out.display(),
        occ.ram_bytes(),
        occ.short_hash(),
        started.elapsed().as_secs_f64()
    );
    for l in occ.layers() {
        let (dx, dz) = l.dims();
        println!(
            "  layer {}: cell {} m, {dx} x {dz} cells, {} covered / {} stored tiles, {} spans",
            l.kind().label(),
            l.cell_size(),
            l.covered_tiles(),
            l.stored_tiles(),
            l.span_count()
        );
    }
    if let Some(h) = occ.heightfield() {
        let (dx, dz) = h.dims();
        println!(
            "  terrain heightfield: pitch {} m, {dx} x {dz} patches, {} covered / {} stored tiles, {} patches",
            h.pitch(),
            h.covered_tiles(),
            h.stored_tiles(),
            h.patch_count()
        );
    }
    Ok(0)
}

fn probe(rest: &[String]) -> Result<u8, String> {
    let f = Flags::parse("probe", rest, &["occ", "from", "to", "eye"])?;
    let occ = Occluder::load(&f.path("occ")?).map_err(|e| e.to_string())?;
    let eye = f.f32_or("eye", 1.5)?;
    let (a, b) = (f.point("from")?, f.point("to")?);
    let (ea, eb) = ([a[0], a[1] + eye, a[2]], [b[0], b[1] + eye, b[2]]);
    let sight = occ.sight(ea, eb);
    println!("{} [{}]: {:?}", occ.label(), occ.short_hash(), sight);
    for (name, p) in [("from", a), ("to", b)] {
        println!("  column under {name} {p:?}:");
        for (kind, lo, hi) in occ.column(p[0], p[2]) {
            println!("    {:<8} {lo:8.2} .. {hi:8.2}", kind.label());
        }
    }
    Ok(0)
}
