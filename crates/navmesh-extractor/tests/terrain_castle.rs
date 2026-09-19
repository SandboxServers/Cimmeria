//! Phase 1.3 acceptance tests: decode real `Terrain` exports from the
//! cooked Castle and Castle_CellBlock maps and check them against the
//! shipped 2013 navmesh.
//!
//! Every test here self-skips when the cooked asset bundle is absent —
//! a skip is printed loudly and is **not** a pass. The numbers pinned
//! below were measured against the QA asset tree; a drift means either
//! the decoder changed or the bundle did.

use std::path::{Path, PathBuf};

use cimmeria_navmesh_extractor::geometry::TriangleSoup;
use cimmeria_navmesh_extractor::nav_roundtrip::XrcNav;
use cimmeria_navmesh_extractor::terrain::{collect_terrain_triangles, TerrainStats};
use cimmeria_navmesh_extractor::umap::enumerate_chunks;
use cimmeria_upk::Package;
use cimmeria_upk_objects::{deserialize_terrain, Terrain};

/// Locate a cooked map directory. The SGW asset tree is a sibling of
/// the Cimmeria repo, but this crate may be tested from the main
/// checkout or from a worktree under `.claude/worktrees/<slug>/`, so
/// walk ancestors instead of hardcoding a climb count.
fn map_dir(name: &str) -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let suffix = PathBuf::from("sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps").join(name);
    for ancestor in manifest.ancestors().take(10) {
        let candidate = ancestor.join(&suffix);
        if candidate.exists() {
            return candidate;
        }
    }
    manifest.join(&suffix)
}

fn skip_if_missing(path: &Path, what: &str) -> bool {
    if !path.exists() {
        eprintln!(
            "SKIPPED {what} — cooked asset bundle not present at {}",
            path.display()
        );
        return true;
    }
    false
}

/// Decode every `Terrain` export in one `.umap`.
fn decode_chunk(path: &Path) -> Vec<Terrain> {
    let pkg = Package::open(path).expect("open chunk");
    pkg.exports
        .iter()
        .filter(|e| pkg.export_class_name(e) == "Terrain")
        .map(|e| {
            let data = pkg.read_export_data(e).expect("read export data");
            deserialize_terrain(&data, &pkg.names)
                .unwrap_or_else(|err| panic!("deserialize {}: {err}", e.object_name))
        })
        .collect()
}

fn chunk_stats(path: &Path) -> (TerrainStats, TriangleSoup) {
    let pkg = Package::open(path).expect("open chunk");
    let mut soup = TriangleSoup::new(Some("Chunk_test".to_string()));
    let stats = collect_terrain_triangles(&pkg, &mut soup);
    (stats, soup)
}

#[test]
fn castle_cellblock_chunk_terrain_is_byte_exact() {
    let dir = map_dir("Castle_CellBlock");
    if skip_if_missing(&dir, "castle_cellblock_chunk_terrain_is_byte_exact") {
        return;
    }
    let path = dir.join("Castle_CellBlock-00000000.umap");
    let terrains = decode_chunk(&path);

    // Castle_CellBlock's convention: 25 separate small Terrain actors
    // per 100 m chunk, no section subdivision.
    assert_eq!(terrains.len(), 25, "terrain actors in chunk 00000000");
    for t in &terrains {
        assert_eq!((t.num_patches_x, t.num_patches_y), (20, 20));
        assert_eq!((t.num_vertices_x, t.num_vertices_y), (21, 21));
        assert_eq!((t.num_sections_x, t.num_sections_y), (1, 1));
        assert_eq!(t.heights.len(), 441);
        assert_eq!(t.info_data.len(), 441);
        assert_eq!((t.alpha_x_size, t.alpha_y_size), (84, 84));
        assert_eq!(t.weighted_texture_map_count, 1);
        assert_eq!(t.weight_map_texture_count, 0);
        // The only bytes this decoder does not model. 9372-byte export,
        // trailer starts at +813, lighting/foliage tail is 152 bytes.
        assert_eq!(t.lighting_trailer_bytes, 152);
        // Every vertex is the neutral 0x8000 — this chunk is dead flat.
        assert!(t.heights.iter().all(|h| *h == 0x8000));
        // Terrain Location is absolute world space, on the 2000 cm grid.
        assert_eq!(t.location[2], 0.0);
        assert_eq!(t.location[0] % 2000.0, 0.0, "loc {:?}", t.location);
        assert_eq!(t.location[1] % 2000.0, 0.0, "loc {:?}", t.location);
    }

    // The 25 actors tile a 5x5 grid covering the chunk's 10000 cm.
    let xs: Vec<f32> = terrains.iter().map(|t| t.location[0]).collect();
    let ys: Vec<f32> = terrains.iter().map(|t| t.location[1]).collect();
    assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), 0.0);
    assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 8000.0);
    assert_eq!(ys.iter().cloned().fold(f32::MAX, f32::min), 0.0);
    assert_eq!(ys.iter().cloned().fold(f32::MIN, f32::max), 8000.0);
}

