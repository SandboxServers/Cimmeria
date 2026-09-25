//! The explorable area of a world, for trimming the occluder (NA27).
//!
//! The big exteriors carry kilometres of terrain nobody can reach. The
//! occluder only needs to answer for places an entity can stand, so its
//! coverage is the navmesh components that hold a real entry point, plus a
//! margin. An entry point is:
//!
//! - a seeded arrival or spawn: spawnlist rows, respawners, stargates and
//!   their arrival points, ring transport regions, and content-chain
//!   teleport and `move_waypoint` targets (`tools/occluder_entry_points.py`
//!   writes them from `db/resources/`);
//! - an arrival authored in the client map: `PlayerStart`, `SGWStargate`
//!   and `SGWTeleporter` actors ([`map_entry_actors`]).
//!
//! A point counts for the component whose polygon lies within
//! [`ENTRY_H_TOL`] horizontally and [`ENTRY_V_TOL`] vertically of it.

use std::collections::HashSet;
use std::path::Path;

use super::{ue3_to_bw, BwTriangle};
use crate::nav_components::NavGraph;
use crate::umap;

/// How far off a polygon, horizontally, an entry point may be and still
/// claim its component. A spawn marker sits on or just beside the floor.
pub const ENTRY_H_TOL: f32 = 3.0;
/// The same, vertically. A PlayerStart floats up to a metre or two above.
pub const ENTRY_V_TOL: f32 = 4.0;

/// Actor classes whose `Location` is where something arrives in the world.
const ENTRY_CLASSES: [&str; 3] = ["PlayerStart", "SGWStargate", "SGWTeleporter"];

/// One entry point.
#[derive(Debug, Clone, PartialEq)]
pub struct EntryPoint {
    pub source: String,
    pub pos: [f32; 3],
}

/// Read `tools/occluder_entry_points.py` output for `world_key` (the
/// `.nav` basename). Rows for `*` are included too.
pub fn read_entry_points(tsv: &Path, world_key: &str) -> crate::Result<Vec<EntryPoint>> {
    let text = std::fs::read_to_string(tsv)?;
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 5 || (f[0] != world_key && f[0] != "*") {
            continue;
        }
        let parse = |s: &str| s.trim().parse::<f32>().ok();
        if let (Some(x), Some(y), Some(z)) = (parse(f[2]), parse(f[3]), parse(f[4])) {
            out.push(EntryPoint {
                source: f[1].to_string(),
                pos: [x, y, z],
            });
        }
    }
    Ok(out)
}

/// The map's own arrival actors, in BigWorld metres.
pub fn map_entry_actors(map_dir: &Path) -> crate::Result<Vec<EntryPoint>> {
    let mut out = Vec::new();
    // The persistent level (`<Map>.umap`) holds the PlayerStart; the
    // chunks hold the gates and teleporters.
    let mut packages = umap::enumerate_chunks(map_dir)?;
    if let Some(name) = map_dir.file_name().and_then(|n| n.to_str()) {
        let persistent = map_dir.join(format!("{name}.umap"));
        if persistent.exists() {
            packages.push(persistent);
        }
    }
    for chunk in packages {
        let pkg = cimmeria_upk::Package::open(&chunk)?;
        for a in cimmeria_upk::extract_actors(&pkg) {
            if !ENTRY_CLASSES.contains(&a.class_name.as_str()) || a.location == [0.0; 3] {
                continue;
            }
            out.push(EntryPoint {
                source: format!("map:{}:{}", a.class_name, a.object_name),
                pos: ue3_to_bw(a.location),
            });
        }
    }
    Ok(out)
}

/// Which components hold an entry point, and which points found none.
#[derive(Debug, Default)]
pub struct Explorable {
    pub components: HashSet<u32>,
    pub located: usize,
    pub unlocated: Vec<EntryPoint>,
}

