//! Shared fixtures and geometry helpers for the BSP integration tests
//! (`bsp_castle_model_decode`, `bsp_castle_floor_evidence`,
//! `bsp_castle_hull_cap`). A plain module of the crate's one
//! integration-test binary, declared once in `tests/it/main.rs`; the
//! test modules pull it in with `use crate::bsp_support::*`.

use std::path::{Path, PathBuf};

use cimmeria_navmesh_extractor::bsp::{
    collect_bsp_models, BspOptions, HullCap, TerrainCeiling, EMIT_REVERSED,
};
use cimmeria_upk::Package;
use cimmeria_upk_objects::model::CollisionFilter;

/// The tile the playtest's two HIGH-confidence walkable points sit in.
pub const INTERIOR_TILE: &str = "Castle-000a0002.umap";
/// The tile holding the Level-5 comms room point.
pub const COMMS_TILE: &str = "Castle-00080002.umap";

// --- Measured baselines (Castle-000a0002.umap, persistent-level Model).
//
// These are the numbers this branch measured, not numbers copied from
// the RE finding. A change to the deserializer that moves any of them
// is a behaviour change that needs re-measuring, not a test to relax.

/// `Nodes.Num()` of the persistent-level `Model`.
pub const TILE_A2_LEVEL_NODES: usize = 399;
/// Fan triangles from those nodes, before any PolyFlags filtering.
pub const TILE_A2_LEVEL_TRIS_UNFILTERED: usize = 1098;

/// Locate a `CookedPC/Maps/<name>` directory by walking up from the
/// crate manifest until a `sgw/Stargate Worlds-QA/...` sibling appears.
/// Matches the probe `staticmesh_castle_cellblock.rs` uses so both
/// tests behave the same from a worktree.
pub fn map_dir(name: &str) -> PathBuf {
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

pub fn castle_dir() -> PathBuf {
    map_dir("Castle")
}

pub fn skip_if_missing(dir: &Path, what: &str) -> bool {
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
pub fn bw_to_ue(bw: [f32; 3]) -> [f32; 3] {
    [bw[2] * 100.0, bw[0] * 100.0, bw[1] * 100.0]
}

/// UE3 right-hand-rule normal of a triangle in its emitted order.
pub fn winding_normal(t: [[f32; 3]; 3]) -> [f32; 3] {
    let u = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
    let v = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

pub fn norm3(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// 2D point-in-triangle over the UE3 XY plane (the BW ground plane).
pub fn contains_xy(t: [[f32; 3]; 3], x: f32, y: f32) -> bool {
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
pub fn interp_z(t: [[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
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
pub struct WorldTri {
    pub tri: [[f32; 3]; 3],
    /// Authored surface normal (`Vectors[vNormal]`), world space.
    pub surf_normal: [f32; 3],
    pub poly_flags: u32,
}

/// Decode a chunk and return every emitted BSP triangle with its
/// authored surface normal, applying the **`PolyFlags` filter only**.
///
/// This is the raw decode: it deliberately skips the hull-cap filter so
/// the cap tests can measure "before". Production geometry goes through
/// [`world_triangles_shipped`].
pub fn world_triangles(chunk: &Path) -> Vec<WorldTri> {
    collect(chunk, false)
}

/// As [`world_triangles`], but with the hull-cap filter applied — i.e.
/// exactly the set `bsp::collect_bsp_triangles` pushes into the soup.
pub fn world_triangles_shipped(chunk: &Path) -> Vec<WorldTri> {
    collect(chunk, true)
}

fn collect(chunk: &Path, exclude_hull_caps: bool) -> Vec<WorldTri> {
    let pkg = Package::open(chunk).expect("open chunk");
    // Same ceiling `extract_map` builds: this chunk's own terrain.
    let ceiling = exclude_hull_caps.then(|| terrain_ceiling(&pkg)).flatten();
    let (instances, _stats) = collect_bsp_models(&pkg);
    let mut out = Vec::new();
    for inst in &instances {
        let t = inst.model.triangulate(CollisionFilter::default());
        let world_tris: Vec<[[f32; 3]; 3]> = t
            .triangles
            .iter()
            .map(|tri| {
                [
                    inst.to_world(tri[0]),
                    inst.to_world(tri[1]),
                    inst.to_world(tri[2]),
                ]
            })
            .collect();
        let cap = ceiling.as_ref().and(HullCap::detect(&world_tris));
        for (i, world_tri) in world_tris.iter().enumerate() {
            let surf_index = t.triangle_surf[i] as usize;
            let n_world = inst.normal_to_world(
                inst.model
                    .surf_normal(surf_index)
                    .unwrap_or([0.0, 0.0, 0.0]),
            );
            if let (Some(cap), Some(ceiling)) = (cap, ceiling.as_ref()) {
                if cap.is_buried_cap(world_tri, n_world, ceiling) {
                    continue;
                }
            }
            let mut world = *world_tri;
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

/// The options `extract_map` would pass for this chunk. Leaks the
/// ceiling so the returned value can borrow it for the caller's
/// lifetime — this is test code walking at most 144 chunks.
pub fn bsp_options(pkg: &Package) -> BspOptions<'static> {
    BspOptions {
        terrain_ceiling: terrain_ceiling(pkg).map(|c| &*Box::leak(Box::new(c))),
    }
}

/// Build the chunk's terrain ceiling exactly as `extract_map` does.
pub fn terrain_ceiling(pkg: &Package) -> Option<TerrainCeiling> {
    let mut soup = cimmeria_navmesh_extractor::geometry::TriangleSoup::new(None);
    cimmeria_navmesh_extractor::terrain::collect_terrain_triangles(pkg, &mut soup);
    TerrainCeiling::from_triangles(soup.triangles_in(0..soup.triangle_count()))
}

/// The 16 `Maps/Castle` chunks that carry a non-stub level `Model` —
/// identical to the set that carries `ModelComponent` exports. These
/// are the interior tiles; the other 128 chunks are outdoor terrain
/// with BSP stubs only.
pub const INTERIOR_TILES: [&str; 16] = [
    "00040009", "0004000a", "00050007", "00060003", "00070002", "00070003", "00070004", "00080002",
    "00080003", "00080004", "00090002", "00090003", "00090004", "000a0002", "000a0003", "000a0004",
];
