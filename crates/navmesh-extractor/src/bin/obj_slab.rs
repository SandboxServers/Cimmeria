//! `obj_slab` — measure the *source* geometry in a box of BigWorld space.
//!
//! The companion to `nav_inspect --gaps`: once the gap finder has named a
//! place where two walkable components nearly meet, this says what is
//! actually there in the chunk OBJs the navmesh was built from — floors and
//! their spacing, how wide an opening is, and how much of the surface is too
//! steep to walk.
//!
//! ```text
//! obj_slab <chunk-dir>
//!     [--at NAME=X,Y,Z[,HALF_XZ[,HALF_Y]]]...   box centred on a point
//!     [--box NAME=X0,Y0,Z0,X1,Y1,Z1]...        explicit box
//!     [--column X,Z]...                        surfaces stacked at a point
//!     [--line X0,Z0,X1,Z1]                     free runs across a line
//!     [--band YLO,YHI]                         occupancy band (default 0.2..1.8
//!                                              above the box floor)
//!     [--levels BUCKET_METRES]                 horizontal area by height
//!     [--cell METRES]                          occupancy cell (default 0.1)
//!     [--tilt DEGREES]                         wall threshold (default 45)
//!     [--margin METRES]                        chunk pre-filter slack (default 60)
//! ```
//!
//! Exit codes: `0` ok, `1` usage/IO error, `2` a box came back empty (no
//! source geometry there at all — which is itself the answer when a gap
//! coincides with an un-extracted actor).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cimmeria_navmesh_extractor::obj_slab::{Slab, SlabSet};

const EXIT_USAGE: u8 = 1;
const EXIT_EMPTY: u8 = 2;

const USAGE: &str = "usage: obj_slab <chunk-dir> [--at NAME=X,Y,Z[,HALF_XZ[,HALF_Y]]]... \
                     [--box NAME=X0,Y0,Z0,X1,Y1,Z1]... [--column X,Z]... [--line X0,Z0,X1,Z1] \
                     [--band YLO,YHI] [--levels M] [--cell M] [--tilt DEG] [--margin M]";

struct Args {
    dir: PathBuf,
    slabs: Vec<Slab>,
    columns: Vec<[f32; 2]>,
    line: Option<[f32; 4]>,
    band: Option<[f32; 2]>,
    levels: Option<f32>,
    cell: f32,
    tilt: f32,
    margin: f32,
}

fn nums(s: &str) -> Result<Vec<f32>, String> {
    s.split(',')
        .map(|p| {
            p.trim()
                .parse::<f32>()
                .map_err(|e| format!("bad number {p:?}: {e}"))
        })
        .collect()
}

/// `NAME=X,Y,Z[,HALF_XZ[,HALF_Y]]` → a box centred on the point. The default
/// half-extents (8 m horizontally, 12 m vertically) cover a doorway and the
/// storey above and below it.
fn parse_at(v: &str) -> Result<Slab, String> {
    let (name, rest) = v
        .split_once('=')
        .ok_or_else(|| format!("--at needs NAME=X,Y,Z — got {v:?}"))?;
    let n = nums(rest)?;
    if n.len() < 3 || n.len() > 5 {
        return Err(format!("--at needs 3 to 5 numbers, got {}", n.len()));
    }
    let hxz = n.get(3).copied().unwrap_or(8.0);
    let hy = n.get(4).copied().unwrap_or(12.0);
    Ok(Slab::around(name, [n[0], n[1], n[2]], [hxz, hy, hxz]))
}

