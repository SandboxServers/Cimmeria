//! Phase 1.4 acceptance tests: decode the BSP `Model` exports of the
//! cooked `Maps/Castle` chunks and confirm the geometry lands where the
//! live playtest says the floors are.
//!
//! Every test self-skips when the cooked client tree is absent, and
//! says so loudly. A skipped test is not a pass.
//!
//! Run with `--nocapture` to see the measurement tables (per-tile
//! triangle counts, PolyFlags histogram, winding buckets, floor probe).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cimmeria_navmesh_extractor::bsp::{collect_bsp_models, collect_bsp_triangles, EMIT_REVERSED};
use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::umap::enumerate_chunks;
use cimmeria_upk::Package;
use cimmeria_upk_objects::model::{deserialize_model, deserialize_polys, CollisionFilter};

/// The tile the playtest's two HIGH-confidence walkable points sit in.
const INTERIOR_TILE: &str = "Castle-000a0002.umap";
/// The tile holding the Level-5 comms room point.
const COMMS_TILE: &str = "Castle-00080002.umap";

// --- Measured baselines (Castle-000a0002.umap, persistent-level Model).
//
// These are the numbers this branch measured, not numbers copied from
// the RE finding. A change to the deserializer that moves any of them
// is a behaviour change that needs re-measuring, not a test to relax.

/// `Nodes.Num()` of the persistent-level `Model`.
const TILE_A2_LEVEL_NODES: usize = 399;
/// Fan triangles from those nodes, before any PolyFlags filtering.
const TILE_A2_LEVEL_TRIS_UNFILTERED: usize = 1098;

/// Locate `CookedPC/Maps/Castle` by walking up from the crate manifest
/// until a `sgw/Stargate Worlds-QA/...` sibling appears. Matches the
/// probe `staticmesh_castle_cellblock.rs` uses so both tests behave the
/// same from a worktree.
fn castle_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suffix = PathBuf::from("sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/Castle");
    for ancestor in manifest.ancestors().take(10) {
        let candidate = ancestor.join(&suffix);
        if candidate.exists() {
            return candidate;
        }
    }
    manifest.join(&suffix)
}

fn skip_if_missing(dir: &Path, what: &str) -> bool {
    if !dir.exists() {
        eprintln!(
            "SKIPPED {what} — cooked client tree not present at {}",
            dir.display()
        );
        return true;
    }
    false
}

/// BigWorld → UE3 cm. `BW = (ue.y/100, ue.z/100, ue.x/100)`, so the
/// inverse is `ue = (bw.z*100, bw.x*100, bw.y*100)`.
fn bw_to_ue(bw: [f32; 3]) -> [f32; 3] {
    [bw[2] * 100.0, bw[0] * 100.0, bw[1] * 100.0]
}

