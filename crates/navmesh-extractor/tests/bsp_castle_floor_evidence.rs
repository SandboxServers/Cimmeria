//! Phase 1.4 floor evidence: does the decoded BSP actually put a
//! walkable surface where the live playtest says players stood, and
//! where the shipped `.nav` has large flat sheets?
//!
//! Also audits the two structures a BSP consumer might be expected to
//! need and does not: the persistent (master) `<MapName>.umap`
//! package, and the `ModelComponent` exports.
//!
//! Every test self-skips when the cooked client tree is absent, and
//! says so loudly. A skipped test is not a pass. Run with
//! `--nocapture` for the probe tables.
//!
//! Companion file: `bsp_castle_model_decode.rs` covers the decode
//! itself.

mod bsp_support;

use bsp_support::*;

use cimmeria_navmesh_extractor::bsp::{collect_bsp_models, collect_bsp_triangles};
use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::umap::enumerate_chunks;
use cimmeria_upk::Package;
use cimmeria_upk_objects::model::{deserialize_model, deserialize_polys, CollisionFilter};
use std::collections::BTreeMap;

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

// ---------------------------------------------------------------------
// Persistent (master) map package — the `<MapName>.umap` that sits
// alongside the chunks and that `enumerate_chunks` deliberately skips.
// ---------------------------------------------------------------------

#[test]
fn enumerate_chunks_skips_the_persistent_map_package() {
    // Pinned because the coordinator needs to know: anything that only
    // walks `enumerate_chunks` never opens `<MapName>.umap`. This is by
    // design in `umap.rs` (the master file has no `-<HEX8>` suffix), so
    // reaching the persistent level means feeding that path in
    // separately — `collect_bsp_triangles` takes a plain `&Package` and
    // makes no chunk assumptions, so it works unchanged.
    let dir = castle_dir();
    if skip_if_missing(&dir, "enumerate_chunks_skips_the_persistent_map_package") {
        return;
    }
    let master = dir.join("Castle.umap");
    assert!(master.exists(), "Castle.umap must exist next to the chunks");
    let chunks = enumerate_chunks(&dir).expect("enumerate_chunks");
    assert!(
        !chunks.iter().any(|c| c == &master),
        "enumerate_chunks must not return the persistent map package"
    );
}

#[test]
fn persistent_map_packages_carry_no_bsp_world_geometry() {
    // The hypothesis under test: the big flat walkable sheets in the
    // shipped `castle_cellblock.nav` come from the persistent package's
    // level `Model`, which no chunk walker visits.
    //
    // They do not. Both master packages hold exactly three `Model`
    // exports — the root builder brush, the level's own, and one
    // TriggerVolume's — and the first two are 108-byte empty stubs.
    for (map, file) in [
        ("Castle_CellBlock", "Castle_CellBlock.umap"),
        ("Castle", "Castle.umap"),
    ] {
        let dir = map_dir(map);
        if skip_if_missing(&dir, "persistent_map_packages_carry_no_bsp_world_geometry") {
            return;
        }
        let path = dir.join(file);
        assert!(path.exists(), "{file} missing");
        let pkg = Package::open(&path).expect("open persistent package");

        let mut failures = Vec::new();
        let mut inventory = Vec::new();
        for (idx, export) in pkg.exports.iter().enumerate() {
            let class = pkg.export_class_name(export);
            if class != "Model" && class != "Polys" {
                continue;
            }
            let owner = if export.package_index > 0 {
                pkg.exports
                    .get((export.package_index - 1) as usize)
                    .map(|o| pkg.export_class_name(o).to_string())
                    .unwrap_or_else(|| "<oob>".into())
            } else {
                "<root>".into()
            };
            let data = pkg.read_export_data(export).expect("read export");
            let nodes = if class == "Model" {
                match deserialize_model(&data, &pkg.names) {
                    Ok(m) => m.nodes.len(),
                    Err(e) => {
                        failures.push(format!("{class}#{} ({}B): {e}", idx + 1, data.len()));
                        continue;
                    }
                }
            } else {
                match deserialize_polys(&data, &pkg.names) {
                    Ok(p) => p.elements.len(),
                    Err(e) => {
                        failures.push(format!("{class}#{} ({}B): {e}", idx + 1, data.len()));
                        continue;
                    }
                }
            };
            inventory.push(format!(
                "{class}#{} owner={owner} {}B nodes-or-elems={nodes}",
                idx + 1,
                data.len()
            ));
        }
        eprintln!("{file}: {} exports total", pkg.exports.len());
        for line in &inventory {
            eprintln!("  {line}");
        }
        assert!(
            failures.is_empty(),
            "{file}: Model/Polys exports must consume their serial data exactly:\n{}",
            failures.join("\n")
        );

        let mut soup = TriangleSoup::new(None);
        let stats = collect_bsp_triangles(&pkg, &mut soup);
        eprintln!(
            "{file}: models total={} parsed={} failed={} empty={} level={} \
             actor_included={} actor_excluded={} -> {} BSP triangles",
            stats.models_total,
            stats.models_parsed,
            stats.models_failed,
            stats.models_empty,
            stats.level_models,
            stats.actor_models_included,
            stats.actor_models_excluded,
            stats.triangles_emitted,
        );
        assert_eq!(stats.models_failed, 0, "{file}: decode failures");
        assert_eq!(
            stats.triangles_emitted, 0,
            "{file}: the persistent package was expected to hold no BSP \
             world geometry — if it now does, the big flat sheets in \
             castle_cellblock.nav may be recoverable from here after all"
        );
    }
}

