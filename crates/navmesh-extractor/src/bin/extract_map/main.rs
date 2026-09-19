//! `extract_map` — drive the navmesh extractor over a whole cooked map
//! and report, in numbers, how much of it the StaticMesh path recovers.
//!
//! Two modes:
//!
//! ```text
//! extract_map extract --cooked-root <DIR> --map <NAME> --out <DIR> --index <CACHE>
//!                     [--chunk-filter <SUBSTR>] [--report <TSV>] [--classes <TSV>]
//!
//! extract_map probe   --obj-dir <DIR> [--mapping all|<LABEL>[,<LABEL>…]]
//!                     [--points <TSV>] [--report <TSV>] [--detail <TSV>]
//!                     [--below <F>] [--above <F>] [--neighbourhood <F>]
//! ```
//!
//! `extract` writes one `<chunkid>o.obj` per chunk plus a combined
//! `<map>.obj`, and a per-chunk coverage TSV (see
//! [`cimmeria_navmesh_extractor::coverage`]).
//!
//! `probe` reads those OBJs back and asks, for every known-walkable
//! world point, whether an upward-facing triangle sits underneath it —
//! under each candidate UE3→BigWorld axis mapping. See
//! [`cimmeria_navmesh_extractor::floor_probe`].
//!
//! The `--index` cache is built on first use (~45 s over the ~5000
//! packages in `CookedPC`) and reloaded from disk afterwards.
//!
//! Args are hand-parsed against `std::env::args` — this crate keeps a
//! tight dependency budget and a clap dependency would be the only
//! thing in it that isn't UE3 parsing.

mod args;
mod extract_mode;
mod probe_mode;

use args::Args;

pub(crate) const USAGE: &str = "\
extract_map — UE3 .umap -> OBJ collision extraction + floor-coverage probe

USAGE:
  extract_map extract --cooked-root <DIR> --map <NAME> --out <DIR> --index <CACHE>
                      [--chunk-filter <SUBSTR>] [--report <TSV>] [--classes <TSV>]
  extract_map probe   --obj-dir <DIR> [--mapping all|<LABEL>[,<LABEL>...]]
                      [--points <TSV>] [--report <TSV>] [--detail <TSV>]
                      [--below <F>] [--above <F>] [--neighbourhood <F>]

extract:
  --cooked-root <DIR>   CookedPC directory (the one holding Maps/).
  --map <NAME>          Map directory name under Maps/, e.g. Castle.
  --out <DIR>           Where the .obj files and reports are written.
  --index <CACHE>       PackageIndex cache file. Built and saved if absent.
  --chunk-filter <S>    Only process chunks whose filename contains S.
  --report <TSV>        Per-chunk coverage TSV. Default <out>/coverage.tsv.
  --classes <TSV>       Export-class census TSV. Default <out>/coverage_classes.tsv.

probe:
  --obj-dir <DIR>       Directory of <chunkid>o.obj files (an extract --out).
  --mapping <SEL>       `all` (48 candidates, the default) or a comma-separated
                        list of labels such as +Y+Z+X. A label reads as
                        \"BigWorld x from ..., BigWorld y (up) from ...,
                        BigWorld z from ...\".
  --points <TSV>        label<TAB>HIGH|MEDIUM<TAB>x<TAB>y<TAB>z[<TAB>source].
                        Defaults to the built-in Castle probe set.
  --report <TSV>        Mapping ranking. Default <obj-dir>/probe_mappings.tsv.
  --detail <TSV>        Per-point detail. Default <obj-dir>/probe_points.tsv.
  --below <F>           Floor may sit this far below the point (default 1.5).
  --above <F>           ...or this far above it (default 0.5).
  --neighbourhood <F>   Radius for the \"any geometry near here\" counters
                        (default 5.0).
";

fn main() {
    // No `tracing-subscriber` here on purpose — this crate's dependency
    // budget is UE3 parsing only, and the binary prints its own summary
    // plus two TSVs. The library's `tracing` calls stay no-ops unless a
    // caller installs a subscriber.
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match Args::parse(&argv) {
        Ok(Some(a)) => a,
        Ok(None) => {
            print!("{USAGE}");
            return;
        }
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            std::process::exit(2);
        }
    };

    let result = match args {
        Args::Extract(a) => extract_mode::run(a),
        Args::Probe(a) => probe_mode::run(a),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

/// `n` as a percentage of `of`, 0 when `of` is 0.
pub(crate) fn pct(n: u64, of: u64) -> f64 {
    if of == 0 {
        0.0
    } else {
        100.0 * n as f64 / of as f64
    }
}
