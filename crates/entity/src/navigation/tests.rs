//! Integration tests for [`NavMesh`] loading and queries, plus the
//! hostile-`.nav`-header regression guards that exercise the loader's
//! per-section bounds checks. The pure-helper widening/log guards for
//! the XRC reader live alongside the helpers in [`super::xrc`].

use super::*;

#[test]
fn load_castle_cellblock_nav() {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        // Skip test if data file not present (CI)
        return;
    }
    let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");
    assert!(mesh.agent_height > 0.0);
    assert!(mesh.agent_radius > 0.0);

    // The guard spawn position should be on the navmesh
    let guard_pos = Vector3::new(-289.465, 68.542, -154.276);
    assert!(
        mesh.is_point_valid(&guard_pos),
        "Guard spawn position should be on navmesh"
    );
}

#[test]
fn load_and_pathfind_castle_cellblock() {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return;
    }
    let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

    // Guard patrol: find path between two known-good positions
    let start = Vector3::new(-289.465, 68.542, -154.276);
    let end = Vector3::new(-280.0, 68.0, -150.0);
    let path_result = mesh.find_path(&start, &end);
    assert!(
        path_result.is_some(),
        "Should find path between nearby points on navmesh"
    );
    let waypoints = path_result.unwrap();
    assert!(
        waypoints.len() >= 2,
        "Path should have at least start and end"
    );
}

#[test]
fn load_and_raycast_castle_cellblock() {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return;
    }
    let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

    let start = Vector3::new(-289.465, 68.542, -154.276);
    let end = Vector3::new(-280.0, 68.0, -150.0);

    // Nearby points should have line of sight
    let has_los = mesh.raycast(&start, &end);
    // We can't assert this is true without knowing the mesh geometry,
    // but at least verify it doesn't crash
    tracing::info!("Raycast result: {has_los}");
}

/// Regression guard: a start position raised above the navmesh
/// (a "flyer" NPC hovering ≥0.5u over the walkable surface) must
/// still resolve to a valid polygon for the raycast. Pre-fix, the
/// tight `START_EXTENTS = [0.5, 0.5, 0.5]` lookup failed for any
/// hover height beyond ~half a unit and `raycast` returned
/// `false` without ever shooting the ray — surfacing as
/// stationary flyer NPCs (e.g., the Ambernol drone, body_set
/// `BS_MOB_DroneFlyer`) never firing their abilities because
/// `npc_ai_fight`'s `!has_los` branch silent-skipped every
/// tick.
///
/// Without the navmesh fixture (CI), the test self-skips per the
/// repo's standard `if !path.exists()` pattern.
#[test]
fn raycast_with_off_mesh_start_projects_to_polygon() {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return;
    }
    let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

    // Pick a known-on-navmesh ground reference (same one the
    // sibling tests use) then lift the start up by 2.5 units —
    // well past `START_EXTENTS = 0.5` but within `DEST_EXTENTS = 3.0`.
    // A flyer drone at body_set hover offset typically sits 0.2–
    // 1.5u above the floor; 2.5u is a deliberately pessimistic
    // case to prove the wider fallback also catches it.
    let ground = Vector3::new(-289.465, 68.542, -154.276);
    let off_mesh = Vector3::new(ground.x, ground.y + 2.5, ground.z);
    let nearby_target = Vector3::new(-280.0, 68.0, -150.0);

    // Pre-fix bug: this returns `false` even though the ground point
    // RIGHT BELOW `off_mesh` has clear LoS to `nearby_target`. The
    // projection-to-polygon retry recovers the correct result.
    let los_off_mesh = mesh.raycast(&off_mesh, &nearby_target);
    let los_on_mesh = mesh.raycast(&ground, &nearby_target);
    assert_eq!(
        los_off_mesh, los_on_mesh,
        "off-mesh start ({:?}) must produce the same LoS result as the \
         on-mesh point directly below it ({:?}) — the projection-to-polygon \
         retry was added to recover from the START_EXTENTS=0.5 lookup \
         failing on flyer NPC positions",
        off_mesh, ground
    );
}