/// UE3 right-hand-rule normal of a triangle in its emitted order.
fn winding_normal(t: [[f32; 3]; 3]) -> [f32; 3] {
    let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

fn norm3(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// 2D point-in-triangle over the UE3 XY plane (the BW ground plane).
fn contains_xy(t: [[f32; 3]; 3], x: f32, y: f32) -> bool {
    let sign = |ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32| {
        (ax - cx) * (by - cy) - (bx - cx) * (ay - cy)
    };
    let d1 = sign(x, y, t[0][0], t[0][1], t[1][0], t[1][1]);
    let d2 = sign(x, y, t[1][0], t[1][1], t[2][0], t[2][1]);
    let d3 = sign(x, y, t[2][0], t[2][1], t[0][0], t[0][1]);
    let has_neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let has_pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(has_neg && has_pos)
}

/// Barycentric interpolation of a triangle's Z at (x, y).
fn interp_z(t: [[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    let d = (t[1][1] - t[2][1]) * (t[0][0] - t[2][0]) + (t[2][0] - t[1][0]) * (t[0][1] - t[2][1]);
    if d.abs() < 1e-6 {
        return None;
    }
    let a = ((t[1][1] - t[2][1]) * (x - t[2][0]) + (t[2][0] - t[1][0]) * (y - t[2][1])) / d;
    let b = ((t[2][1] - t[0][1]) * (x - t[2][0]) + (t[0][0] - t[2][0]) * (y - t[2][1])) / d;
    let c = 1.0 - a - b;
    Some(a * t[0][2] + b * t[1][2] + c * t[2][2])
}

/// One world-space triangle plus the authored normal of the surface it
/// came from. The floor probe must not use the winding-derived normal
/// (that's the question the winding test answers), so both travel
/// together.
struct WorldTri {
    tri: [[f32; 3]; 3],
    /// Authored surface normal (`Vectors[vNormal]`), world space.
    surf_normal: [f32; 3],
    poly_flags: u32,
}

/// Decode a chunk and return every emitted BSP triangle with its
/// authored surface normal, using the production filter.
fn world_triangles(chunk: &Path) -> Vec<WorldTri> {
    let pkg = Package::open(chunk).expect("open chunk");
    let (instances, _stats) = collect_bsp_models(&pkg);
    let mut out = Vec::new();
    for inst in &instances {
        let t = inst.model.triangulate(CollisionFilter::default());
        for (i, tri) in t.triangles.iter().enumerate() {
            let surf_index = t.triangle_surf[i] as usize;
            let n_local = inst
                .model
                .surf_normal(surf_index)
                .unwrap_or([0.0, 0.0, 0.0]);
            // A level model is world space already; a brush model's
            // normal would need the rotation applied. Castle has zero
            // non-empty brush models, so rotate only when it matters.
            let n_world = if inst.is_level_model {
                n_local
            } else {
                let o = inst.transform.apply([0.0; 3]);
                let p = inst.transform.apply(n_local);
                [p[0] - o[0], p[1] - o[1], p[2] - o[2]]
            };
            let mut world = [
                inst.to_world(tri[0]),
                inst.to_world(tri[1]),
                inst.to_world(tri[2]),
            ];
            if EMIT_REVERSED {
                world.swap(1, 2);
            }
            out.push(WorldTri {
                tri: world,
                surf_normal: n_world,
                poly_flags: inst.model.surfs[surf_index].poly_flags,
            });
        }
    }
    out
}

#[test]
fn castle_interior_tile_models_parse_with_zero_remainder() {
    let dir = castle_dir();
    if skip_if_missing(
        &dir,
        "castle_interior_tile_models_parse_with_zero_remainder",
    ) {
        return;
    }
    let chunk = dir.join(INTERIOR_TILE);
    assert!(
        chunk.exists(),
        "{INTERIOR_TILE} missing from the asset tree"
    );
    let pkg = Package::open(&chunk).expect("open chunk");

    // EVERY Model export, including the ones the extractor excludes
    // (trigger volumes) and the 108-byte stubs — the exactness contract
    // has to hold for all of them or the field layout is wrong.
    let mut models = 0usize;
    let mut failures = Vec::new();
    let mut biggest: Option<(usize, usize)> = None; // (export index, nodes)
    for (idx, export) in pkg.exports.iter().enumerate() {
        if pkg.export_class_name(export) != "Model" {
            continue;
        }
        models += 1;
        let data = pkg.read_export_data(export).expect("read export");
        match deserialize_model(&data, &pkg.names) {
            Ok(m) => {
                if biggest.map(|b| m.nodes.len() > b.1).unwrap_or(true) {
                    biggest = Some((idx + 1, m.nodes.len()));
                }
            }
            Err(e) => failures.push(format!("Model#{} ({}B): {e}", idx + 1, data.len())),
        }
    }
    eprintln!(
        "{INTERIOR_TILE}: {models} Model exports, {} failures",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "every Model export must consume its serial data exactly:\n{}",
        failures.join("\n")
    );
    assert!(models >= 60, "expected ~69 Model exports, found {models}");

    // Same contract for Polys — the cheap cross-check.
    let mut polys = 0usize;
    let mut poly_failures = Vec::new();
    let mut poly_tris = 0usize;
    for (idx, export) in pkg.exports.iter().enumerate() {
        if pkg.export_class_name(export) != "Polys" {
            continue;
        }
        polys += 1;
        let data = pkg.read_export_data(export).expect("read export");
        match deserialize_polys(&data, &pkg.names) {
            Ok(p) => poly_tris += p.triangle_count(),
            Err(e) => poly_failures.push(format!("Polys#{} ({}B): {e}", idx + 1, data.len())),
        }
    }
    eprintln!(
        "{INTERIOR_TILE}: {polys} Polys exports, {} failures, {poly_tris} fan triangles total",
        poly_failures.len()
    );
    assert!(
        poly_failures.is_empty(),
        "every Polys export must consume its serial data exactly:\n{}",
        poly_failures.join("\n")
    );

    let (level_export, level_nodes) = biggest.expect("at least one Model");
    eprintln!("{INTERIOR_TILE}: largest Model is export #{level_export} with {level_nodes} nodes");
    assert_eq!(
        level_nodes, TILE_A2_LEVEL_NODES,
        "persistent-level Model node count changed"
    );
}

#[test]
fn castle_interior_tile_level_model_triangle_count() {
    let dir = castle_dir();
    if skip_if_missing(&dir, "castle_interior_tile_level_model_triangle_count") {
        return;
    }
    let chunk = dir.join(INTERIOR_TILE);
    let pkg = Package::open(&chunk).expect("open chunk");
    let (instances, stats) = collect_bsp_models(&pkg);

    eprintln!(
        "{INTERIOR_TILE}: models total={} parsed={} failed={} empty={} \
         level={} actor_included={} actor_excluded={} builder={}",
        stats.models_total,
        stats.models_parsed,
        stats.models_failed,
        stats.models_empty,
        stats.level_models,
        stats.actor_models_included,
        stats.actor_models_excluded,
        stats.builder_brush_models,
    );
    eprintln!(
        "{INTERIOR_TILE}: unclassified owner classes = {:?}",
        stats.unclassified_owner_classes
    );
    assert_eq!(stats.models_failed, 0);
    assert!(
        stats.unclassified_volume_classes.is_empty(),
        "an unrecognised *Volume class was excluded by the fallback rule: {:?}",
        stats.unclassified_volume_classes
    );

    let level: Vec<_> = instances.iter().filter(|i| i.is_level_model).collect();
    assert_eq!(
        level.len(),
        1,
        "exactly one non-empty persistent-level Model expected"
    );
    let model = &level[0].model;
    assert_eq!(model.nodes.len(), TILE_A2_LEVEL_NODES);

    let unfiltered = model.triangulate(CollisionFilter::KEEP_ALL);
    let filtered = model.triangulate(CollisionFilter::default());
    eprintln!(
        "{INTERIOR_TILE} level Model: points={} verts={} surfs={} nodes={} \
         (no-face={} out-of-range={} degenerate={})",
        model.points.len(),
        model.verts.len(),
        model.surfs.len(),
        unfiltered.nodes_total,
        unfiltered.nodes_without_vertices,
        unfiltered.nodes_out_of_range,
        unfiltered.nodes_degenerate,
    );
    eprintln!(
        "  triangles: unfiltered={} filtered={} (excluded={})",
        unfiltered.triangles.len(),
        filtered.triangles.len(),
        filtered.triangles_excluded
    );
    eprintln!("  PolyFlags histogram (value -> face-carrying nodes):");
    for (value, count) in &unfiltered.poly_flag_histogram {
        eprintln!("    {value:#010x}  {count}");
    }
    eprintln!("  NodeFlags histogram (value -> face-carrying nodes):");
    for (value, count) in &unfiltered.node_flag_histogram {
        eprintln!("    {value:#04x}  {count}");
    }
    eprintln!("  per-flag triangle exclusion counts (independent of the active filter):");
    for (name, bit, count) in &unfiltered.excluded_by_flag {
        eprintln!("    {name:<14} {bit:#010x}  {count}");
    }

    assert_eq!(
        unfiltered.nodes_out_of_range, 0,
        "a node index fell outside its array — FBspNode offsets are wrong"
    );
    assert_eq!(
        unfiltered.triangles.len(),
        TILE_A2_LEVEL_TRIS_UNFILTERED,
        "unfiltered fan triangle count changed"
    );
}

#[test]
fn bsp_floor_winding_matches_navbuilder_walkable_convention() {
    // NavBuilder calls a triangle walkable when the UE3 right-hand-rule
    // normal of the EMITTED order points down (`n_ue3.z < 0`), because
    // the OBJ column swap turns that into Recast's +Y up. BSP fans are
    // synthesised from node vertex pools, so the orientation has to be
    // measured on real data rather than assumed.
    let dir = castle_dir();
    if skip_if_missing(
        &dir,
        "bsp_floor_winding_matches_navbuilder_walkable_convention",
    ) {
        return;
    }
    let tris = world_triangles(&dir.join(INTERIOR_TILE));
    assert!(!tris.is_empty());

    // Bucket near-horizontal triangles by winding-normal Z sign and by
    // BW height (= ue.z / 100), rounded to the nearest metre.
    let mut buckets: BTreeMap<(i64, &'static str), (usize, f64)> = BTreeMap::new();
    let mut agree = 0usize;
    let mut disagree = 0usize;
    for wt in &tris {
        let n = winding_normal(wt.tri);
        let len = norm3(n);
        if len < 1e-6 {
            continue;
        }
        let nz = n[2] / len;
        if nz.abs() < 0.86 {
            continue; // not near-horizontal (>~30 deg from flat)
        }
        let area = 0.5 * len as f64 / 10_000.0; // cm^2 -> m^2
        let bw_y = (wt.tri[0][2] + wt.tri[1][2] + wt.tri[2][2]) / 3.0 / 100.0;
        let key = (
            bw_y.round() as i64,
            if nz < 0.0 { "n.z<0" } else { "n.z>0" },
        );
        let e = buckets.entry(key).or_insert((0, 0.0));
        e.0 += 1;
        e.1 += area;

        // Does the emitted winding agree with the authored surface
        // normal, or is it inverted?
        let sn = wt.surf_normal;
        let dot = n[0] * sn[0] + n[1] * sn[1] + n[2] * sn[2];
        if dot > 0.0 {
            agree += 1;
        } else if dot < 0.0 {
            disagree += 1;
        }
    }

    eprintln!(
        "{INTERIOR_TILE} near-horizontal BSP triangle buckets (EMIT_REVERSED={EMIT_REVERSED}):"
    );
    eprintln!(
        "  {:>8}  {:>7}  {:>6}  {:>12}",
        "BW y (m)", "sign", "tris", "area m^2"
    );
    for ((h, sign), (count, area)) in &buckets {
        eprintln!("  {h:>8}  {sign:>7}  {count:>6}  {area:>12.1}");
    }
    eprintln!("  emitted winding vs authored surf normal: agree={agree} disagree={disagree}");

    // The playtest's two HIGH-confidence walkable points in this tile
    // are at BW y = 66.79, so the floor slab rounds to 67 m.
    let up: usize = buckets
        .iter()
        .filter(|((h, s), _)| (66..=68).contains(h) && *s == "n.z<0")
        .map(|(_, v)| v.0)
        .sum();
    let down: usize = buckets
        .iter()
        .filter(|((h, s), _)| (66..=68).contains(h) && *s == "n.z>0")
        .map(|(_, v)| v.0)
        .sum();
    eprintln!("  at BW y 66..68: n.z<0 -> {up} tris, n.z>0 -> {down} tris");
    assert!(
        up + down > 0,
        "no near-horizontal BSP geometry at the known walkable height — \
         either the decode or the BW/UE3 mapping is wrong"
    );
    assert!(
        up > down,
        "floor triangles at the known walkable height emit with n_ue3.z > 0 \
         ({down} vs {up}); NavBuilder would treat them as ceilings. Flip \
         `bsp::EMIT_REVERSED` and re-measure."
    );
    // The reversal only makes sense as a *correction*: the raw node
    // order agrees with the authored surface normal, and NavBuilder
    // wants the UE3 render convention (right-hand-rule normal =
    // -surface normal). Pinning the post-reversal relationship shows a
    // future reader that `EMIT_REVERSED` isn't arbitrary — if someone
    // flips it, this assertion is what fails.
    assert!(
        disagree > agree,
        "after reversal the emitted winding must OPPOSE the authored \
         surface normal (agree={agree} disagree={disagree}); if it agrees, \
         either EMIT_REVERSED or the node vertex order changed"
    );
}

#[test]
fn bsp_floor_probe_at_known_walkable_points() {
    // Task 5 — the question the spike turns on: does the decoded BSP
    // put an upward-facing surface under each known-walkable point?
    //
    // "Upward-facing" is judged by the AUTHORED surface normal
    // (`Vectors[vNormal]`), not by emitted winding, so the answer is
    // independent of the winding question above. BW up is +ue.z.
    let dir = castle_dir();
    if skip_if_missing(&dir, "bsp_floor_probe_at_known_walkable_points") {
        return;
    }

    let probes: [(&str, &str, [f32; 3]); 3] = [
        ("Zuritska cell", INTERIOR_TILE, [268.0, 66.79, 1042.59]),
        ("Romney corridor end", INTERIOR_TILE, [244.0, 66.79, 1036.0]),
        ("Level-5 comms room", COMMS_TILE, [271.7, 55.2, 858.0]),
    ];

    // ~1.5 BW units either side of the recorded Y, in cm.
    //
    // The brief asks for a face "under the point within ~1.5 units
    // below its Y", but the window is symmetric on purpose: the
    // recorded telemetry Y sits slightly *below* the floor plane at
    // two of the three points (-9 cm and -25 cm; the third is exact),
    // so a strictly-below window reports a false absence for a floor
    // that is plainly there. The signed offset is printed for every
    // hit so the sidedness stays visible instead of being hidden by
    // the tolerance.
    const BELOW_CM: f32 = 150.0;
    const ABOVE_CM: f32 = 150.0;

    let mut cache: BTreeMap<String, Vec<WorldTri>> = BTreeMap::new();
    let mut results = Vec::new();
    for (label, tile, bw) in probes {
        let chunk = dir.join(tile);
        if !chunk.exists() {
            eprintln!("SKIPPED probe {label} — {tile} missing");
            continue;
        }
        let tris = cache
            .entry(tile.to_string())
            .or_insert_with(|| world_triangles(&chunk));
        let ue = bw_to_ue(bw);

        let mut best: Option<(f32, f32, u32)> = None; // (drop cm, nz, flags)
        let mut in_window = 0usize;
        let mut over_xy = 0usize;
        // Nearest upward-facing face at this XY at ANY height, so an
        // "absent" answer can distinguish "no geometry here at all"
        // from "floor is at a different height".
        let mut nearest_up: Option<(f32, f32)> = None; // (drop cm, bw y)
        for wt in tris.iter() {
            if !contains_xy(wt.tri, ue[0], ue[1]) {
                continue;
            }
            over_xy += 1;
            let Some(z) = interp_z(wt.tri, ue[0], ue[1]) else {
                continue;
            };
            let drop = ue[2] - z;
            let n = wt.surf_normal;
            let len = norm3(n);
            if len < 1e-6 {
                continue;
            }
            let nz = n[2] / len;
            if nz > 0.0 && nearest_up.map(|b| drop.abs() < b.0.abs()).unwrap_or(true) {
                nearest_up = Some((drop, z / 100.0));
            }
            if !(-ABOVE_CM..=BELOW_CM).contains(&drop) {
                continue;
            }
            in_window += 1;
            if nz <= 0.0 {
                continue; // not upward-facing in BW
            }
            if best.map(|b| drop.abs() < b.0.abs()).unwrap_or(true) {
                best = Some((drop, nz, wt.poly_flags));
            }
        }

        match best {
            Some((drop, nz, flags)) => {
                eprintln!(
                    "FLOOR PRESENT  {label} ({tile}) bw={bw:?} ue=({:.0},{:.0},{:.0}) \
                     -> upward BSP face {:.1} cm below, surf n.z={nz:.3}, PolyFlags={flags:#x}",
                    ue[0], ue[1], ue[2], drop
                );
                results.push((label, true));
            }
            None => {
                let nearest = match nearest_up {
                    Some((drop, bw_y)) => format!(
                        "nearest upward BSP face at this XY is {drop:.0} cm away (BW y {bw_y:.2})"
                    ),
                    None => "no upward-facing BSP face at this XY at ANY height".to_string(),
                };
                eprintln!(
                    "FLOOR ABSENT   {label} ({tile}) bw={bw:?} ue=({:.0},{:.0},{:.0}) \
                     -> no upward-facing BSP face within {BELOW_CM} cm below \
                     ({in_window} candidate(s) in the height window, {over_xy} BSP \
                     triangle(s) span this XY); {nearest}",
                    ue[0], ue[1], ue[2]
                );
                results.push((label, false));
            }
        }
    }

    assert_eq!(results.len(), 3, "all three probe tiles must be present");
    // This test reports rather than gates: an absent BSP floor is a
    // real finding (the floor may be a StaticMesh or Terrain), not a
    // decoder failure. What WOULD be a decoder failure is producing no
    // geometry at all, which the other tests cover.
}

#[test]
fn castle_all_chunks_bsp_scan() {
    // Full-map sweep: every chunk must decode with zero failures, and
    // the per-tile triangle table is the deliverable.
    let dir = castle_dir();
    if skip_if_missing(&dir, "castle_all_chunks_bsp_scan") {
        return;
    }
    let chunks = enumerate_chunks(&dir).expect("enumerate_chunks");
    assert!(
        chunks.len() >= 140,
        "expected ~144 Castle chunks, found {}",
        chunks.len()
    );

    let mut per_tile: Vec<(String, usize, usize)> = Vec::new(); // name, tris, nodes
    let mut total_tris = 0usize;
    let mut total_models = 0usize;
    let mut total_failed = 0usize;
    let mut total_excluded_volumes = 0usize;
    let mut total_flag_excluded = 0usize;
    let mut all_errors: Vec<String> = Vec::new();
    let mut unclassified: BTreeMap<String, usize> = BTreeMap::new();
    let mut flag_hist: BTreeMap<u32, usize> = BTreeMap::new();

    for chunk in &chunks {
        let pkg = match Package::open(chunk) {
            Ok(p) => p,
            Err(e) => {
                all_errors.push(format!("{}: open: {e}", chunk.display()));
                continue;
            }
        };
        let mut soup = TriangleSoup::new(None);
        let stats = collect_bsp_triangles(&pkg, &mut soup);
        total_models += stats.models_total;
        total_failed += stats.models_failed;
        total_excluded_volumes += stats.actor_models_excluded;
        total_flag_excluded += stats.triangles_excluded;
        total_tris += stats.triangles_emitted;
        all_errors.extend(stats.parse_errors.iter().cloned());
        for (c, n) in &stats.unclassified_owner_classes {
            *unclassified.entry(c.clone()).or_default() += n;
        }
        for (v, n) in &stats.poly_flag_histogram {
            *flag_hist.entry(*v).or_default() += n;
        }
        assert_eq!(
            stats.nodes_out_of_range,
            0,
            "{}: node index out of range",
            chunk.display()
        );
        let name = chunk
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        per_tile.push((name, stats.triangles_emitted, stats.nodes_total));
    }

    per_tile.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("Castle BSP sweep: {} chunks", chunks.len());
    eprintln!(
        "  Model exports={total_models} decode failures={total_failed} \
         volume models excluded={total_excluded_volumes}"
    );
    eprintln!("  triangles emitted={total_tris} (filter removed {total_flag_excluded})");
    eprintln!(
        "  chunks with BSP geometry: {}",
        per_tile.iter().filter(|t| t.1 > 0).count()
    );
    eprintln!("  unclassified owner classes: {unclassified:?}");
    eprintln!("  PolyFlags histogram across the map (value -> face-carrying nodes):");
    for (v, n) in &flag_hist {
        eprintln!("    {v:#010x}  {n}");
    }
    eprintln!("  top 10 tiles by BSP triangle count:");
    for (name, tris, nodes) in per_tile.iter().take(10) {
        eprintln!("    {name:<26} tris={tris:<7} nodes={nodes}");
    }

    assert!(
        all_errors.is_empty(),
        "BSP decode failures across the Castle map:\n{}",
        all_errors.join("\n")
    );
    assert!(
        total_tris > 0,
        "the whole Castle map produced no BSP triangles"
    );
}
