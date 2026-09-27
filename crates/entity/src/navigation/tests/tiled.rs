//! Tiled (`XRCT`) `.nav` loading on a synthetic two-tile mesh.
//!
//! CI has no client assets, so the fixture is written byte by byte: two
//! 10 m x 10 m tiles side by side along X, each one quad at y = 1, with the
//! shared edge marked as a Recast tile portal (`0x8000 | dir`) exactly as
//! `rcBuildPolyMesh` marks it when the tile was built with a border. Every
//! query that crosses x = 10 only works if both tiles went into the same
//! `dtNavMesh` and Detour linked the portals, which is the whole point of
//! the tiled layout.

use super::make_tmp_nav_path;
use super::*;

const CS: f32 = 0.5;
const CH: f32 = 0.2;
/// Tile side in cells and in metres.
const TILE_CELLS: u16 = 20;
const TILE_M: f32 = TILE_CELLS as f32 * CS;
/// Floor height in cells (5 * 0.2 = 1.0 m).
const FLOOR_CELLS: u16 = 5;
const FLOOR_Y: f32 = FLOOR_CELLS as f32 * CH;

const NULL_IDX: u16 = 0xffff;
/// Recast portal markers (`RecastMesh.cpp`, `mesh.borderSize > 0`).
const PORTAL_X_MINUS: u16 = 0x8000;
const PORTAL_X_PLUS: u16 = 0x8000 | 2;