/// Symmetric regression for the end-projection fallback. The end
/// point (the player being shot at) must also be projected to its
/// nearest polygon when off-mesh — Detour's raycast halts at the
/// mesh boundary when `end_pos` lies outside any walkable poly,
/// returning `t < 1.0` and surfacing as `has_los = false` even
/// when no real geometry blocks the shot. Without the fix,
/// stationary NPCs see this as "no LoS" and silently hold fire
/// (`npc_ai::stationary_holds`).
///
/// Observed instance: castle_cellblock NPC 100143 vs lomiada at
/// 2026-06-04 11:04:44–46 UTC — `dist=12.7–13.0m`, `max_range=30m`,
/// `in_range=true`, `has_los=false`, two ticks of `stationary_holds`
/// with the player visually in clear sight (the navmesh just
/// didn't cover the player's exact tile).
///
/// Reverting `raycast`'s end-projection block trips this test by
/// returning `false` (target off-mesh halt) instead of matching
/// the on-mesh-target baseline.
#[test]
fn raycast_with_off_mesh_end_projects_to_polygon() {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return;
    }
    let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

    // Same ground reference used by the sibling raycast tests, then
    // lift the *target* (end) up 2.5 units — past `START_EXTENTS = 0.5`
    // but within `DEST_EXTENTS = 3.0`. Symmetric to the off-mesh-start
    // case; without end-projection the call returns `false` because
    // Detour halts at the mesh boundary near the target.
    let start = Vector3::new(-289.465, 68.542, -154.276);
    let on_mesh_target = Vector3::new(-280.0, 68.0, -150.0);
    let off_mesh_target = Vector3::new(on_mesh_target.x, on_mesh_target.y + 2.5, on_mesh_target.z);

    let los_to_off_mesh = mesh.raycast(&start, &off_mesh_target);
    let los_to_on_mesh = mesh.raycast(&start, &on_mesh_target);
    assert_eq!(
        los_to_off_mesh, los_to_on_mesh,
        "off-mesh end ({:?}) must produce the same LoS result as the \
         on-mesh point directly below it ({:?}) — the end-side \
         projection-to-polygon was added to recover from Detour halting \
         at the mesh boundary when the target tile isn't covered by \
         the navmesh (castle_cellblock NPC 100143 vs lomiada, \
         2026-06-04 11:04 UTC).",
        off_mesh_target, on_mesh_target
    );
}

// ── Hostile-input regression guards ────────────────────────────────
//
// Each of the five tests below synthesises a `.nav` header in which
// exactly one count field is poisoned with `0xFFFFFFFF` and the rest
// are zero, writes it to a temp file, and asserts that `NavMesh::load`
// rejects it with `CimmeriaError::NavHeaderOutOfRange` naming the
// offending field. Reverting any single bounds check in `load`
// (e.g., dropping the `check_count` wrapping `nverts`) causes the
// matching test to fail by trying to allocate / read past EOF instead.
//
// Layout written below (mirrors `NavMesh::load` exactly through the
// first allocation site of each section; later sections need only
// enough header bytes for the cap to trip before the alloc):
//
//   agent_height/climb/radius   3 × f32     (12 bytes)
//   nverts/npolys/nvp/border    4 × u32     (16 bytes)
//   cs/ch                       2 × f32     (8  bytes)
//   bmin/bmax                   6 × f32     (24 bytes)
//   ...sections beyond this only need their leading count headers...

/// Construct a unique temp-file path so concurrent test runs don't
/// clobber each other. We deliberately don't use the `tempfile`
/// crate to keep `cimmeria-entity` dep-free at this layer.
///
/// Suffix combines pid + thread id + nanosecond timestamp — matching
/// the navmesh-extractor's `tempdir()` helper. Cargo's default test
/// runner parallelises by default, so naming the file by pid+nanos
/// alone would race two threads into the same path on fast hardware
/// where multiple tests start within the same nanosecond.
fn make_tmp_nav_path(suffix: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let tid = std::thread::current().id();
    let mut p = std::env::temp_dir();
    p.push(format!(
        "cimmeria_navmesh_load_{pid}_{tid:?}_{nanos}_{suffix}.nav"
    ));
    p
}

/// Build a minimal header with all counts zero, then let the caller
/// overwrite a single 4-byte slot with the hostile value. Returns
/// a byte vector long enough to reach the detail-section header so
/// any of the five caps can trip without truncated-read confusion.
fn synthesise_header() -> Vec<u8> {
    let mut buf = Vec::new();
    // agent_height/climb/radius — values don't matter, just non-NaN.
    buf.extend_from_slice(&0.6_f32.to_le_bytes());
    buf.extend_from_slice(&0.9_f32.to_le_bytes());
    buf.extend_from_slice(&0.6_f32.to_le_bytes());
    // nverts/npolys/nvp/border_size — start all zero so the test can
    // overwrite exactly one.
    buf.extend_from_slice(&0_u32.to_le_bytes());
    buf.extend_from_slice(&0_u32.to_le_bytes());
    buf.extend_from_slice(&6_u32.to_le_bytes()); // nvp can't be 0 if the test wants
                                                 // to reach detail headers via valid `polys`
                                                 // sizing — but with npolys=0 the alloc is
                                                 // zero-sized regardless. Use a sane default.
    buf.extend_from_slice(&0_u32.to_le_bytes()); // border_size
                                                 // cs/ch
    buf.extend_from_slice(&0.3_f32.to_le_bytes());
    buf.extend_from_slice(&0.2_f32.to_le_bytes());
    // bmin/bmax — six f32s.
    for _ in 0..6 {
        buf.extend_from_slice(&0.0_f32.to_le_bytes());
    }
    // verts/polys/regs/flags/areas are all zero-sized when counts are
    // zero, so the read cursor lands directly on the detail header.
    // detail_nmeshes/detail_nverts/detail_ntris — three u32 slots.
    buf.extend_from_slice(&0_u32.to_le_bytes());
    buf.extend_from_slice(&0_u32.to_le_bytes());
    buf.extend_from_slice(&0_u32.to_le_bytes());
    buf
}