#[test]
fn castle_cellblock_bsp_horizontal_sheets() {
    // Does the decoded Castle_CellBlock BSP (persistent + all chunks)
    // reproduce the three large flat walkable sheets the shipped
    // `data/spaces/castle_cellblock.nav` contains but StaticMesh
    // extraction did not recover?
    //
    //   30,499 m^2 at BW y ~= 94.6  x[-384.4,-212.5] z[-239.8,-56.2]
    //   22,838 m^2 at BW y ~= 53.4  x[-169.6, -36.1] z[-193.9,-22.3]
    //    3,836 m^2 at BW y ~= 24.8  x[-148.3, -37.0] z[-178.3,-34.9]
    //
    // Reported, not asserted beyond "the decode produced geometry":
    // whether the sheets are BSP is the finding, either way.
    let dir = map_dir("Castle_CellBlock");
    if skip_if_missing(&dir, "castle_cellblock_bsp_horizontal_sheets") {
        return;
    }
    let mut packages = vec![dir.join("Castle_CellBlock.umap")];
    packages.extend(enumerate_chunks(&dir).expect("enumerate_chunks"));

    // (BW y bucket in half-metres) -> (tris, area m^2, x min/max, z min/max)
    let mut buckets: BTreeMap<i64, (usize, f64, f32, f32, f32, f32)> = BTreeMap::new();
    let mut total_tris = 0usize;
    let mut packages_with_bsp = 0usize;
    for path in &packages {
        let Ok(pkg) = Package::open(path) else {
            continue;
        };
        let (instances, stats) = collect_bsp_models(&pkg);
        assert_eq!(
            stats.models_failed,
            0,
            "{}: Model decode failure: {:?}",
            path.display(),
            stats.parse_errors
        );
        if instances.is_empty() {
            continue;
        }
        let mut emitted = 0usize;
        for inst in &instances {
            let t = inst.model.triangulate(CollisionFilter::default());
            for tri in &t.triangles {
                emitted += 1;
                let w = [
                    inst.to_world(tri[0]),
                    inst.to_world(tri[1]),
                    inst.to_world(tri[2]),
                ];
                let n = winding_normal(w);
                let len = norm3(n);
                if len < 1e-6 {
                    continue;
                }
                if (n[2] / len).abs() < 0.86 {
                    continue; // not near-horizontal
                }
                let area = 0.5 * len as f64 / 10_000.0;
                // BW = (ue.y/100, ue.z/100, ue.x/100)
                let bw_y = (w[0][2] + w[1][2] + w[2][2]) / 3.0 / 100.0;
                let key = (bw_y * 2.0).round() as i64;
                let e =
                    buckets
                        .entry(key)
                        .or_insert((0, 0.0, f32::MAX, f32::MIN, f32::MAX, f32::MIN));
                e.0 += 1;
                e.1 += area;
                for v in &w {
                    let (bx, bz) = (v[1] / 100.0, v[0] / 100.0);
                    e.2 = e.2.min(bx);
                    e.3 = e.3.max(bx);
                    e.4 = e.4.min(bz);
                    e.5 = e.5.max(bz);
                }
            }
        }
        total_tris += emitted;
        if emitted > 0 {
            packages_with_bsp += 1;
        }
    }

    eprintln!(
        "Castle_CellBlock BSP: {} packages scanned (1 persistent + {} chunks), \
         {packages_with_bsp} with BSP geometry, {total_tris} triangles",
        packages.len(),
        packages.len() - 1
    );
    eprintln!("  near-horizontal BSP area by BW y (0.5 m buckets, >= 100 m^2 only):");
    eprintln!(
        "  {:>9}  {:>6}  {:>11}  {:>19}  {:>19}",
        "BW y", "tris", "area m^2", "BW x range", "BW z range"
    );
    let mut rows: Vec<_> = buckets.iter().filter(|(_, v)| v.1 >= 100.0).collect();
    rows.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap());
    for (k, v) in rows.iter().take(25) {
        eprintln!(
            "  {:>9.1}  {:>6}  {:>11.1}  [{:>8.1},{:>8.1}]  [{:>8.1},{:>8.1}]",
            **k as f64 / 2.0,
            v.0,
            v.1,
            v.2,
            v.3,
            v.4,
            v.5
        );
    }

    for (label, y, area) in [
        ("sheet A", 94.6f64, 30_499.0f64),
        ("sheet B", 53.4, 22_838.0),
        ("sheet C", 24.8, 3_836.0),
    ] {
        let hit: f64 = buckets
            .iter()
            .filter(|(k, _)| ((**k as f64 / 2.0) - y).abs() <= 1.0)
            .map(|(_, v)| v.1)
            .sum();
        eprintln!(
            "  {label}: nav sheet {area:.0} m^2 at BW y {y} -> BSP has {hit:.1} m^2 \
             of near-horizontal area within +/-1 m"
        );
    }

    assert!(total_tris > 0, "Castle_CellBlock produced no BSP triangles");
}