fn put_u32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_i32(b: &mut Vec<u8>, v: i32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_f32(b: &mut Vec<u8>, v: f32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}

/// One tile: a single quad covering the whole tile, `neis` giving the
/// neighbour slot of its four edges (v0-v1 at x=0, v1-v2 at z=max, v2-v3
/// at x=max, v3-v0 at z=0). The vertices go round in Recast's order,
/// clockwise seen from +Y with X right and Z up: Detour's segment-polygon
/// test (`dtIntersectSegmentPoly2D`, behind raycast and line of sight)
/// depends on it. No detail section: Detour triangulates the polygon
/// itself.
fn put_tile(b: &mut Vec<u8>, tile_x: i32, tile_y: i32, neis: [u16; 4]) {
    put_i32(b, tile_x);
    put_i32(b, tile_y);
    let nvp = 6u32;
    put_u32(b, 4); // nverts
    put_u32(b, 1); // npolys
    put_u32(b, nvp);
    put_u32(b, 5); // border_size (walkableRadius + 3); informational
    put_f32(b, CS);
    put_f32(b, CH);
    let x0 = tile_x as f32 * TILE_M;
    let z0 = tile_y as f32 * TILE_M;
    for v in [x0, 0.0, z0, x0 + TILE_M, 2.0, z0 + TILE_M] {
        put_f32(b, v);
    }
    for [x, z] in [
        [0, 0],
        [0, TILE_CELLS],
        [TILE_CELLS, TILE_CELLS],
        [TILE_CELLS, 0],
    ] {
        put_u16(b, x);
        put_u16(b, FLOOR_CELLS);
        put_u16(b, z);
    }
    for v in [0, 1, 2, 3, NULL_IDX, NULL_IDX] {
        put_u16(b, v);
    }
    for v in [neis[0], neis[1], neis[2], neis[3], NULL_IDX, NULL_IDX] {
        put_u16(b, v);
    }
    put_u16(b, 0); // regs
    put_u16(b, 1); // flags: walkable
    b.push(63); // areas: RC_WALKABLE_AREA
    put_u32(b, 0); // detail_nmeshes
    put_u32(b, 0); // detail_nverts
    put_u32(b, 0); // detail_ntris
}

/// Header for `ntiles` tiles at origin (0, 0, 0).
fn header(version: u32, ntiles: u32, max_tile_polys: u32) -> Vec<u8> {
    let mut b = b"XRCT".to_vec();
    put_u32(&mut b, version);
    for v in [1.8, 0.6, 0.6] {
        put_f32(&mut b, v);
    }
    for v in [0.0, 0.0, 0.0] {
        put_f32(&mut b, v);
    }
    put_f32(&mut b, TILE_M);
    put_f32(&mut b, TILE_M);
    put_u32(&mut b, ntiles);
    put_u32(&mut b, max_tile_polys);
    b
}

/// The two-tile fixture. `linked = false` writes the shared edge as a
/// plain border on both sides, which is what a loader that dropped the
/// portal markers (or a builder that built tiles with no border) would
/// hand Detour.
fn two_tiles(linked: bool) -> Vec<u8> {
    let (east, west) = if linked {
        (PORTAL_X_PLUS, PORTAL_X_MINUS)
    } else {
        (NULL_IDX, NULL_IDX)
    };
    let mut b = header(1, 2, 1);
    put_tile(&mut b, 0, 0, [NULL_IDX, NULL_IDX, east, NULL_IDX]);
    put_tile(&mut b, 1, 0, [west, NULL_IDX, NULL_IDX, NULL_IDX]);
    b
}

fn write(bytes: &[u8], suffix: &str) -> std::path::PathBuf {
    let path = make_tmp_nav_path(suffix);
    std::fs::write(&path, bytes).expect("write tmp nav");
    path
}

fn load(bytes: &[u8], suffix: &str) -> cimmeria_common::Result<NavMesh> {
    let path = write(bytes, suffix);
    let mesh = NavMesh::load(&path);
    let _ = std::fs::remove_file(&path);
    mesh
}

fn west_point() -> Vector3 {
    Vector3::new(2.0, FLOOR_Y, 5.0)
}
fn east_point() -> Vector3 {
    Vector3::new(18.0, FLOOR_Y, 5.0)
}

#[test]
fn a_tiled_file_loads_every_tile_into_one_mesh() {
    let bytes = two_tiles(true);
    let mesh = load(&bytes, "tiled_ok").expect("two-tile mesh loads");
    let fp = mesh.fingerprint();
    assert_eq!(fp.tiles, 2);
    assert_eq!(fp.npolys, 2);
    assert_eq!(fp.nverts, 8);
    assert_eq!(fp.file_bytes, bytes.len() as u64);
    assert_eq!(mesh.poly_count(), 2);
    assert_eq!(mesh.bmin, [0.0, 0.0, 0.0]);
    assert_eq!(mesh.bmax, [2.0 * TILE_M, 2.0, TILE_M]);
    // A point in the second tile only resolves if that tile was added.
    assert!(mesh.is_point_valid(&east_point()));
    assert!(mesh.is_point_valid(&west_point()));
}

#[test]
fn find_path_crosses_the_tile_border() {
    let mesh = load(&two_tiles(true), "tiled_path").unwrap();
    let outcome = mesh.find_path(&west_point(), &east_point());
    assert_eq!(outcome.status, PathStatus::Ok, "{outcome:?}");
    let last = *outcome.waypoints.last().expect("waypoints");
    assert!(
        (last.x - 18.0).abs() < 0.01 && (last.z - 5.0).abs() < 0.01,
        "{last:?}"
    );
}

/// The regression guard for the portal links: with the shared edge
/// written as a border, the two tiles are islands and the same query is
/// only a partial corridor. If this and the test above ever agree, the
/// fixture is not exercising cross-tile linking at all.
#[test]
fn unlinked_tiles_are_islands() {
    let mesh = load(&two_tiles(false), "tiled_islands").unwrap();
    assert!(mesh.is_point_valid(&east_point()));
    let outcome = mesh.find_path(&west_point(), &east_point());
    assert_eq!(outcome.status, PathStatus::Partial, "{outcome:?}");
}

#[test]
fn surface_queries_work_across_the_border() {
    let mesh = load(&two_tiles(true), "tiled_surface").unwrap();

    // Height in the second tile.
    let h = mesh
        .get_height_near(15.0, FLOOR_Y + 1.0, 5.0)
        .expect("height");
    assert!((h - FLOOR_Y).abs() < 0.05, "{h}");

    // A clear sight line from one tile into the other.
    let eye = Vector3::new(2.0, FLOOR_Y + 1.5, 5.0);
    let target = Vector3::new(18.0, FLOOR_Y + 1.5, 5.0);
    assert_eq!(mesh.line_of_sight(&eye, &target), LineOfSight::Clear);

    // A slide that starts in tile 0 ends in tile 1, stopped by tile 1's
    // far edge rather than by the seam.
    let end = mesh
        .move_along_surface(&west_point(), &Vector3::new(25.0, FLOOR_Y, 5.0))
        .expect("slide");
    assert!(end.x > 19.5 && end.x <= 20.0 + 1e-3, "{end:?}");

    // Recovery onto the second tile from a point just past its far edge.
    let p = mesh
        .nearest_point_within(&Vector3::new(20.5, FLOOR_Y, 5.0), 1.0, 1.0)
        .expect("recovered");
    assert!(p.x > 19.9 && p.x <= 20.0 + 1e-3, "{p:?}");

    let verdict = mesh.diagnose_point(&Vector3::new(15.0, FLOOR_Y + 0.5, 5.0));
    assert!(verdict.valid, "{verdict:?}");
}

fn assert_rejected(bytes: &[u8], suffix: &str, want_field: &str) {
    match load(bytes, suffix) {
        Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { field, .. }) => {
            assert_eq!(field, want_field)
        }
        Err(other) => panic!("expected NavHeaderOutOfRange({want_field}), got {other:?}"),
        Ok(_) => panic!("expected NavHeaderOutOfRange({want_field}), the file loaded"),
    }
}

