//! Phase 1.3 — `Terrain` actor extraction.
//!
//! For each `Terrain` export in a chunk `.umap`:
//!
//! 1. Decode the heightmap and per-vertex info flags via
//!    [`cimmeria_upk_objects::deserialize_terrain`].
//! 2. Build the actor transform from the same `Location` / `Rotation` /
//!    `DrawScale` / `DrawScale3D` properties the StaticMesh walker uses.
//!    A terrain's `Location` is **absolute world space**, not
//!    chunk-relative, so no per-chunk offset is applied.
//! 3. Emit two triangles per visible patch at full heightmap resolution.
//!
//! # Local vertex space
//!
//! [`Terrain::local_vertex`] returns UE3's pre-scale terrain space: one
//! unit per patch in X/Y and `(h - 32768) / 128` in Z. Feeding that
//! through [`ActorTransform::apply`] reproduces UE3's terrain
//! LocalToWorld exactly, including the `DrawScale3D` default of
//! `(100, 100, 100)` that makes a patch 100 cm wide. Every shipped SGW
//! map decodes to 100 cm patches under that default — see
//! `SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D` for the evidence.
//!
//! # Winding
//!
//! NavBuilder marks a triangle walkable when the UE3 right-hand-rule
//! normal of the emitted index order points **down**
//! (`N_recast.y = -n_ue3.z`, and Recast wants `N_recast.y > 0`). Terrain
//! geometry is synthesised here rather than read from an index buffer,
//! so the quad split is chosen to satisfy that and pinned by
//! [`tests::flat_patch_normals_point_down_in_ue3`].
//!
//! # Group naming
//!
//! NavBuilder's `Mesh::loadOBJ` silently drops any OBJ group whose name
//! starts with `Terrain_` (`mesh.cpp:88-96`). This module pushes into
//! whatever soup the caller supplies and never names a group itself —
//! the orchestrator tags chunk soups `Chunk_<id>`. If a caller ever does
//! hand us a `Terrain_*` soup we log an error rather than emit geometry
//! that would be thrown away downstream.

use cimmeria_upk::Package;
use cimmeria_upk_objects::{deserialize_terrain, Terrain};

use crate::geometry::TriangleSoup;
use crate::transform::ActorTransform;

/// `Terrain` is an `AActor` subclass: 32-byte native header before the
/// tagged-property stream. (`StaticMesh` uses 4, components use 8.)
const TERRAIN_CLASS_NAME: &str = "Terrain";

/// Per-package terrain extraction counters.
///
/// Returned by [`collect_terrain_triangles`] so the orchestrator can log
/// coverage without re-walking the soup.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TerrainStats {
    /// `Terrain`-class exports found in the package.
    pub terrain_actors: usize,
    /// Exports whose payload failed to decode. Non-zero means missing
    /// ground geometry — never silently acceptable.
    pub parse_failures: usize,
    /// Patch quads across every decoded terrain, holes included.
    pub quads_total: usize,
    /// Quads suppressed by `TID_Visibility_Off` — the footprints where
    /// buildings and tunnels cut through the terrain.
    pub quads_holed: usize,
    /// Triangles pushed into the soup (`2 * (quads_total - quads_holed)`).
    pub triangles_emitted: usize,
}

/// Walk every `Terrain` export in `pkg` and push its world-space
/// collision triangles into `soup`.
///
/// Decode failures are counted and logged rather than aborting the
/// chunk: one bad terrain actor should not cost the other 24 in a
/// Castle_CellBlock chunk. Callers that need zero-tolerance should
/// assert `stats.parse_failures == 0`.
pub fn collect_terrain_triangles(pkg: &Package, soup: &mut TriangleSoup) -> TerrainStats {
    if let Some(group) = &soup.group {
        if group.starts_with("Terrain_") {
            tracing::error!(
                group,
                "terrain soup group starts with `Terrain_`; NavBuilder's loadOBJ \
                 skips those groups and every triangle pushed here would be discarded"
            );
        }
    }

    let mut stats = TerrainStats::default();

    for export in &pkg.exports {
        if pkg.export_class_name(export) != TERRAIN_CLASS_NAME {
            continue;
        }
        stats.terrain_actors += 1;

        let data = match pkg.read_export_data(export) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(
                    terrain = %export.object_name,
                    error = %e,
                    "could not read Terrain export data"
                );
                stats.parse_failures += 1;
                continue;
            }
        };

        let terrain = match deserialize_terrain(&data, &pkg.names) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    terrain = %export.object_name,
                    error = %e,
                    "could not deserialize Terrain export"
                );
                stats.parse_failures += 1;
                continue;
            }
        };

        let xf = terrain_transform(&terrain);
        let emitted = push_terrain_triangles(&terrain, &xf, soup);

        stats.quads_total += terrain.quad_count();
        stats.quads_holed += terrain.quad_count() - emitted / 2;
        stats.triangles_emitted += emitted;
    }

    stats
}

