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
//!     [--quiet]                    suppress the per-component table
//! ```
//!
//! Exit codes: `0` ok, `1` usage/IO error, `2` a probe was out of tolerance,
//! `3` the probes did not all land in the same component, `4` the mesh
//! exceeded `--max-components`.

use std::path::PathBuf;
use std::process::ExitCode;

use cimmeria_navmesh_extractor::nav_components::NavGraph;
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
    quiet: bool,
}

const USAGE: &str = "usage: nav_inspect <file.nav> [--probe NAME=X,Y,Z]... [--probes FILE] \
                     [--h-tol M] [--v-tol M] [--max-components N] [--quiet]";

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
fn parse_probe_file(path: &PathBuf) -> Result<Vec<Probe>, String> {
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
    let mut it = std::env::args().skip(1);
    let mut path: Option<PathBuf> = None;
    let mut probes = Vec::new();
    let mut h_tol = 2.0f32;
    let mut v_tol = 3.0f32;
    let mut max_components = None;
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
                probes.extend(parse_probe_file(&PathBuf::from(v))?);
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
        quiet,
    })
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

    let graph = NavGraph::from_nav(&nav);
    let stats = graph.component_stats();

    println!("file        {}", args.path.display());
    println!(
        "header      nverts={} npolys={} nvp={} cs={} ch={} border={}",
        nav.nverts, nav.npolys, nav.nvp, nav.cs, nav.ch, nav.border_size
    );
    println!(
        "agent       height={} climb={} radius={}",
        nav.agent_height, nav.agent_climb, nav.agent_radius
    );
    println!(
        "bounds      x[{:.2}, {:.2}]  y[{:.2}, {:.2}]  z[{:.2}, {:.2}]",
        nav.bmin[0], nav.bmax[0], nav.bmin[1], nav.bmax[1], nav.bmin[2], nav.bmax[2]
    );
    let total_area: f64 = stats.iter().map(|s| s.area_xz).sum();
    println!(
        "components  {} (total walkable XZ area {:.1} m^2)",
        graph.component_count, total_area
    );
    if !graph.asymmetric_links.is_empty() {
        println!(
            "WARNING     {} one-sided neighbour link(s); first: {:?}",
            graph.asymmetric_links.len(),
            &graph.asymmetric_links[..graph.asymmetric_links.len().min(4)]
        );
    }

    if !args.quiet {
        println!("\n  id   polys      area_m2   bounds (x / y / z)");
        for s in stats.iter().take(20) {
            println!(
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
            );
        }
        if stats.len() > 20 {
            println!("  ... {} more component(s)", stats.len() - 20);
        }
    }

    let mut failed = false;
    let mut components_seen: Vec<(String, u32)> = Vec::new();

    if !args.probes.is_empty() {
        println!(
            "\nprobes (h-tol {:.2} m, v-tol {:.2} m)",
            args.h_tol, args.v_tol
        );
        for p in &args.probes {
            // Tolerance-first: a buried sheet whose footprint covers the
            // probe must not out-rank the floor the probe is standing
            // on. See `NavGraph::locate_within`.
            match graph.locate_within(p.pos, args.h_tol, args.v_tol) {
                None => {
                    println!("  {:<20} NO POLYGON (mesh is empty)", p.name);
                    failed = true;
                }
                Some(hit) => {
                    let ok = hit.horizontal_distance <= args.h_tol
                        && hit.vertical_distance.abs() <= args.v_tol;
                    println!(
                        "  {:<20} poly={:<6} component={:<4} h={:.2} m  dy={:+.2} m  {}",
                        p.name,
                        hit.poly,
                        hit.component,
                        hit.horizontal_distance,
                        hit.vertical_distance,
                        if ok { "ok" } else { "OUT OF TOLERANCE" }
                    );
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
    if distinct.len() > 1 {
        eprintln!(
            "nav_inspect: probes span {} components: {:?}",
            distinct.len(),
            components_seen
        );
        return ExitCode::from(EXIT_PROBES_DISCONNECTED);
    }
    if failed {
        eprintln!("nav_inspect: one or more probes were out of tolerance");
        return ExitCode::from(EXIT_PROBE_OUT_OF_TOLERANCE);
    }
    if let Some(max) = args.max_components {
        if graph.component_count > max {
            eprintln!(
                "nav_inspect: {} components exceeds --max-components {}",
                graph.component_count, max
            );
            return ExitCode::from(EXIT_TOO_MANY_COMPONENTS);
        }
    }
    ExitCode::SUCCESS
}
