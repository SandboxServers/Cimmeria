//! `nav_inspect` — report the connectivity of an XRC `.nav` and check that
//! named probe points land on one shared walkable region.
//!
//! This is the acceptance gate for a navmesh build: NavBuilder exits 0 even
//! when it writes nothing at all (`builder.cpp::exportNavmesh` logs `FAULT`
//! and returns `void`), so "the command succeeded" proves nothing. A mesh is
//! only usable when the places NPCs and players actually stand are reachable
//! from each other.
//!
//! ```text
//! nav_inspect <file.nav>
//!     [--probe NAME=X,Y,Z]...      named probe point in BigWorld coords
//!     [--probes FILE]              probe list: "NAME X Y Z" or "X Y Z"
//!     [--h-tol METRES]             max horizontal gap to a poly (default 2.0)
//!     [--v-tol METRES]             max |vertical| gap to a poly (default 3.0)
//!     [--max-components N]         fail if the mesh has more than N regions
//!     [--gaps]                     report the gaps between the components
//!                                  the probes landed in
//!     [--gap-pair A,B]...          report the gaps between two component ids
//!     [--gap-h METRES]             gap search radius, horizontal (default 3.0)
//!     [--gap-v METRES]             gap search radius, vertical   (default 3.0)
//!     [--gap-count N]              approaches to list per pair   (default 5)
//!     [--quiet]                    suppress the per-component table
//! ```
//!
//! `--gaps` answers the follow-up question to a split probe set: *where* are
//! the two regions closest, and would a chain of small bridges join them?
//! See [`cimmeria_navmesh_extractor::nav_components::gaps`].
//!
//! Exit codes: `0` ok, `1` usage/IO error, `2` a probe was out of tolerance,
//! `3` the probes did not all land in the same component, `4` the mesh
//! exceeded `--max-components`. Gap reporting never changes the exit code —
//! it is diagnostics for a failure the probe gate has already reported.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cimmeria_navmesh_extractor::nav_components::{ComponentStat, GapGraph, NavGraph};
use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;

const EXIT_USAGE: u8 = 1;
const EXIT_PROBE_OUT_OF_TOLERANCE: u8 = 2;
const EXIT_PROBES_DISCONNECTED: u8 = 3;
const EXIT_TOO_MANY_COMPONENTS: u8 = 4;

struct Probe {
    name: String,
    pos: [f32; 3],
}

struct Args {
    path: PathBuf,
    probes: Vec<Probe>,
    h_tol: f32,
    v_tol: f32,
    max_components: Option<u32>,
    /// Report gaps between whichever components the probes resolved into.
    gaps: bool,
    /// Explicit component pairs to report gaps for.
    gap_pairs: Vec<(u32, u32)>,
    gap_h: f32,
    gap_v: f32,
    gap_count: usize,
    quiet: bool,
}

const USAGE: &str = "usage: nav_inspect <file.nav> [--probe NAME=X,Y,Z]... [--probes FILE] \
                     [--h-tol M] [--v-tol M] [--max-components N] [--gaps] \
                     [--gap-pair A,B]... [--gap-h M] [--gap-v M] [--gap-count N] [--quiet]";

fn parse_xyz(s: &str) -> Result<[f32; 3], String> {
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(format!("expected X,Y,Z — got {s:?}"));
    }
    let mut out = [0.0f32; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p
            .parse::<f32>()
            .map_err(|e| format!("bad coordinate {p:?}: {e}"))?;
    }
    Ok(out)
}

/// One probe per line: `NAME X Y Z` or bare `X Y Z`. `#` starts a comment.
fn parse_probe_file(path: &Path) -> Result<Vec<Probe>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tok: Vec<&str> = line.split_whitespace().collect();
        let (name, nums) = match tok.len() {
            3 => (format!("probe{}", out.len()), &tok[0..3]),
            4 => (tok[0].to_string(), &tok[1..4]),
            n => {
                return Err(format!(
                    "{}:{}: expected 3 or 4 fields, got {n}",
                    path.display(),
                    lineno + 1
                ))
            }
        };
        let mut pos = [0.0f32; 3];
        for (i, p) in nums.iter().enumerate() {
            pos[i] = p.parse::<f32>().map_err(|e| {
                format!(
                    "{}:{}: bad coordinate {p:?}: {e}",
                    path.display(),
                    lineno + 1
                )
            })?;
        }
        out.push(Probe { name, pos });
    }
    Ok(out)
}

