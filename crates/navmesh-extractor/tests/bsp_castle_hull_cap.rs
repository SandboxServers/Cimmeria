//! Regression guards for the outer-hull cap filter
//! (`bsp::hull_cap`).
//!
//! The Castle interior's enclosing additive CSG block leaves a
//! horizontal top face at BW y 79.36 and a bottom face at BW y 26.88 in
//! every chunk that carries the hull. The top face is upward-facing, so
//! NavBuilder rasterises it into walkable polygons at y 79.5-80.3
//! spanning x[203,477] z[615,1101] — buried under the terrain (y 93-204
//! there), reachable by nothing. On the 144-chunk build it is 87,709 m²,
//! 8.1 % of the mesh's walkable area.
//!
//! What these tests pin, in order:
//!
//! 1. the filter fires on every chunk that carries the hull and removes
//!    only the two extreme planes,
//! 2. it removes the buried subset and not the whole geometric skin —
//!    the part that pokes above the terrain or sits under a terrain
//!    hole stays,
//! 3. it removes **nothing** from the two known floor planes: the 1 m
//!    grid probe returns the same 4,518 / 2,389 hits it did before, and
//!    every known-walkable point still sits on an upward BSP face.
//!
//! The burial half of the rule is load-bearing, not belt-and-braces:
//! `hull_cap`'s module doc records that the geometric half alone
//! deletes two of the three large sheets the shipped
//! `data/spaces/castle_cellblock.nav` contains.
//!
//! Every test self-skips when the cooked client tree is absent, and says
//! so loudly. A skipped test is not a pass.

mod bsp_support;

use bsp_support::*;

use cimmeria_navmesh_extractor::bsp::{collect_bsp_triangles, BspOptions};
use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_upk::Package;

/// The two planes the Castle hull skin sits on, BW metres.
const HULL_TOP_BW_Y: f32 = 79.36;
const HULL_BOTTOM_BW_Y: f32 = 26.88;
/// Chunks whose level `Model` carries the enclosing hull. The other
/// three chunks in `INTERIOR_TILES` hold small brush fragments whose
/// outer planes are not buried under their chunk's terrain, so they are
/// kept.
const HULL_TILES: [&str; 13] = [
    "00060003", "00070002", "00070003", "00070004", "00080002", "00080003", "00080004", "00090002",
    "00090003", "00090004", "000a0002", "000a0003", "000a0004",
];

/// No terrain ceiling == no burial evidence == nothing dropped. This
/// is also what a caller that forgot the ceiling would get, which is
/// why the "before" side of every comparison below uses it.
fn keep_all() -> BspOptions<'static> {
    BspOptions::default()
}

#[test]
fn hull_cap_fires_on_every_chunk_that_carries_the_hull() {
    let dir = castle_dir();
    if skip_if_missing(&dir, "hull_cap_fires_on_exactly_the_thirteen_hull_chunks") {
        return;
    }

    let mut fired: Vec<String> = Vec::new();
    let mut dropped_total = 0usize;
    let mut area_total = 0.0f64;
    println!(
        "{:<10} {:>8} {:>8} {:>8} {:>12}",
        "tile", "kept", "dropped", "models", "capArea m2"
    );
    for tile in INTERIOR_TILES {
        let path = dir.join(format!("Castle-{tile}.umap"));
        if !path.exists() {
            eprintln!("  (tile {tile} missing)");
            continue;
        }
        let pkg = Package::open(&path).expect("open chunk");

        let ceiling = terrain_ceiling(&pkg);
        let mut soup = TriangleSoup::new(None);
        let stats = collect_bsp_triangles(
            &pkg,
            &mut soup,
            BspOptions {
                terrain_ceiling: ceiling.as_ref(),
            },
        );
        let mut raw = TriangleSoup::new(None);
        let raw_stats = collect_bsp_triangles(&pkg, &mut raw, keep_all());

        assert_eq!(
            raw_stats.hull_cap_triangles_excluded, 0,
            "{tile}: without a terrain ceiling nothing may be dropped"
        );
        assert_eq!(
            stats.triangles_emitted + stats.hull_cap_triangles_excluded,
            raw_stats.triangles_emitted,
            "{tile}: filtered + dropped must equal the unfiltered count"
        );
        println!(
            "{tile:<10} {:>8} {:>8} {:>8} {:>12.0}",
            stats.triangles_emitted,
            stats.hull_cap_triangles_excluded,
            stats.models_with_hull,
            stats.hull_cap_area_m2
        );
        if stats.hull_cap_triangles_excluded > 0 {
            fired.push(tile.to_string());
        }
        dropped_total += stats.hull_cap_triangles_excluded;
        area_total += stats.hull_cap_area_m2;
    }

    println!("TOTAL dropped {dropped_total} triangles, {area_total:.0} m2");
    for tile in HULL_TILES {
        assert!(
            fired.iter().any(|t| t == tile),
            "chunk {tile} carries the enclosing hull but nothing was dropped \
             from it; fired on {fired:?}"
        );
    }
    // Measured: 178,168 m² over 1,142 triangles. The whole geometric
    // skin is 246,195 m²; the missing 28 % is the part that pokes above
    // the terrain or sits under a terrain hole, which the burial test
    // correctly keeps. A band rather than an exact number because the
    // area is a float sum — but tight enough that losing either cap
    // plane, or the burial test, fails here.
    assert!(
        (170_000.0..190_000.0).contains(&area_total),
        "expected ~178,000 m2 of buried hull skin, got {area_total:.0} m2 \
         (the unburied geometric skin is 246,195 m2, so a number near that \
          means the terrain ceiling stopped being consulted)"
    );
}

