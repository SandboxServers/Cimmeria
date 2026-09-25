//! `occluder_extract` — build and measure a world's collision-geometry
//! occluder (`data/spaces/<world>.occ`, NPC AI restoration NA27, #784).
//!
//! ```text
//! occluder_extract build   --cooked-root DIR --map NAME --index CACHE --out FILE.occ
//!                          [--nav FILE.nav [--entry-points FILE.tsv]] [--page 64]
//!                          [--report FILE.tsv] [build knobs]
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
//! `build` writes the shipped, paged file (`cimmeria_occluder::paged`).
//! With `--nav` the coverage is the world's explorable area: the navmesh
//! components holding an entry point (`--entry-points`, written by
//! `tools/occluder_entry_points.py`, plus the map's own PlayerStart,
//! SGWStargate and SGWTeleporter actors), grown by `--margin` (15 m by
//! default there). Geometry outside it is not rasterised at all.
//!
//! `probe` prints one segment's verdict (eye height added to both points,
//! which are standing positions) and the columns under both ends.
//!
//! Exit codes: 0 ok, 1 usage or I/O error, 2 the build produced nothing.

mod args;
mod build;
mod measure;

use std::path::Path;
use std::process::ExitCode;

use cimmeria_occluder::{OccluderBuilder, PagedOccluder};
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
        "build" => build::run(rest),
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

fn probe(rest: &[String]) -> Result<u8, String> {
    let f = Flags::parse("probe", rest, &["occ", "from", "to", "eye"])?;
    let path = f.path("occ")?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let eye = f.f32_or("eye", 1.5)?;
    let (a, b) = (f.point("from")?, f.point("to")?);
    let (ea, eb) = ([a[0], a[1] + eye, a[2]], [b[0], b[1] + eye, b[2]]);
    let columns = if bytes.starts_with(&cimmeria_occluder::paged::PAGED_MAGIC) {
        let occ = PagedOccluder::from_bytes(bytes).map_err(|e| e.to_string())?;
        println!(
            "{} [{}]: {:?}",
            occ.label(),
            occ.short_hash(),
            occ.sight(ea, eb)
        );
        vec![
            ("from", a, occ.column(a[0], a[2])),
            ("to", b, occ.column(b[0], b[2])),
        ]
    } else {
        let occ = cimmeria_occluder::format::decode(&bytes).map_err(|e| e.to_string())?;
        println!(
            "{} [{}]: {:?}",
            occ.label(),
            occ.short_hash(),
            occ.sight(ea, eb)
        );
        vec![
            ("from", a, occ.column(a[0], a[2])),
            ("to", b, occ.column(b[0], b[2])),
        ]
    };
    for (name, p, col) in columns {
        println!("  column under {name} {p:?}:");
        for (kind, lo, hi) in col {
            println!("    {:<8} {lo:8.2} .. {hi:8.2}", kind.label());
        }
    }
    Ok(0)
}