fn parse_args() -> Result<Args, String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    parse_args_from(&argv)
}

/// The real argument parser. Split out from [`parse_args`] so the flag
/// handling can be exercised without a process boundary.
fn parse_args_from(argv: &[String]) -> Result<Args, String> {
    let mut it = argv.iter().cloned();
    let mut path: Option<PathBuf> = None;
    let mut probes = Vec::new();
    let mut h_tol = 2.0f32;
    let mut v_tol = 3.0f32;
    let mut max_components = None;
    let mut gaps = false;
    let mut gap_pairs = Vec::new();
    let mut gap_h = 3.0f32;
    let mut gap_v = 3.0f32;
    let mut gap_count = 5usize;
    let mut quiet = false;

    while let Some(a) = it.next() {
        match a.as_str() {
            "--probe" => {
                let v = it.next().ok_or("--probe needs NAME=X,Y,Z")?;
                let (name, coords) = v
                    .split_once('=')
                    .ok_or_else(|| format!("--probe needs NAME=X,Y,Z — got {v:?}"))?;
                probes.push(Probe {
                    name: name.to_string(),
                    pos: parse_xyz(coords)?,
                });
            }
            "--probes" => {
                let v = it.next().ok_or("--probes needs a file path")?;
                probes.extend(parse_probe_file(Path::new(&v))?);
            }
            "--h-tol" => {
                h_tol = it
                    .next()
                    .ok_or("--h-tol needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --h-tol: {e}"))?
            }
            "--v-tol" => {
                v_tol = it
                    .next()
                    .ok_or("--v-tol needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --v-tol: {e}"))?
            }
            "--max-components" => {
                max_components = Some(
                    it.next()
                        .ok_or("--max-components needs a value")?
                        .parse()
                        .map_err(|e| format!("bad --max-components: {e}"))?,
                )
            }
            "--gaps" => gaps = true,
            "--gap-pair" => {
                let v = it.next().ok_or("--gap-pair needs A,B")?;
                let (a, b) = v
                    .split_once(',')
                    .ok_or_else(|| format!("--gap-pair needs A,B — got {v:?}"))?;
                gap_pairs.push((
                    a.trim()
                        .parse()
                        .map_err(|e| format!("bad component id {a:?}: {e}"))?,
                    b.trim()
                        .parse()
                        .map_err(|e| format!("bad component id {b:?}: {e}"))?,
                ));
            }
            "--gap-h" => {
                gap_h = it
                    .next()
                    .ok_or("--gap-h needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --gap-h: {e}"))?
            }
            "--gap-v" => {
                gap_v = it
                    .next()
                    .ok_or("--gap-v needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --gap-v: {e}"))?
            }
            "--gap-count" => {
                gap_count = it
                    .next()
                    .ok_or("--gap-count needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --gap-count: {e}"))?
            }
            "--quiet" => quiet = true,
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if other.starts_with('-') => return Err(format!("unknown flag {other:?}")),
            other => {
                if path.is_some() {
                    return Err(format!("unexpected extra argument {other:?}"));
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    Ok(Args {
        path: path.ok_or_else(|| USAGE.to_string())?,
        probes,
        h_tol,
        v_tol,
        max_components,
        gaps,
        gap_pairs,
        gap_h,
        gap_v,
        gap_count,
        quiet,
    })
}

/// Print the closest approaches between `a` and `b`, and — when they are not
/// directly within reach — the cheapest chain of intermediate components that
/// would join them if every hop were bridged.
fn report_pair(
    out: &mut impl Write,
    graph: &NavGraph,
    gaps: &GapGraph,
    stats: &HashMap<u32, &ComponentStat>,
    a: u32,
    b: u32,
) -> io::Result<()> {
    let describe = |c: u32| match stats.get(&c) {
        Some(s) => format!("{c} ({:.0} m^2, {} polys)", s.area_xz, s.poly_count),
        None => format!("{c} (unknown)"),
    };
    writeln!(out, "\n  component {} <-> {}", describe(a), describe(b))?;
    if a >= graph.component_count || b >= graph.component_count {
        return writeln!(
            out,
            "    no such component (mesh has {})",
            graph.component_count
        );
    }

    let direct = gaps.approaches(a, b);
    if direct.is_empty() {
        writeln!(out, "    direct: nothing within the search radius")?;
    } else {
        writeln!(out, "    direct approaches, closest first:")?;
        for app in &direct {
            writeln!(
                out,
                "      h={:5.2} m  dy={:+6.2} m   ({:.1}, {:.1}, {:.1}) -> ({:.1}, {:.1}, {:.1})",
                app.horizontal,
                app.vertical,
                app.point_from[0],
                app.point_from[1],
                app.point_from[2],
                app.point_to[0],
                app.point_to[1],
                app.point_to[2],
            )?;
        }
    }

    match gaps.bottleneck_path(a, b) {
        None => writeln!(
            out,
            "    chain: no route under h<={:.2} m / v<={:.2} m at any number of hops",
            gaps.h_max, gaps.v_max
        )?,
        Some(chain) if chain.is_empty() => writeln!(out, "    chain: same component")?,
        Some(chain) => {
            let widest = chain.iter().fold(0.0f32, |m, h| m.max(h.horizontal));
            let total: f32 = chain.iter().map(|h| h.horizontal).sum();
            writeln!(
                out,
                "    chain: {} hop(s), widest {:.2} m, total {:.2} m",
                chain.len(),
                widest,
                total
            )?;
            for hop in &chain {
                writeln!(
                    out,
                    "      {:>4} -> {:<4} h={:5.2} m  dy={:+6.2} m   at ({:.1}, {:.1}, {:.1})  [{}]",
                    hop.from,
                    hop.to,
                    hop.horizontal,
                    hop.vertical,
                    hop.point_from[0],
                    hop.point_from[1],
                    hop.point_from[2],
                    describe(hop.to),
                )?;
            }
        }
    }
    Ok(())
}

/// Everything after the file has been parsed: write the report, then decide
/// the exit code.
///
/// Reporting through `out` rather than `println!`, and collecting the two
/// stderr lines into `diag` rather than printing them, is what lets the tests
/// assert on the whole behaviour without a process boundary or a captured
/// stdout.
fn run(out: &mut impl Write, nav: &XrcNav, args: &Args, diag: &mut Vec<String>) -> io::Result<u8> {
    let graph = NavGraph::from_nav(nav);
    let stats = graph.component_stats();

    writeln!(out, "file        {}", args.path.display())?;
    writeln!(
        out,
        "header      nverts={} npolys={} nvp={} cs={} ch={} border={}",
        nav.nverts, nav.npolys, nav.nvp, nav.cs, nav.ch, nav.border_size
    )?;
    writeln!(
        out,
        "agent       height={} climb={} radius={}",
        nav.agent_height, nav.agent_climb, nav.agent_radius
    )?;
    writeln!(
        out,
        "bounds      x[{:.2}, {:.2}]  y[{:.2}, {:.2}]  z[{:.2}, {:.2}]",
        nav.bmin[0], nav.bmax[0], nav.bmin[1], nav.bmax[1], nav.bmin[2], nav.bmax[2]
    )?;
    let total_area: f64 = stats.iter().map(|s| s.area_xz).sum();
    writeln!(
        out,
        "components  {} (total walkable XZ area {:.1} m^2)",
        graph.component_count, total_area
    )?;
    if !graph.asymmetric_links.is_empty() {
        writeln!(
            out,
            "WARNING     {} one-sided neighbour link(s); first: {:?}",
            graph.asymmetric_links.len(),
            &graph.asymmetric_links[..graph.asymmetric_links.len().min(4)]
        )?;
    }

    if !args.quiet {
        writeln!(out, "\n  id   polys      area_m2   bounds (x / y / z)")?;
        for s in stats.iter().take(20) {
            writeln!(
                out,
                "  {:<4} {:<7} {:>11.1}   [{:.1}, {:.1}] / [{:.1}, {:.1}] / [{:.1}, {:.1}]",
                s.id,
                s.poly_count,
                s.area_xz,
                s.bmin[0],
                s.bmax[0],
                s.bmin[1],
                s.bmax[1],
                s.bmin[2],
                s.bmax[2]
            )?;
        }
        if stats.len() > 20 {
            writeln!(out, "  ... {} more component(s)", stats.len() - 20)?;
        }
    }

    let mut failed = false;
    let mut components_seen: Vec<(String, u32)> = Vec::new();

    if !args.probes.is_empty() {
        writeln!(
            out,
            "\nprobes (h-tol {:.2} m, v-tol {:.2} m)",
            args.h_tol, args.v_tol
        )?;
        for p in &args.probes {
            // Tolerance-first: a buried sheet whose footprint covers the
            // probe must not out-rank the floor the probe is standing
            // on. See `NavGraph::locate_within`.
            match graph.locate_within(p.pos, args.h_tol, args.v_tol) {
                None => {
                    writeln!(out, "  {:<20} NO POLYGON (mesh is empty)", p.name)?;
                    failed = true;
                }
                Some(hit) => {
                    let ok = hit.horizontal_distance <= args.h_tol
                        && hit.vertical_distance.abs() <= args.v_tol;
                    writeln!(
                        out,
                        "  {:<20} poly={:<6} component={:<4} h={:.2} m  dy={:+.2} m  {}",
                        p.name,
                        hit.poly,
                        hit.component,
                        hit.horizontal_distance,
                        hit.vertical_distance,
                        if ok { "ok" } else { "OUT OF TOLERANCE" }
                    )?;
                    if ok {
                        components_seen.push((p.name.clone(), hit.component));
                    } else {
                        failed = true;
                    }
                }
            }
        }
    }

    let distinct: Vec<u32> = {
        let mut v: Vec<u32> = components_seen.iter().map(|(_, c)| *c).collect();
        v.sort_unstable();
        v.dedup();
        v
    };

    let mut wanted: Vec<(u32, u32)> = args.gap_pairs.clone();
    if args.gaps {
        for (i, a) in distinct.iter().enumerate() {
            for b in &distinct[i + 1..] {
                wanted.push((*a, *b));
            }
        }
    }
    if !wanted.is_empty() {
        // The chain search needs every component in the graph, not just the
        // ones named: finding the intermediates is the point.
        let gap_graph = graph.component_gaps(args.gap_h, args.gap_v, args.gap_count);
        writeln!(
            out,
            "\ngaps (search h<={:.2} m, v<={:.2} m, {} component pair(s) in range)",
            args.gap_h,
            args.gap_v,
            gap_graph.pair_count()
        )?;
        let by_id: HashMap<u32, &ComponentStat> = stats.iter().map(|s| (s.id, s)).collect();
        wanted.sort_unstable();
        wanted.dedup();
        for (a, b) in wanted {
            report_pair(out, &graph, &gap_graph, &by_id, a, b)?;
        }
    }

    if distinct.len() > 1 {
        diag.push(format!(
            "nav_inspect: probes span {} components: {components_seen:?}",
            distinct.len()
        ));
        return Ok(EXIT_PROBES_DISCONNECTED);
    }
    if failed {
        diag.push("nav_inspect: one or more probes were out of tolerance".to_string());
        return Ok(EXIT_PROBE_OUT_OF_TOLERANCE);
    }
    if let Some(max) = args.max_components {
        if graph.component_count > max {
            diag.push(format!(
                "nav_inspect: {} components exceeds --max-components {max}",
                graph.component_count
            ));
            return Ok(EXIT_TOO_MANY_COMPONENTS);
        }
    }
    Ok(0)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("nav_inspect: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    let bytes = match std::fs::read(&args.path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("nav_inspect: {}: {e}", args.path.display());
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let nav = match XrcNav::read(&mut std::io::Cursor::new(&bytes)) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("nav_inspect: {}: {e}", args.path.display());
            return ExitCode::from(EXIT_USAGE);
        }
    };

    let mut out = io::BufWriter::new(io::stdout().lock());
    let mut diag = Vec::new();
    let code = match run(&mut out, &nav, &args, &mut diag) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nav_inspect: write failed: {e}");
            return ExitCode::from(EXIT_USAGE);
        }
    };
    if let Err(e) = out.flush() {
        eprintln!("nav_inspect: write failed: {e}");
        return ExitCode::from(EXIT_USAGE);
    }
    for line in &diag {
        eprintln!("{line}");
    }
    ExitCode::from(code)
}

// A bin target's file IS the crate root, so a bare `mod tests;` would look
// for `src/bin/tests.rs` and collide with every other bin. Point at the
// per-binary subdirectory explicitly.
#[cfg(test)]
#[path = "nav_inspect/tests.rs"]
mod tests;