#[test]
fn an_unknown_version_is_rejected() {
    let mut bytes = two_tiles(true);
    bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
    assert_rejected(&bytes, "tiled_version", "version");
}

#[test]
fn a_tile_count_over_the_cap_is_rejected_before_any_tile_is_read() {
    let mut b = header(1, u32::MAX, 1);
    put_tile(&mut b, 0, 0, [NULL_IDX; 4]);
    assert_rejected(&b, "tiled_ntiles", "ntiles");
}

#[test]
fn zero_tiles_is_rejected() {
    assert_rejected(&header(1, 0, 1), "tiled_zero", "ntiles");
}

/// 2^16 tiles need 16 bits and 2^7 polygons 7: 23 > 22, which
/// `dtNavMesh::init` would refuse. Rejected by the loader with a named
/// field instead of a generic "failed to create".
#[test]
fn a_poly_ref_budget_detour_would_refuse_is_rejected() {
    assert_rejected(&header(1, 1 << 16, 128), "tiled_bits", "ntiles");
}

#[test]
fn a_tile_with_more_polygons_than_the_header_allows_is_rejected() {
    let mut b = header(1, 1, 1);
    put_tile(&mut b, 0, 0, [NULL_IDX; 4]);
    // npolys is the second u32 of the block: header (48 bytes) + tile x/y
    // (8) + nverts (4).
    let at = 48 + 8 + 4;
    b[at..at + 4].copy_from_slice(&2u32.to_le_bytes());
    assert_rejected(&b, "tiled_npolys", "npolys");
}

#[test]
fn a_hostile_tile_vertex_count_is_rejected_by_the_tile_cap() {
    let mut b = header(1, 1, 1);
    put_tile(&mut b, 0, 0, [NULL_IDX; 4]);
    let at = 48 + 8;
    // Under the single-mesh cap (1,000,000) but not a u16 index space.
    b[at..at + 4].copy_from_slice(&70_000u32.to_le_bytes());
    assert_rejected(&b, "tiled_nverts", "nverts");
}

#[test]
fn two_tiles_at_the_same_position_fail_to_load() {
    let mut b = header(1, 2, 1);
    put_tile(&mut b, 0, 0, [NULL_IDX; 4]);
    put_tile(&mut b, 0, 0, [NULL_IDX; 4]);
    assert!(load(&b, "tiled_dup").is_err());
}

#[test]
fn a_truncated_tiled_file_fails_to_load() {
    let bytes = two_tiles(true);
    assert!(load(&bytes[..bytes.len() - 3], "tiled_trunc").is_err());
}
