//! Occluder inputs and accuracy tooling (NPC AI restoration NA27, #784).
//!
//! The runtime occluder (`cimmeria-occluder`) is a column grid of solid Y
//! spans built from the same collision triangles NavBuilder consumes. This
//! module is the extractor's side of it:
//!
//! - [`for_each_chunk`] walks a map's chunks and hands each chunk's
//!   triangles to a callback, already in **BigWorld metres** and split by
//!   source (terrain vs. StaticMesh + BSP), without writing an OBJ.
//! - [`exact`] is the ground truth: an exact segment-vs-triangle tracer.
//! - [`explorable`] finds the navmesh components a real entry point lands
//!   on, which the shipped build trims its coverage to.
//! - [`sweep`] samples point pairs on a `.nav`, and scores the occluder
//!   (and the navmesh ray) against the tracer.
//!
//! [`for_each_chunk`] runs the per-chunk sequence of
//! [`crate::extract_map_with_report`] (StaticMesh, then terrain, then BSP
//! with the hull-cap filter fed from that terrain) with the default
//! options. It is a second copy of that sequence, kept apart so the
//! occluder build never has to round-trip gigabytes of OBJ text on the
//! outdoor maps; `tests::the_walk_matches_extract_map` pins the two to the
//! same triangle count.

pub mod exact;
pub mod explorable;
pub mod sweep;

#[cfg(test)]
mod explorable_tests;
#[cfg(test)]
mod tests;

use std::path::Path;

use cimmeria_upk_objects::PackageIndex;

use crate::geometry::Triangle as UeTriangle;
use crate::{bsp, chunk_id, staticmesh, terrain, umap};

/// A triangle in BigWorld metres (`[x, y up, z]` per vertex).
pub type BwTriangle = [[f32; 3]; 3];

/// UE3 centimetres to BigWorld metres: `bw = (ue.y, ue.z, ue.x) / 100`.
/// The same mapping NavBuilder's `loadOBJ` applies to the OBJ's swizzled
/// order (`x = obj.z / 100, y = obj.y / 100, z = obj.x / 100`).
pub fn ue3_to_bw(v: [f32; 3]) -> [f32; 3] {
    [v[1] / 100.0, v[2] / 100.0, v[0] / 100.0]
}

fn tri_to_bw(t: &UeTriangle) -> BwTriangle {
    [ue3_to_bw(t[0]), ue3_to_bw(t[1]), ue3_to_bw(t[2])]
}

/// One chunk's collision triangles, BigWorld metres.
#[derive(Debug, Default)]
pub struct ChunkTriangles {
    pub chunk_id: u32,
    /// StaticMesh and BSP triangles.
    pub geometry: Vec<BwTriangle>,
    /// Terrain triangles.
    pub terrain: Vec<BwTriangle>,
}

/// Per-map totals from [`for_each_chunk`].
#[derive(Debug, Default, Clone, Copy)]
pub struct WalkStats {
    pub chunks: usize,
    pub staticmesh_triangles: usize,
    pub terrain_triangles: usize,
    pub bsp_triangles: usize,
    pub terrain_parse_failures: usize,
    pub bsp_models_failed: usize,
}

impl WalkStats {
    /// Every triangle handed to the callback.
    pub fn triangles(&self) -> usize {
        self.staticmesh_triangles + self.terrain_triangles + self.bsp_triangles
    }
}

/// Walk every chunk of `map_dir` and call `visit` with its triangles.
///
/// `index = None` is the extractor's degraded mode: terrain and BSP only.
/// `include_interp_actors` — default `false`, opt-in — is the same knob
/// `ExtractOptions` uses for the `.nav` side: doors, gates, lifts and
/// elevators are disproportionately `InterpActor` in this content, and
/// baking one's cooked (usually closed) pose into an `.occ` can block
/// line of sight through an opening a player can actually see and shoot
/// through. See `staticmesh::MESH_ACTOR_CLASSES`'s doc.
pub fn for_each_chunk(
    map_dir: &Path,
    index: Option<&PackageIndex>,
    include_interp_actors: bool,
    mut visit: impl FnMut(&ChunkTriangles),
) -> crate::Result<WalkStats> {
    let mut stats = WalkStats::default();
    let mut archetype_cache = staticmesh::ArchetypeCache::default();
    let mut chunks = umap::enumerate_chunks(map_dir)?;
    chunks.sort();
    for chunk_path in chunks {
        let id = chunk_id::ChunkId::from_umap_path(&chunk_path)?;
        let pkg = cimmeria_upk::Package::open(&chunk_path)?;
        let mut extraction = staticmesh::extract_chunk_from_package(
            &pkg,
            index,
            &mut archetype_cache,
            include_interp_actors,
        );
        let sm_end = extraction.soup.triangle_count();
        let terrain_stats = terrain::collect_terrain_triangles(&pkg, &mut extraction.soup);
        let terrain_end = extraction.soup.triangle_count();
        let ceiling =
            bsp::TerrainCeiling::from_triangles(extraction.soup.triangles_in(sm_end..terrain_end));
        let bsp_stats = bsp::collect_bsp_triangles(
            &pkg,
            &mut extraction.soup,
            bsp::BspOptions {
                terrain_ceiling: ceiling.as_ref(),
            },
        );
        let all_end = extraction.soup.triangle_count();
        stats.chunks += 1;
        stats.staticmesh_triangles += sm_end;
        stats.terrain_triangles += terrain_end - sm_end;
        stats.bsp_triangles += all_end - terrain_end;
        stats.terrain_parse_failures += terrain_stats.parse_failures;
        stats.bsp_models_failed += bsp_stats.models_failed;
        if all_end == 0 {
            continue;
        }
        let soup = &extraction.soup;
        let mut chunk = ChunkTriangles {
            chunk_id: id.raw(),
            geometry: soup.triangles_in(0..sm_end).iter().map(tri_to_bw).collect(),
            terrain: soup
                .triangles_in(sm_end..terrain_end)
                .iter()
                .map(tri_to_bw)
                .collect(),
        };
        chunk.geometry.extend(
            soup.triangles_in(terrain_end..all_end)
                .iter()
                .map(tri_to_bw),
        );
        // The StaticMesh walk's order is not stable between runs (hash-map
        // iteration), and a first-come rule in the terrain heightfield makes
        // the build order-sensitive. Sorting each chunk makes every build of
        // a map byte-identical.
        let key = |t: &BwTriangle| t.map(|v| v.map(f32::to_bits));
        chunk.geometry.sort_unstable_by_key(key);
        chunk.terrain.sort_unstable_by_key(key);
        visit(&chunk);
    }
    Ok(stats)
}