#[test]
fn castle_tile_terrain_is_byte_exact() {
    let dir = map_dir("Castle");
    if skip_if_missing(&dir, "castle_tile_terrain_is_byte_exact") {
        return;
    }
    let terrains = decode_chunk(&dir.join("Castle-000a0002.umap"));

    // Castle's convention: ONE Terrain actor per chunk, partitioned
    // into NumSectionsX * NumSectionsY = 25 TerrainComponent exports.
    assert_eq!(terrains.len(), 1, "Castle uses one Terrain per chunk");
    let t = &terrains[0];
    assert_eq!((t.num_patches_x, t.num_patches_y), (100, 100));
    assert_eq!((t.num_vertices_x, t.num_vertices_y), (101, 101));
    assert_eq!((t.num_sections_x, t.num_sections_y), (5, 5));
    assert_eq!(t.heights.len(), 10201);
    assert_eq!(t.info_data.len(), 10201);
    assert_eq!((t.alpha_x_size, t.alpha_y_size), (404, 404));
    // Three texture layers here — the count is content-dependent, not
    // always 1 as the first RE sample suggested.
    assert_eq!(t.weighted_texture_map_count, 3);
    assert_eq!(t.weight_map_texture_count, 0);
    assert_eq!(t.lighting_trailer_bytes, 164);
    assert_eq!(t.location, [100_000.0, 20_000.0, 0.0]);
    // Real relief, not the flat 0x8000 sheet Castle_CellBlock ships.
    assert_eq!(t.heights.iter().copied().min(), Some(44226));
    assert_eq!(t.heights.iter().copied().max(), Some(54199));
    // DrawScale3D is absent on every Castle terrain: 100 patches x the
    // class-default 100 cm = exactly one 100 m chunk.
    assert_eq!(t.draw_scale_3d, [100.0, 100.0, 100.0]);
}

#[test]
fn castle_cellblock_hole_quads_are_skipped() {
    let dir = map_dir("Castle_CellBlock");
    if skip_if_missing(&dir, "castle_cellblock_hole_quads_are_skipped") {
        return;
    }

    // fffefffe sits directly under the castle: 9500 of its 10000 patch
    // quads are flagged TID_Visibility_Off.
    let (holed, _) = chunk_stats(&dir.join("Castle_CellBlock-fffefffe.umap"));
    assert_eq!(holed.terrain_actors, 25);
    assert_eq!(holed.parse_failures, 0);
    assert_eq!(holed.quads_total, 10_000);
    assert_eq!(holed.quads_holed, 9_500);
    assert_eq!(holed.triangles_emitted, 1_000);

    // fffefffd is the densest StaticMesh chunk but its terrain has no
    // holes at all — a full 20000-triangle sheet.
    let (solid, soup) = chunk_stats(&dir.join("Castle_CellBlock-fffefffd.umap"));
    assert_eq!(solid.quads_total, 10_000);
    assert_eq!(solid.quads_holed, 0);
    assert_eq!(solid.triangles_emitted, 20_000);
    assert_eq!(soup.triangle_count(), 20_000);
}