/// Byte offsets of each header count field inside `synthesise_header()`'s
/// output. Encoded as constants so a layout change here forces a
/// matching update in the tests below.
// Byte budget: 3 × f32 (12) + 4 × u32 (16) + 2 × f32 (8) + 6 × f32 (24)
// = 60 bytes before the detail header. detail_* occupies bytes 60..72.
const OFFSET_NVERTS: usize = 12;
const OFFSET_NPOLYS: usize = 16;
const OFFSET_NVP: usize = 20;
const OFFSET_DETAIL_NMESHES: usize = 60;
const OFFSET_DETAIL_NVERTS: usize = 64;
const OFFSET_DETAIL_NTRIS: usize = 68;

fn write_hostile_at(field_offset: usize) -> std::path::PathBuf {
    use std::io::Write;
    let mut buf = synthesise_header();
    let bytes = 0xFFFF_FFFF_u32.to_le_bytes();
    buf[field_offset..field_offset + 4].copy_from_slice(&bytes);
    let path = make_tmp_nav_path(&format!("hostile_{field_offset}"));
    let mut f = std::fs::File::create(&path).expect("create tmp nav");
    f.write_all(&buf).expect("write tmp nav");
    path
}

fn assert_hostile_field(path: &std::path::Path, expected_field: &str) {
    let result = NavMesh::load(path);
    // Clean up the temp file before asserting so a failed assertion
    // doesn't leave litter under $TMP.
    let _ = std::fs::remove_file(path);
    match result {
        Err(cimmeria_common::CimmeriaError::NavHeaderOutOfRange { field, .. }) => {
            assert_eq!(
                field, expected_field,
                "wrong field flagged in NavHeaderOutOfRange"
            );
        }
        other => panic!(
            "expected NavHeaderOutOfRange({expected_field}), got {other:?}\n\
             If this regressed: the bounds check on `{expected_field}` in \
             NavMesh::load was reverted; restore the check_count call."
        ),
    }
}

/// Hostile `nverts` must trip `MAX_NVERTS` before `vec![0u16; (nverts * 3)]`
/// wraps to a 3-element Vec (or the unwrapped 12 GB allocation crashes
/// the server).
#[test]
fn navmesh_load_rejects_oversized_nverts() {
    let path = write_hostile_at(OFFSET_NVERTS);
    assert_hostile_field(&path, "nverts");
}

/// Hostile `npolys` must trip `MAX_NPOLYS` before the
/// `npolys * nvp * 2` multiplication can overflow.
#[test]
fn navmesh_load_rejects_oversized_npolys() {
    let path = write_hostile_at(OFFSET_NPOLYS);
    assert_hostile_field(&path, "npolys");
}

/// Hostile `nvp` must trip `MAX_NVP = 64`. Without this check, a `.nav`
/// with `nvp = 0xFFFFFFFF` and `npolys = 1` would still pass through
/// the `npolys` and `nverts` caps but the `polys_len` multiplication
/// would overflow u32 (or even u64 if we hadn't widened first).
#[test]
fn navmesh_load_rejects_oversized_nvp() {
    let path = write_hostile_at(OFFSET_NVP);
    assert_hostile_field(&path, "nvp");
}

/// Hostile `detail_nmeshes` must trip the detail-section cap.
#[test]
fn navmesh_load_rejects_oversized_detail_nmeshes() {
    let path = write_hostile_at(OFFSET_DETAIL_NMESHES);
    assert_hostile_field(&path, "detail_nmeshes");
}

/// Hostile `detail_nverts` must trip the detail-section cap.
#[test]
fn navmesh_load_rejects_oversized_detail_nverts() {
    let path = write_hostile_at(OFFSET_DETAIL_NVERTS);
    assert_hostile_field(&path, "detail_nverts");
}

/// Hostile `detail_ntris` must trip the detail-section cap.
#[test]
fn navmesh_load_rejects_oversized_detail_ntris() {
    let path = write_hostile_at(OFFSET_DETAIL_NTRIS);
    assert_hostile_field(&path, "detail_ntris");
}

#[test]
fn load_and_height_query() {
    let path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !path.exists() {
        return;
    }
    let mesh = NavMesh::load(path).expect("Failed to load castle_cellblock.nav");

    // Query height at the guard position XZ.
    // The navmesh is in Recast/BigWorld coordinate space — Y values won't
    // match UE3 game coordinates directly. Just verify we get a result and
    // that it's a finite number within the mesh bounds.
    let height = mesh.get_height_at(-289.465, -154.276);
    assert!(
        height.is_some(),
        "Should find height at guard spawn XZ position"
    );
    let h = height.unwrap();
    assert!(h.is_finite(), "Height should be finite, got {h}");
    // Height should be between the mesh's Y bounds (with some tolerance)
    assert!(
        h >= mesh.bmin[1] - 1.0 && h <= mesh.bmax[1] + 1.0,
        "Height {h} should be within mesh Y bounds [{}, {}]",
        mesh.bmin[1],
        mesh.bmax[1]
    );
}