/// `NAME=X0,Y0,Z0,X1,Y1,Z1` → an explicit box. Corners are sorted, so either
/// order works.
fn parse_box(v: &str) -> Result<Slab, String> {
    let (name, rest) = v
        .split_once('=')
        .ok_or_else(|| format!("--box needs NAME=X0,Y0,Z0,X1,Y1,Z1 — got {v:?}"))?;
    let n = nums(rest)?;
    if n.len() != 6 {
        return Err(format!("--box needs 6 numbers, got {}", n.len()));
    }
    let lo = [n[0].min(n[3]), n[1].min(n[4]), n[2].min(n[5])];
    let hi = [n[0].max(n[3]), n[1].max(n[4]), n[2].max(n[5])];
    Ok(Slab::new(name, lo, hi))
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let mut dir = None;
    let mut slabs = Vec::new();
    let mut columns = Vec::new();
    let mut line = None;
    let mut band = None;
    let mut levels = None;
    let mut cell = 0.1f32;
    let mut tilt = 45.0f32;
    let mut margin = 60.0f32;

    while let Some(a) = it.next() {
        match a.as_str() {
            "--at" => slabs.push(parse_at(&it.next().ok_or("--at needs a value")?)?),
            "--box" => slabs.push(parse_box(&it.next().ok_or("--box needs a value")?)?),
            "--column" => {
                let n = nums(&it.next().ok_or("--column needs X,Z")?)?;
                if n.len() != 2 {
                    return Err("--column needs X,Z".into());
                }
                columns.push([n[0], n[1]]);
            }
            "--line" => {
                let n = nums(&it.next().ok_or("--line needs X0,Z0,X1,Z1")?)?;
                if n.len() != 4 {
                    return Err("--line needs X0,Z0,X1,Z1".into());
                }
                line = Some([n[0], n[1], n[2], n[3]]);
            }
            "--band" => {
                let n = nums(&it.next().ok_or("--band needs YLO,YHI")?)?;
                if n.len() != 2 {
                    return Err("--band needs YLO,YHI".into());
                }
                band = Some([n[0], n[1]]);
            }
            "--levels" => levels = Some(one(&mut it, "--levels")?),
            "--cell" => cell = one(&mut it, "--cell")?,
            "--tilt" => tilt = one(&mut it, "--tilt")?,
            "--margin" => margin = one(&mut it, "--margin")?,
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if other.starts_with('-') => return Err(format!("unknown flag {other:?}")),
            other => {
                if dir.is_some() {
                    return Err(format!("unexpected extra argument {other:?}"));
                }
                dir = Some(PathBuf::from(other));
            }
        }
    }
    if slabs.is_empty() {
        return Err("at least one --at or --box is required".into());
    }
    Ok(Args {
        dir: dir.ok_or_else(|| USAGE.to_string())?,
        slabs,
        columns,
        line,
        band,
        levels,
        cell,
        tilt,
        margin,
    })
}