#[test]
fn hull_cap_removes_only_the_two_extreme_planes() {
    // The filter is geometric, so the thing to pin is *where* the
    // removed faces are: every one of them on the hull top or bottom,
    // and nothing in between. If the Z-extent scan ever picks up a
    // stray point the dropped set would spread across other heights.
    let dir = castle_dir();
    if skip_if_missing(&dir, "hull_cap_removes_only_the_two_extreme_planes") {
        return;
    }

    for tile in HULL_TILES {
        let path = dir.join(format!("Castle-{tile}.umap"));
        if !path.exists() {
            eprintln!("  (tile {tile} missing)");
            continue;
        }
        let before = world_triangles(&path);
        let after = world_triangles_shipped(&path);
        assert!(
            after.len() < before.len(),
            "{tile}: the cap filter removed nothing"
        );

        // Every face present before but not after must be flat and on
        // one of the two planes, facing outward.
        let kept: std::collections::HashSet<[u32; 9]> = after.iter().map(|w| key(&w.tri)).collect();
        let mut off_plane = Vec::new();
        for w in &before {
            if kept.contains(&key(&w.tri)) {
                continue;
            }
            let bw_y = (w.tri[0][2] + w.tri[1][2] + w.tri[2][2]) / 3.0 / 100.0;
            let nz = w.surf_normal[2] / norm3(w.surf_normal).max(1e-9);
            let on_top = (bw_y - HULL_TOP_BW_Y).abs() < 0.02 && nz > 0.0;
            let on_bottom = (bw_y - HULL_BOTTOM_BW_Y).abs() < 0.02 && nz < 0.0;
            if !on_top && !on_bottom {
                off_plane.push(format!("bw_y={bw_y:.2} surf_n.z={nz:+.3}"));
            }
        }
        assert!(
            off_plane.is_empty(),
            "{tile}: the cap filter dropped {} face(s) that are not on the \
             hull skin: {:?}",
            off_plane.len(),
            &off_plane[..off_plane.len().min(8)]
        );
    }
}

#[test]
fn hull_cap_leaves_the_known_floor_planes_untouched() {
    // The acceptance condition from the brief: the 1 m grid floor probe
    // over the two known floor planes must return the same hit counts
    // with the filter on as it did before it existed.
    //
    // 4,518 / 10,000 for the Interrogation Block at BW y 66.79 and
    // 2,389 / 10,000 for the Level-5 comms room at BW y 55.20 — both
    // measured on this branch's parent, both far from the hull skin at
    // 26.88 / 79.36.
    let dir = castle_dir();
    if skip_if_missing(&dir, "hull_cap_leaves_the_known_floor_planes_untouched") {
        return;
    }

    for (tile, label, plane_y, ox, oz, expected) in [
        (
            "000a0002",
            "Interrogation Block floor",
            66.79f32,
            200.0f32,
            1000.0f32,
            4518usize,
        ),
        ("00080002", "Level-5 comms floor", 55.20, 200.0, 800.0, 2389),
    ] {
        let path = dir.join(format!("Castle-{tile}.umap"));
        if !path.exists() {
            eprintln!("SKIPPED grid probe {label} — tile {tile} missing");
            continue;
        }
        let before = grid_hits(&world_triangles(&path), plane_y, ox, oz);
        let after = grid_hits(&world_triangles_shipped(&path), plane_y, ox, oz);
        println!("GRID PROBE {label} (tile {tile}): before={before} after={after}");
        assert_eq!(
            before, expected,
            "{label}: the unfiltered baseline moved — re-measure before \
             trusting the after/before comparison"
        );
        assert_eq!(
            after, before,
            "{label}: the hull-cap filter removed floor coverage"
        );
    }
}

