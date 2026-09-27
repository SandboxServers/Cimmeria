//! Phase 1.4 acceptance tests: decode the BSP `Model` exports of the
//! cooked `Maps/Castle` chunks byte-exactly, and pin the emitted
//! triangle winding against NavBuilder's walkable convention.
//!
//! Every test self-skips when the cooked client tree is absent, and
//! says so loudly. A skipped test is not a pass.
//!
//! Run with `--nocapture` to see the measurement tables (per-tile
//! triangle counts, PolyFlags histogram, winding buckets).
//!
//! Companion file: `bsp_castle_floor_evidence.rs` answers "is the
//! floor here".

use crate::bsp_support::*;

use cimmeria_navmesh_extractor::bsp::collect_bsp_triangles;
use cimmeria_navmesh_extractor::bsp::{collect_bsp_models, EMIT_REVERSED};
use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::umap::enumerate_chunks;
use cimmeria_upk::Package;
use cimmeria_upk_objects::model::{deserialize_model, deserialize_polys, CollisionFilter};
use std::collections::BTreeMap;

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
        let stats = collect_bsp_triangles(&pkg, &mut soup, bsp_options(&pkg));
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

    per_tile.sort_by_key(|a| std::cmp::Reverse(a.1));
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
    eprintln!(
        "  every tile with BSP geometry, descending (`*` = on the known interior-tile list):"
    );
    for (name, tris, nodes) in per_tile.iter().filter(|t| t.1 > 0) {
        let id = name
            .trim_start_matches("Castle-")
            .trim_end_matches(".umap")
            .to_string();
        let mark = if INTERIOR_TILES.contains(&id.as_str()) {
            "*"
        } else {
            " "
        };
        eprintln!("    {mark} {name:<26} tris={tris:<7} nodes={nodes}");
    }

    // The set of tiles with BSP geometry must be exactly the set that
    // carries `ModelComponent` exports — the interior tiles. If a tile
    // drops out of one list but not the other, either the decode broke
    // or the map content changed.
    let with_geometry: std::collections::BTreeSet<String> = per_tile
        .iter()
        .filter(|t| t.1 > 0)
        .map(|t| {
            t.0.trim_start_matches("Castle-")
                .trim_end_matches(".umap")
                .to_string()
        })
        .collect();
    let expected: std::collections::BTreeSet<String> =
        INTERIOR_TILES.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        with_geometry, expected,
        "the tiles with BSP geometry must be exactly the 16 interior \
         tiles that carry ModelComponent exports"
    );

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