fn one(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<f32, String> {
    it.next()
        .ok_or_else(|| format!("{flag} needs a value"))?
        .parse()
        .map_err(|e| format!("bad {flag}: {e}"))
}

fn report(slab: &Slab, args: &Args) {
    println!(
        "\nbox {}  x[{:.1}, {:.1}]  y[{:.1}, {:.1}]  z[{:.1}, {:.1}]",
        slab.name,
        slab.bmin[0],
        slab.bmax[0],
        slab.bmin[1],
        slab.bmax[1],
        slab.bmin[2],
        slab.bmax[2]
    );
    println!("  triangles  {}", slab.tris.len());
    if slab.tris.is_empty() {
        println!("  EMPTY — no source geometry in this box");
        return;
    }
    let chunks: Vec<String> = slab
        .by_chunk
        .iter()
        .map(|(k, v)| format!("{k}:{v}"))
        .collect();
    println!("  from       {}", chunks.join(" "));

    let profile = slab.slope_profile(&[5.0, 45.0, 60.0, 90.1]);
    println!(
        "  footprint  flat<=5deg {:.1} m^2 | walkable<=45deg {:.1} m^2 | ramp 45-60 {:.1} m^2 | \
         steep>60 {:.1} m^2",
        profile[0].1, profile[1].1, profile[2].1, profile[3].1
    );

    if let Some(bucket) = args.levels {
        println!("  levels (near-horizontal area by {bucket:.2} m of height):");
        for l in slab.level_histogram(bucket, 60.0) {
            println!(
                "      y[{:7.2},{:7.2}]  {:9.1} m^2  {:6} tris   x[{:.1}, {:.1}] z[{:.1}, {:.1}]",
                l.y_lo, l.y_hi, l.area_xz, l.tris, l.bmin[0], l.bmax[0], l.bmin[1], l.bmax[1]
            );
        }
    }

    for c in &args.columns {
        let col = slab.column(c[0], c[1], 60.0);
        if col.is_empty() {
            println!("  column ({:.1}, {:.1}): no surface", c[0], c[1]);
            continue;
        }
        println!("  column ({:.1}, {:.1}):", c[0], c[1]);
        let mut prev: Option<f32> = None;
        for s in &col {
            let step = prev.map(|p| s.y - p).unwrap_or(0.0);
            println!(
                "      y={:8.2}  tilt={:5.1} deg  faces_up={:<5}  step_from_below={:+.2} m",
                s.y, s.tilt_degrees, s.faces_up, step
            );
            prev = Some(s.y);
        }
    }

    if let Some(l) = args.line {
        let [ylo, yhi] = args
            .band
            .unwrap_or([slab.bmin[1] + 0.2, slab.bmin[1] + 1.8]);
        let occ = slab.occupancy(ylo, yhi, args.cell, args.tilt);
        let runs = occ.free_runs([l[0], l[1]], [l[2], l[3]]);
        println!(
            "  line ({:.1}, {:.1}) -> ({:.1}, {:.1}), band y[{:.2}, {:.2}], \
             walls steeper than {:.0} deg, cell {:.2} m",
            l[0], l[1], l[2], l[3], ylo, yhi, args.tilt, args.cell
        );
        if runs.is_empty() {
            println!("      fully blocked");
        }
        for (s, e) in &runs {
            println!(
                "      clear {:.2} m  (at {:.2}..{:.2} m along the line)",
                e - s,
                s,
                e
            );
        }
    }
}

fn run(args: Args) -> Result<u8, String> {
    let dir: &Path = &args.dir;
    let mut set = SlabSet::new(slabs_of(&args));
    set.margin = args.margin;
    set.load(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    println!(
        "chunks     {} read, {} skipped by the grid pre-filter (margin {:.0} m)",
        set.chunks_read.len(),
        set.chunks_skipped,
        args.margin
    );
    let mut empty = false;
    for slab in &set.slabs {
        report(slab, &args);
        empty |= slab.tris.is_empty();
    }
    Ok(if empty { EXIT_EMPTY } else { 0 })
}

/// Clone the requested boxes out of `args` — [`Slab`] carries its own
/// triangle buffer, which `SlabSet` fills, so the set needs owned copies.
fn slabs_of(args: &Args) -> Vec<Slab> {
    args.slabs
        .iter()
        .map(|s| Slab::new(s.name.clone(), s.bmin, s.bmax))
        .collect()
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("obj_slab: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    match run(args) {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("obj_slab: {e}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_defaults_to_an_8m_horizontal_12m_vertical_box() {
        let s = parse_at("g=280,43.4,875").unwrap();
        assert_eq!(s.name, "g");
        assert!((s.bmin[0] - 272.0).abs() < 1e-3);
        assert!((s.bmax[0] - 288.0).abs() < 1e-3);
        assert!((s.bmin[1] - 31.4).abs() < 1e-3);
        assert!((s.bmax[1] - 55.4).abs() < 1e-3);
    }

    #[test]
    fn at_accepts_explicit_half_extents() {
        let s = parse_at("g=0,0,0,2,3").unwrap();
        assert_eq!(s.bmin, [-2.0, -3.0, -2.0]);
        assert_eq!(s.bmax, [2.0, 3.0, 2.0]);
    }

    #[test]
    fn box_corners_are_sorted_so_either_order_works() {
        let a = parse_box("b=10,20,30,0,0,0").unwrap();
        let b = parse_box("b=0,0,0,10,20,30").unwrap();
        assert_eq!(a.bmin, b.bmin);
        assert_eq!(a.bmax, b.bmax);
        assert_eq!(a.bmin, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn malformed_specs_are_rejected_rather_than_defaulted() {
        assert!(parse_at("no-equals").is_err());
        assert!(parse_at("g=1,2").is_err(), "3 numbers minimum");
        assert!(parse_at("g=1,2,3,4,5,6").is_err(), "5 numbers maximum");
        assert!(parse_box("b=1,2,3").is_err());
        assert!(parse_box("b=1,2,3,4,5,x").is_err());
    }
}