#[test]
fn hull_cap_keeps_every_known_walkable_point_on_a_floor() {
    // The five points the 17-tile acceptance build lands within 0.4 m
    // of. Asked of the extractor rather than the .nav: is there still an
    // upward-facing BSP face under each one after the filter?
    let dir = castle_dir();
    if skip_if_missing(&dir, "hull_cap_keeps_every_known_walkable_point_on_a_floor") {
        return;
    }

    let probes: [(&str, &str, [f32; 3]); 6] = [
        ("zuritska_cell", "000a0002", [268.0, 66.79, 1042.59]),
        ("romney_corridor", "000a0002", [244.0, 66.79, 1036.0]),
        ("comms_room", "00080002", [271.7, 55.2, 858.0]),
        ("nid_guard_116", "00080002", [294.16, 55.39, 894.44]),
        ("armory", "00090004", [466.365, 70.397, 991.466]),
        // The throne room floor is BSP too: removing the BSP plane at
        // BW y 38.08 drops its 1 m grid coverage from 381/841 to 27/841.
        ("throne_room", "00060003", [353.42, 38.17, 636.0]),
    ];

    let mut cache: std::collections::BTreeMap<String, Vec<WorldTri>> = Default::default();
    for (label, tile, bw) in probes {
        let path = dir.join(format!("Castle-{tile}.umap"));
        if !path.exists() {
            eprintln!("SKIPPED probe {label} — tile {tile} missing");
            continue;
        }
        let tris = cache
            .entry(tile.to_string())
            .or_insert_with(|| world_triangles_shipped(&path));
        let drop = nearest_upward_drop_cm(tris, bw)
            .unwrap_or_else(|| panic!("{label}: no upward BSP face at this XY at any height"));
        println!("{label:<18} upward BSP face {drop:+.1} cm from the recorded Y");
        assert!(
            drop.abs() <= 40.0,
            "{label}: nearest upward BSP face is {drop:.0} cm from the recorded \
             Y — the hull-cap filter took out a real floor"
        );
    }
}

// --- helpers ---------------------------------------------------------

/// Bitwise identity key for a triangle — the two collections are built
/// from the same floats, so an exact key is safe and exact is what we
/// want (a near-match would hide a shifted vertex).
fn key(t: &[[f32; 3]; 3]) -> [u32; 9] {
    let mut k = [0u32; 9];
    for (i, v) in t.iter().enumerate() {
        for (j, c) in v.iter().enumerate() {
            k[i * 3 + j] = c.to_bits();
        }
    }
    k
}

/// How many of a 100x100 one-metre grid over a chunk's footprint have an
/// upward-facing BSP face within 1.5 BW units of `plane_y`.
fn grid_hits(tris: &[WorldTri], plane_y: f32, ox: f32, oz: f32) -> usize {
    const TOL: f32 = 1.5;
    let plane_ue_z = plane_y * 100.0;
    let near: Vec<&WorldTri> = tris
        .iter()
        .filter(|wt| {
            let len = norm3(wt.surf_normal);
            if len < 1e-6 || wt.surf_normal[2] / len <= 0.0 {
                return false;
            }
            let zc = (wt.tri[0][2] + wt.tri[1][2] + wt.tri[2][2]) / 3.0;
            (zc - plane_ue_z).abs() <= TOL * 100.0
        })
        .collect();

    let mut hits = 0;
    for gx in 0..100 {
        for gz in 0..100 {
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
    hits
}

/// Signed distance, in cm, from `bw` down to the nearest upward-facing
/// BSP face under the same XY. Positive means the face is below.
fn nearest_upward_drop_cm(tris: &[WorldTri], bw: [f32; 3]) -> Option<f32> {
    let ue = bw_to_ue(bw);
    let mut best: Option<f32> = None;
    for wt in tris {
        let len = norm3(wt.surf_normal);
        if len < 1e-6 || wt.surf_normal[2] / len <= 0.0 {
            continue;
        }
        if !contains_xy(wt.tri, ue[0], ue[1]) {
            continue;
        }
        let Some(z) = interp_z(wt.tri, ue[0], ue[1]) else {
            continue;
        };
        let drop = ue[2] - z;
        if best.map(|b: f32| drop.abs() < b.abs()).unwrap_or(true) {
            best = Some(drop);
        }
    }
    best
}