// ---------------------------------------------------------------------
// ModelComponent audit + grid floor probe.
// ---------------------------------------------------------------------

#[test]
fn model_components_do_not_gate_bsp_collision() {
    // `ModelComponent` is the *rendering* proxy for a subset of the
    // level `Model`'s nodes (its source anchor is `UnModelRender.cpp`).
    // BSP collision is served by `UModel` itself: the decompiled
    // point-classify walker (`UnModelCollision.cpp`) descends the
    // Model's own `iChild[3]` tree from the root and never consults a
    // component. So the Model's `Nodes` array is the complete
    // collision surface set and ModelComponents can be ignored —
    // PROVIDED no component turns collision off for its slice.
    //
    // This test checks that proviso the cheap way: it reads every
    // ModelComponent's tagged-property block and fails if any of them
    // carries a collision-disabling flag. It also prints the property
    // histogram so a reader can see what these components actually
    // declare.
    let dir = castle_dir();
    if skip_if_missing(&dir, "model_components_do_not_gate_bsp_collision") {
        return;
    }

    let mut prop_hist: BTreeMap<String, usize> = BTreeMap::new();
    let mut bool_false: BTreeMap<String, usize> = BTreeMap::new();
    let mut components = 0usize;
    let mut tiles_with_components = 0usize;
    for tile in INTERIOR_TILES {
        let path = dir.join(format!("Castle-{tile}.umap"));
        if !path.exists() {
            eprintln!("  (tile {tile} missing)");
            continue;
        }
        let pkg = Package::open(&path).expect("open chunk");
        let mut here = 0usize;
        for export in &pkg.exports {
            if pkg.export_class_name(export) != "ModelComponent" {
                continue;
            }
            here += 1;
            components += 1;
            let Ok(data) = pkg.read_export_data(export) else {
                continue;
            };
            // Components carry an 8-byte binary prefix before their
            // property block (verified for StaticMeshComponent in
            // `staticmesh.rs`; same UObject/UComponent shape).
            if data.len() <= 8 {
                continue;
            }
            let props = cimmeria_upk::parse_tagged_properties(&data, 8, &pkg.names);
            for p in &props {
                *prop_hist.entry(p.name.clone()).or_default() += 1;
                if let cimmeria_upk::PropValue::Bool(false) = p.value {
                    *bool_false.entry(p.name.clone()).or_default() += 1;
                }
            }
        }
        if here > 0 {
            tiles_with_components += 1;
        }
    }

    eprintln!(
        "ModelComponent audit: {components} components across \
         {tiles_with_components} of {} interior tiles",
        INTERIOR_TILES.len()
    );
    eprintln!("  tagged-property histogram: {prop_hist:?}");
    eprintln!("  properties explicitly set false: {bool_false:?}");

    // The flags that would make a component's slice non-blocking. If
    // any component sets one, the "ignore ModelComponents" shortcut is
    // unsafe and the decoder needs to learn the component node lists.
    for flag in [
        "CollideActors",
        "BlockActors",
        "BlockRigidBody",
        "BlockZeroExtent",
    ] {
        assert_eq!(
            bool_false.get(flag).copied().unwrap_or(0),
            0,
            "a ModelComponent sets {flag} = False — BSP collision is no \
             longer uniform across the level Model's nodes, so the \
             decoder must parse ModelComponent node lists instead of \
             emitting every node"
        );
    }
}