/// Build the actor transform a decoded terrain should be rendered with.
pub fn terrain_transform(terrain: &Terrain) -> ActorTransform {
    ActorTransform {
        location: terrain.location,
        rotation: terrain.rotation,
        draw_scale: terrain.draw_scale,
        draw_scale_3d: terrain.draw_scale_3d,
    }
}

/// Triangulate one decoded terrain into `soup`, returning the number of
/// triangles pushed.
///
/// Quad `(i, j)` spans grid vertices `(i, j)`–`(i+1, j+1)`. It is split
/// along the `v00`–`v11` diagonal and emitted as `[v00, v11, v10]` and
/// `[v00, v01, v11]`, which puts the UE3 right-hand-rule normal on `-Z`
/// for a level patch — the orientation NavBuilder treats as walkable
/// ground.
///
/// Quads flagged `TID_Visibility_Off` are skipped entirely. Those are
/// the building and tunnel footprints; emitting them would pave over
/// interiors with a false ground plane.
pub fn push_terrain_triangles(
    terrain: &Terrain,
    xf: &ActorTransform,
    soup: &mut TriangleSoup,
) -> usize {
    let mut emitted = 0usize;

    for j in 0..terrain.num_patches_y {
        for i in 0..terrain.num_patches_x {
            if !terrain.quad_visible(i, j) {
                continue;
            }
            // Every index here is < num_vertices_*, guaranteed by the
            // parser's `num_vertices == num_patches + 1` invariant, so
            // `local_vertex` cannot return None. Be defensive anyway:
            // a future relaxation of that invariant should drop the
            // quad, not panic mid-extraction.
            let (Some(v00), Some(v10), Some(v11), Some(v01)) = (
                terrain.local_vertex(i, j),
                terrain.local_vertex(i + 1, j),
                terrain.local_vertex(i + 1, j + 1),
                terrain.local_vertex(i, j + 1),
            ) else {
                tracing::warn!(i, j, "terrain quad references an out-of-grid vertex");
                continue;
            };

            let w00 = xf.apply(v00);
            let w10 = xf.apply(v10);
            let w11 = xf.apply(v11);
            let w01 = xf.apply(v01);

            soup.push([w00, w11, w10]);
            soup.push([w00, w01, w11]);
            emitted += 2;
        }
    }

    emitted
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_upk_objects::{SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D, TERRAIN_ZSCALE};

    const NEUTRAL: u16 = 0x8000;

    /// A `w`×`h`-patch terrain with the supplied raw heights and info
    /// bytes, placed at `location` with the SGW class-default scale.
    fn terrain(w: u32, h: u32, heights: Vec<u16>, info: Vec<u8>, location: [f32; 3]) -> Terrain {
        Terrain {
            num_patches_x: w,
            num_patches_y: h,
            num_vertices_x: w + 1,
            num_vertices_y: h + 1,
            num_sections_x: 1,
            num_sections_y: 1,
            max_tesselation_level: 1,
            location,
            rotation: [0; 3],
            draw_scale: 1.0,
            draw_scale_3d: SGW_TERRAIN_DEFAULT_DRAW_SCALE_3D,
            heights,
            info_data: info,
            alpha_x_size: 0,
            alpha_y_size: 0,
            weighted_texture_map_count: 0,
            weight_map_texture_count: 0,
            lighting_trailer_bytes: 0,
        }
    }

    fn flat(w: u32, h: u32) -> Terrain {
        let n = ((w + 1) * (h + 1)) as usize;
        terrain(w, h, vec![NEUTRAL; n], vec![0; n], [0.0; 3])
    }

    /// UE3 right-hand-rule face normal of a triangle.
    fn normal(t: [[f32; 3]; 3]) -> [f32; 3] {
        let e1 = [t[1][0] - t[0][0], t[1][1] - t[0][1], t[1][2] - t[0][2]];
        let e2 = [t[2][0] - t[0][0], t[2][1] - t[0][1], t[2][2] - t[0][2]];
        [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ]
    }

    fn tri(soup: &TriangleSoup, n: usize) -> [[f32; 3]; 3] {
        let f = soup.faces[n];
        [
            soup.vertices[f[0] as usize - 1],
            soup.vertices[f[1] as usize - 1],
            soup.vertices[f[2] as usize - 1],
        ]
    }

    #[test]
    fn flat_patch_normals_point_down_in_ue3() {
        // The winding pin. NavBuilder computes N_recast.y = -n_ue3.z and
        // only accepts a triangle as walkable ground when that is
        // positive, so every emitted terrain triangle must have
        // n_ue3.z < 0. Flipping either quad's index order breaks this.
        let t = flat(4, 4);
        let mut soup = TriangleSoup::new(None);
        let emitted = push_terrain_triangles(&t, &terrain_transform(&t), &mut soup);
        assert_eq!(emitted, 32);
        for k in 0..soup.faces.len() {
            let n = normal(tri(&soup, k));
            assert!(
                n[2] < 0.0,
                "triangle {k} normal {n:?} does not point down in UE3 space"
            );
        }
    }

    #[test]
    fn a_two_by_two_terrain_emits_eight_triangles_on_a_100cm_grid() {
        let t = flat(2, 2);
        let mut soup = TriangleSoup::new(None);
        push_terrain_triangles(&t, &terrain_transform(&t), &mut soup);
        assert_eq!(soup.triangle_count(), 8);

        // Patch spacing comes from the class-default DrawScale3D, so a
        // 2-patch terrain spans exactly 200 cm.
        let xs: Vec<f32> = soup.vertices.iter().map(|v| v[0]).collect();
        let ys: Vec<f32> = soup.vertices.iter().map(|v| v[1]).collect();
        assert_eq!(xs.iter().cloned().fold(f32::MAX, f32::min), 0.0);
        assert_eq!(xs.iter().cloned().fold(f32::MIN, f32::max), 200.0);
        assert_eq!(ys.iter().cloned().fold(f32::MIN, f32::max), 200.0);
        // Neutral height ⇒ world Z is exactly the actor's Location.Z.
        assert!(soup.vertices.iter().all(|v| v[2] == 0.0));
    }

    #[test]
    fn location_places_the_terrain_in_absolute_world_space() {
        let t = terrain(1, 1, vec![NEUTRAL; 4], vec![0; 4], [8000.0, -2000.0, 350.0]);
        let mut soup = TriangleSoup::new(None);
        push_terrain_triangles(&t, &terrain_transform(&t), &mut soup);
        assert!(soup.vertices.contains(&[8000.0, -2000.0, 350.0]));
        assert!(soup.vertices.contains(&[8100.0, -1900.0, 350.0]));
    }

    #[test]
    fn raw_height_is_scaled_by_zscale_then_draw_scale_3d_z() {
        // +128 raw = +1 local unit = +100 cm world at the default Z
        // scale of 100.
        let mut heights = vec![NEUTRAL; 4];
        heights[0] = NEUTRAL + 128;
        let t = terrain(1, 1, heights, vec![0; 4], [0.0; 3]);
        assert_eq!(t.local_vertex(0, 0).unwrap()[2], 128.0 * TERRAIN_ZSCALE);
        let mut soup = TriangleSoup::new(None);
        push_terrain_triangles(&t, &terrain_transform(&t), &mut soup);
        assert!(soup.vertices.contains(&[0.0, 0.0, 100.0]));
    }

    #[test]
    fn a_hole_quad_emits_no_triangles() {
        // 2x2 patches, vertex (0,0) flagged ⇒ quad (0,0) is a hole.
        let n = 9;
        let mut info = vec![0u8; n];
        info[0] = 1;
        let t = terrain(2, 2, vec![NEUTRAL; n], info, [0.0; 3]);
        let mut soup = TriangleSoup::new(None);
        let emitted = push_terrain_triangles(&t, &terrain_transform(&t), &mut soup);
        assert_eq!(emitted, 6, "3 of 4 quads should survive");
        // Nothing may be emitted inside the hole's 0..100 cm footprint.
        for f in &soup.faces {
            let vs = [
                soup.vertices[f[0] as usize - 1],
                soup.vertices[f[1] as usize - 1],
                soup.vertices[f[2] as usize - 1],
            ];
            let cx = vs.iter().map(|v| v[0]).sum::<f32>() / 3.0;
            let cy = vs.iter().map(|v| v[1]).sum::<f32>() / 3.0;
            assert!(
                !(cx < 100.0 && cy < 100.0),
                "triangle centroid ({cx}, {cy}) falls inside the hole quad"
            );
        }
    }

    #[test]
    fn a_fully_holed_terrain_emits_nothing() {
        let n = 9;
        let t = terrain(2, 2, vec![NEUTRAL; n], vec![1; n], [0.0; 3]);
        let mut soup = TriangleSoup::new(None);
        assert_eq!(
            push_terrain_triangles(&t, &terrain_transform(&t), &mut soup),
            0
        );
        assert_eq!(soup.triangle_count(), 0);
    }

    #[test]
    fn explicit_draw_scale_3d_changes_patch_spacing_and_height() {
        let mut t = flat(1, 1);
        t.draw_scale_3d = [100.0, 100.0, 200.0];
        t.heights[3] = NEUTRAL + 128; // vertex (1,1)
        let mut soup = TriangleSoup::new(None);
        push_terrain_triangles(&t, &terrain_transform(&t), &mut soup);
        assert!(soup.vertices.contains(&[100.0, 100.0, 200.0]));
    }
}