/// Resolve every entry point to a component of `graph`.
pub fn explorable_components(graph: &NavGraph, points: &[EntryPoint]) -> Explorable {
    let mut out = Explorable::default();
    for p in points {
        match graph.locate_within(p.pos, ENTRY_H_TOL, ENTRY_V_TOL) {
            Some(hit)
                if hit.horizontal_distance <= ENTRY_H_TOL
                    && hit.vertical_distance.abs() <= ENTRY_V_TOL =>
            {
                out.components.insert(hit.component);
                out.located += 1;
            }
            _ => out.unlocated.push(p.clone()),
        }
    }
    out
}

/// How close, horizontally, another component's polygon must come to a
/// kept one to be kept too. A navmesh splits at a door, a stair or a ledge
/// the agent cannot step: a player crosses such a gap, the mesh does not.
pub const LINK_H: f32 = 5.0;
/// The same, vertically: one storey of stairs, not the roof above.
pub const LINK_V: f32 = 3.0;

/// Grow `kept` with every component that comes within [`LINK_H`] /
/// [`LINK_V`] of it, transitively. Returns how many were added.
pub fn grow_components(graph: &NavGraph, kept: &mut HashSet<u32>) -> usize {
    // Per polygon: its component and its bounding box.
    let boxes: Vec<(u32, [f32; 3], [f32; 3])> = graph
        .polys
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut lo = [f32::INFINITY; 3];
            let mut hi = [f32::NEG_INFINITY; 3];
            for &v in &p.verts {
                let v = graph.verts[v as usize];
                for k in 0..3 {
                    lo[k] = lo[k].min(v[k]);
                    hi[k] = hi[k].max(v[k]);
                }
            }
            (graph.component[i], lo, hi)
        })
        .collect();
    let bucket = |x: f32| (x / (4.0 * LINK_H)).floor() as i64;
    let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = Default::default();
    for (i, (_, lo, hi)) in boxes.iter().enumerate() {
        for bz in bucket(lo[2] - LINK_H)..=bucket(hi[2] + LINK_H) {
            for bx in bucket(lo[0] - LINK_H)..=bucket(hi[0] + LINK_H) {
                grid.entry((bx, bz)).or_default().push(i);
            }
        }
    }
    let near = |a: usize, b: usize| {
        let (_, alo, ahi) = boxes[a];
        let (_, blo, bhi) = boxes[b];
        let gap = |k: usize| (blo[k] - ahi[k]).max(alo[k] - bhi[k]).max(0.0);
        gap(0).hypot(gap(2)) <= LINK_H && gap(1) <= LINK_V
    };
    let before = kept.len();
    loop {
        let mut added = Vec::new();
        for (i, (c, lo, hi)) in boxes.iter().enumerate() {
            if !kept.contains(c) {
                continue;
            }
            for bz in bucket(lo[2])..=bucket(hi[2]) {
                for bx in bucket(lo[0])..=bucket(hi[0]) {
                    for &j in grid.get(&(bx, bz)).into_iter().flatten() {
                        let cj = boxes[j].0;
                        if !kept.contains(&cj) && !added.contains(&cj) && near(i, j) {
                            added.push(cj);
                        }
                    }
                }
            }
        }
        if added.is_empty() {
            return kept.len() - before;
        }
        kept.extend(added);
    }
}

/// The polygons of `components`, fan-triangulated: the coverage the
/// occluder builder is given.
pub fn component_triangles(graph: &NavGraph, components: &HashSet<u32>) -> Vec<BwTriangle> {
    let mut out = Vec::new();
    for (i, poly) in graph.polys.iter().enumerate() {
        if !components.contains(&graph.component[i]) {
            continue;
        }
        let v: Vec<[f32; 3]> = poly
            .verts
            .iter()
            .map(|&k| graph.verts[k as usize])
            .collect();
        for k in 1..v.len().saturating_sub(1) {
            out.push([v[0], v[k], v[k + 1]]);
        }
    }
    out
}