#[test]
fn castle_chunk_terrain_emits_full_resolution_triangles() {
    let dir = map_dir("Castle");
    if skip_if_missing(&dir, "castle_chunk_terrain_emits_full_resolution_triangles") {
        return;
    }
    let (stats, soup) = chunk_stats(&dir.join("Castle-000a0002.umap"));
    assert_eq!(stats.terrain_actors, 1);
    assert_eq!(stats.parse_failures, 0);
    assert_eq!(stats.quads_total, 10_000);
    assert_eq!(stats.quads_holed, 0);
    assert_eq!(stats.triangles_emitted, 20_000);
    assert_eq!(soup.triangle_count(), 20_000);

    // One 100-patch terrain spans exactly one 100 m chunk.
    let xs: Vec<f32> = soup.vertices.iter().map(|v| v[0]).collect();
    let ys: Vec<f32> = soup.vertices.iter().map(|v| v[1]).collect();
    assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), 100_000.0);
    assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 110_000.0);
    assert_eq!(ys.iter().cloned().fold(f32::MAX, f32::min), 20_000.0);
    assert_eq!(ys.iter().cloned().fold(f32::MIN, f32::max), 30_000.0);
}

/// Sum the XZ-plane area of every shipped nav polygon whose average
/// BW y falls in `band`, and return `(area_m2, x_min, x_max, z_min, z_max)`.
fn shipped_band(nav: &XrcNav, band: std::ops::Range<f32>) -> (f64, f32, f32, f32, f32) {
    let nvp = nav.nvp as usize;
    let (mut area, mut x0, mut x1, mut z0, mut z1) =
        (0.0f64, f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for p in 0..nav.npolys as usize {
        let idx: Vec<usize> = (0..nvp)
            .map(|k| nav.polys[p * nvp * 2 + k])
            .filter(|v| *v != 0xFFFF)
            .map(|v| v as usize)
            .collect();
        if idx.len() < 3 {
            continue;
        }
        let pts: Vec<[f32; 3]> = idx
            .iter()
            .map(|i| {
                [
                    nav.verts[i * 3] as f32 * nav.cs + nav.bmin[0],
                    nav.verts[i * 3 + 1] as f32 * nav.ch + nav.bmin[1],
                    nav.verts[i * 3 + 2] as f32 * nav.cs + nav.bmin[2],
                ]
            })
            .collect();
        let y = pts.iter().map(|q| q[1]).sum::<f32>() / pts.len() as f32;
        if !band.contains(&y) {
            continue;
        }
        let mut cross = 0.0f64;
        for k in 0..pts.len() {
            let a = pts[k];
            let b = pts[(k + 1) % pts.len()];
            cross += a[0] as f64 * b[2] as f64 - b[0] as f64 * a[2] as f64;
        }
        area += cross.abs() * 0.5;
        for q in &pts {
            x0 = x0.min(q[0]);
            x1 = x1.max(q[0]);
            z0 = z0.min(q[2]);
            z1 = z1.max(q[2]);
        }
    }
    (area, x0, x1, z0, z1)
}

#[test]
fn castle_cellblock_terrain_reproduces_the_shipped_ground_sheet() {
    let dir = map_dir("Castle_CellBlock");
    if skip_if_missing(
        &dir,
        "castle_cellblock_terrain_reproduces_the_shipped_ground_sheet",
    ) {
        return;
    }

    let mut total = TerrainStats::default();
    let (mut ux0, mut ux1, mut uy0, mut uy1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let mut zs: Vec<f32> = Vec::new();
    for chunk in enumerate_chunks(&dir).expect("enumerate_chunks") {
        let (stats, soup) = chunk_stats(&chunk);
        total.terrain_actors += stats.terrain_actors;
        total.parse_failures += stats.parse_failures;
        total.quads_total += stats.quads_total;
        total.quads_holed += stats.quads_holed;
        total.triangles_emitted += stats.triangles_emitted;
        for v in &soup.vertices {
            ux0 = ux0.min(v[0]);
            ux1 = ux1.max(v[0]);
            uy0 = uy0.min(v[1]);
            uy1 = uy1.max(v[1]);
            if !zs.contains(&v[2]) {
                zs.push(v[2]);
            }
        }
    }

    assert_eq!(total.parse_failures, 0, "every Terrain export must decode");
    assert_eq!(total.terrain_actors, 1600, "64 chunks x 25 terrain actors");
    assert_eq!(total.quads_total, 640_000);
    assert_eq!(total.quads_holed, 34_327);
    assert_eq!(total.triangles_emitted, 1_211_346);
    // Every Castle_CellBlock terrain is flat at Location.Z = 0.
    assert_eq!(zs, vec![0.0], "all decoded terrain sits at world Z = 0");

    // Decoded coverage, converted to BW: x = ue_y/100, z = ue_x/100.
    let (bx0, bx1) = (uy0 / 100.0, uy1 / 100.0);
    let (bz0, bz1) = (ux0 / 100.0, ux1 / 100.0);
    let decoded_area = (total.quads_total - total.quads_holed) as f64; // 1 m² per quad

    let nav_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/spaces/castle_cellblock.nav");
    let mut f = std::fs::File::open(&nav_path).expect("shipped castle_cellblock.nav");
    let nav = XrcNav::read(&mut f).expect("parse shipped nav");
    let (ground_area, sx0, sx1, sz0, sz1) = shipped_band(&nav, 0.0..0.5);

    eprintln!(
        "decoded terrain: {decoded_area:.0} m² over BW x[{bx0:.1},{bx1:.1}] z[{bz0:.1},{bz1:.1}]\n\
         shipped y≈0.2 sheet: {ground_area:.0} m² over BW x[{sx0:.1},{sx1:.1}] z[{sz0:.1},{sz1:.1}]"
    );

    // Footprint must line up to within Recast's agent-radius erosion
    // (0.6 m) plus a cell of slack.
    for (a, b, what) in [
        (bx0, sx0, "x min"),
        (bx1, sx1, "x max"),
        (bz0, sz0, "z min"),
        (bz1, sz1, "z max"),
    ] {
        assert!(
            (a - b).abs() < 1.5,
            "{what}: decoded {a:.2} vs shipped {b:.2}"
        );
    }

    // The shipped ground sheet also covers the 34,327 m² of building
    // footprint that terrain punches out as holes, so it is slightly
    // LARGER than the decoded terrain. Anything below ~90% would mean
    // the decode lost real ground.
    let ratio = decoded_area / ground_area;
    assert!(
        (0.90..=1.02).contains(&ratio),
        "decoded/shipped ground area ratio {ratio:.3} out of range \
         (decoded {decoded_area:.0} m², shipped {ground_area:.0} m²)"
    );
}

/// Bilinear sample of a decoded terrain's world height at UE3
/// `(x, y)` cm. Returns `None` when the point is outside the actor.
fn sample_world_height(t: &Terrain, ux: f32, uy: f32) -> Option<f32> {
    let sx = t.draw_scale * t.draw_scale_3d[0];
    let sy = t.draw_scale * t.draw_scale_3d[1];
    let fx = (ux - t.location[0]) / sx;
    let fy = (uy - t.location[1]) / sy;
    if fx < 0.0 || fy < 0.0 {
        return None;
    }
    let (i, j) = (fx as u32, fy as u32);
    if i >= t.num_patches_x || j >= t.num_patches_y {
        return None;
    }
    let (tx, ty) = (fx - i as f32, fy - j as f32);
    let h = |a: u32, b: u32| t.local_vertex(a, b).unwrap()[2] * t.draw_scale * t.draw_scale_3d[2];
    let z0 = h(i, j) * (1.0 - tx) + h(i + 1, j) * tx;
    let z1 = h(i, j + 1) * (1.0 - tx) + h(i + 1, j + 1) * tx;
    Some(t.location[2] + z0 * (1.0 - ty) + z1 * ty)
}

#[test]
fn castle_terrain_matches_the_gate_room_seed_height() {
    let dir = map_dir("Castle");
    if skip_if_missing(&dir, "castle_terrain_matches_the_gate_room_seed_height") {
        return;
    }
    // Gate room / DHD, BW (806, 55.1, 517) -> UE3 (x = z*100, y = x*100).
    let terrains = decode_chunk(&dir.join("Castle-00050008.umap"));
    assert_eq!(terrains.len(), 1);
    let bw_y = sample_world_height(&terrains[0], 51_700.0, 80_600.0)
        .expect("gate-room point inside Castle-00050008 terrain")
        / 100.0;
    eprintln!("gate room: decoded BW y = {bw_y:.2}, seed BW y = 55.10");

    // This is the calibration point for the DrawScale3D Z default: at
    // the class default of 100 the decoded ground is 55.14 against a
    // seed of 55.10; at 200 it would be 110.28. A 1 m tolerance keeps
    // the test honest without pinning float noise.
    assert!(
        (bw_y - 55.10).abs() < 1.0,
        "decoded terrain height {bw_y:.2} does not match the gate-room seed 55.10"
    );
}

#[test]
fn castle_terrain_sits_below_the_checkpoint_bravo_seed() {
    let dir = map_dir("Castle");
    if skip_if_missing(&dir, "castle_terrain_sits_below_the_checkpoint_bravo_seed") {
        return;
    }
    // Checkpoint Bravo, BW ~(960, 24-28, 478) — MEDIUM confidence, and
    // the recorded position is on the checkpoint structure rather than
    // bare ground, so this is a containment check, not a calibration
    // point: the terrain must be *under* the seed and within a
    // building's height of it.
    let terrains = decode_chunk(&dir.join("Castle-00040009.umap"));
    let bw_y = sample_world_height(&terrains[0], 47_800.0, 96_000.0)
        .expect("Checkpoint Bravo inside Castle-00040009 terrain")
        / 100.0;
    eprintln!("checkpoint bravo: decoded BW y = {bw_y:.2}, seed BW y = 24..28");
    assert!(
        bw_y < 24.0 && bw_y > 24.0 - 15.0,
        "terrain at Checkpoint Bravo is {bw_y:.2}; expected just below the 24..28 seed band"
    );
}

#[test]
fn every_castle_chunk_terrain_decodes() {
    let dir = map_dir("Castle");
    if skip_if_missing(&dir, "every_castle_chunk_terrain_decodes") {
        return;
    }
    let mut total = TerrainStats::default();
    let mut chunks = 0usize;
    for chunk in enumerate_chunks(&dir).expect("enumerate_chunks") {
        chunks += 1;
        let (stats, _) = chunk_stats(&chunk);
        total.terrain_actors += stats.terrain_actors;
        total.parse_failures += stats.parse_failures;
        total.quads_total += stats.quads_total;
        total.quads_holed += stats.quads_holed;
        total.triangles_emitted += stats.triangles_emitted;
    }
    eprintln!("Castle: {chunks} chunks, {total:?}");
    assert_eq!(chunks, 144);
    assert_eq!(total.parse_failures, 0);
    assert_eq!(total.terrain_actors, 144, "one Terrain actor per chunk");
    assert_eq!(total.quads_total, 1_440_000);
    // Castle's terrain is almost hole-free: 555 of 1.44 M quads. The
    // castle interior is modelled with BSP/StaticMesh on top of an
    // unbroken hillside, unlike Castle_CellBlock where the building
    // footprint is punched clean out of the ground plane.
    assert_eq!(total.quads_holed, 555);
    assert_eq!(total.triangles_emitted, 2_878_890);
}