#[test]
fn bsp_floor_grid_probe_over_the_known_floor_planes() {
    // nav-extract's StaticMesh probe found a floor at 2 of 1,365 grid
    // points in the Interrogation Block. This is the same question
    // asked of the BSP decode: over a 1 m grid across each tile's
    // 100x100 BW footprint, how many points have an upward-facing BSP
    // face on the known floor plane?
    //
    // "Upward-facing" uses the authored surface normal, so the answer
    // does not depend on emitted winding.
    let dir = castle_dir();
    if skip_if_missing(&dir, "bsp_floor_grid_probe_over_the_known_floor_planes") {
        return;
    }

    // (tile, label, floor plane BW y, tile grid origin in BW x/z)
    //
    // Chunk id `0x00XX00YY` decodes to positionX = low u16, positionZ =
    // high u16, 100 BW units per chunk (see `chunk_id.rs`).
    let planes = [
        (
            "000a0002",
            "Interrogation Block floor",
            66.79f32,
            200.0f32,
            1000.0f32,
        ),
        ("00080002", "Level-5 comms floor", 55.20, 200.0, 800.0),
    ];
    // Vertical tolerance either side of the plane, in BW units.
    const TOL: f32 = 1.5;

    let mut any_hits = false;
    for (tile, label, plane_y, ox, oz) in planes {
        let path = dir.join(format!("Castle-{tile}.umap"));
        if !path.exists() {
            eprintln!("SKIPPED grid probe {label} — tile {tile} missing");
            continue;
        }
        let tris = world_triangles(&path);

        // Pre-filter to upward-facing faces near the plane so the grid
        // sweep stays cheap in a debug build.
        let plane_ue_z = plane_y * 100.0;
        let near: Vec<&WorldTri> = tris
            .iter()
            .filter(|wt| {
                let len = norm3(wt.surf_normal);
                if len < 1e-6 {
                    return false;
                }
                if wt.surf_normal[2] / len <= 0.0 {
                    return false;
                }
                let zc = (wt.tri[0][2] + wt.tri[1][2] + wt.tri[2][2]) / 3.0;
                (zc - plane_ue_z).abs() <= TOL * 100.0
            })
            .collect();

        let mut hits = 0usize;
        let mut total = 0usize;
        let mut area = 0.0f64;
        for wt in &near {
            area += 0.5 * norm3(winding_normal(wt.tri)) as f64 / 10_000.0;
        }
        // 1 m grid over the chunk's 100x100 BW footprint.
        for gx in 0..100 {
            for gz in 0..100 {
                total += 1;
                let bw = [ox + gx as f32 + 0.5, plane_y, oz + gz as f32 + 0.5];
                let ue = bw_to_ue(bw);
                let hit = near.iter().any(|wt| {
                    contains_xy(wt.tri, ue[0], ue[1])
                        && interp_z(wt.tri, ue[0], ue[1])
                            .map(|z| (ue[2] - z).abs() <= TOL * 100.0)
                            .unwrap_or(false)
                });
                if hit {
                    hits += 1;
                }
            }
        }
        eprintln!(
            "GRID PROBE {label} (tile {tile}, BW y {plane_y}): {hits}/{total} \
             1 m grid points have an upward-facing BSP face within +/-{TOL} BW \
             units ({} candidate faces, {area:.0} m^2)",
            near.len()
        );
        if hits > 0 {
            any_hits = true;
        }
    }

    assert!(
        any_hits,
        "no BSP floor coverage at either known floor plane — this is the \
         critical-path result and it must not silently regress"
    );
}
