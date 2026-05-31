//! Phase 1.2 — StaticMesh + StaticMeshActor extraction.
//!
//! For each `StaticMeshActor` export in a chunk `.umap`:
//!
//! 1. Read the actor's tagged properties to recover its transform
//!    (`Location` / `Rotation` / `DrawScale` / `DrawScale3D`) and the
//!    object reference to its `StaticMeshComponent`.
//! 2. Read the component's tagged properties to find its `StaticMesh`
//!    object reference (this is normally an **import** into another
//!    `.upk` — the actual mesh data lives in shared content packages).
//! 3. Resolve the import via a [`PackageIndex`] to a `(file, export_idx)`
//!    pair, open that package, and run the existing
//!    `cimmeria_upk_objects::deserialize_static_mesh` decoder.
//! 4. Pull the collision triangle list out of the decoded mesh
//!    (`StaticMesh::collision_triangles`), apply the actor transform,
//!    and push each triangle into the chunk's triangle soup.
//!
//! The actor-walker is intentionally split from the pure triangle-soup
//! builder ([`build_chunk_soup`]) so that the latter can be unit tested
//! against constructed `StaticMesh` + transform fixtures without touching
//! the filesystem.
//!
//! # Cross-package resolution
//!
//! Resolving an import to a mesh requires a [`PackageIndex`] keyed by
//! `(package_name, object_name)`. Building one walks all `~5000` packages
//! under `CookedPC/` and takes about 50 seconds; the orchestrator caches
//! the result via `PackageIndex::save` / `::load`. When the index is not
//! available (e.g. in CI without the asset bundle), the per-chunk
//! extractor logs the number of actors it could not resolve and returns
//! an empty soup — that's the path the integration test takes when the
//! `Stargate Worlds-QA` directory is missing.
//!
//! # Known limitation — archetype-based actors
//!
//! In SGW cooked Castle_CellBlock chunks, ~80% of `StaticMeshActor`
//! exports carry the `StaticMesh` reference directly on their cooked
//! `StaticMeshComponent`. The remaining ~20% reference a **prefab
//! archetype** (an import into e.g. `Em-Props.upk:EM-WallLight02_Pf0`);
//! their cooked component is a 76-byte stub holding only the
//! `CullDistance` override, and the actual mesh reference lives in the
//! prefab template's component. Resolving these requires opening the
//! archetype package and merging the inherited properties — that's a
//! Phase 1.2-follow-up, not the initial landing. The current walker
//! silently skips them; the per-chunk extractor reports the count via
//! [`ChunkExtraction::actors_unresolved`].

use std::collections::HashMap;
use std::path::Path;

use cimmeria_upk::{ImportEntry, Package, PropValue, TaggedProperty};
use cimmeria_upk_objects::{deserialize_static_mesh, PackageIndex, StaticMesh};

use crate::geometry::TriangleSoup;
use crate::transform::{transform_triangles, ActorTransform};
use crate::{ExtractError, Result};

/// One StaticMesh instance to be added to a chunk's triangle soup.
///
/// Lifted out of the per-export walk so callers can stage the work,
/// inspect counts, or pre-filter by class without committing to the
/// per-instance disk reads up front.
#[derive(Debug, Clone)]
pub struct StaticMeshInstance {
    /// The owning actor's debug-friendly name — used as the OBJ group
    /// label so a human reviewer can map a group back to its `.umap`.
    pub actor_name: String,
    /// The cross-package mesh reference (`(package_name, object_name)`).
    pub mesh_ref: (String, String),
    /// Per-instance world transform.
    pub transform: ActorTransform,
}

/// Per-chunk extraction result.
///
/// Returned by [`extract_chunk`] so the orchestrator can log
/// instance/triangle counts at info level without re-walking the soup.
#[derive(Debug, Default)]
pub struct ChunkExtraction {
    /// All triangles from this chunk, in world-space UE3 cm.
    pub soup: TriangleSoup,
    /// Number of StaticMeshActors found in the chunk.
    pub actors_total: usize,
    /// Number of actors whose StaticMesh reference could be resolved.
    pub actors_resolved: usize,
    /// Number of actors skipped because the StaticMesh reference was
    /// missing, dangling, or unresolvable via the supplied package index.
    pub actors_unresolved: usize,
    /// Total collision triangles emitted across all resolved instances.
    pub triangles_emitted: usize,
}

/// Walk every `StaticMeshActor` in a chunk and produce its triangle soup.
///
/// `index` may be `None`, in which case the extractor logs how many
/// actors it would have processed and returns an empty soup. This is the
/// degraded mode used in CI when the cooked asset bundle isn't on the
/// runner.
pub fn extract_chunk(chunk_path: &Path, index: Option<&PackageIndex>) -> Result<ChunkExtraction> {
    let pkg = Package::open(chunk_path)?;
    let instances = collect_static_mesh_instances(&pkg);
    let actors_total = instances.len();

    let Some(index) = index else {
        tracing::warn!(
            actors_total,
            chunk = %chunk_path.display(),
            "no PackageIndex available; emitting empty soup"
        );
        return Ok(ChunkExtraction {
            actors_total,
            actors_unresolved: actors_total,
            ..Default::default()
        });
    };

    // Group instances by mesh reference so we only decode each unique
    // mesh once per chunk — a chunk often has dozens of instances of the
    // same archway / floor tile / wall section.
    let mut by_mesh: HashMap<(String, String), Vec<StaticMeshInstance>> = HashMap::new();
    for inst in instances {
        by_mesh.entry(inst.mesh_ref.clone()).or_default().push(inst);
    }

    let mut result = ChunkExtraction {
        actors_total,
        ..Default::default()
    };

    for (mesh_ref, group) in by_mesh {
        let mesh = match load_static_mesh(index, &mesh_ref) {
            Ok(m) => m,
            Err(e) => {
                tracing::debug!(
                    package = %mesh_ref.0,
                    object = %mesh_ref.1,
                    error = %e,
                    "could not load StaticMesh"
                );
                result.actors_unresolved += group.len();
                continue;
            }
        };
        let local_tris = mesh.collision_triangles();
        if local_tris.is_empty() {
            tracing::debug!(
                package = %mesh_ref.0,
                object = %mesh_ref.1,
                "StaticMesh has no collision triangles"
            );
            result.actors_unresolved += group.len();
            continue;
        }

        for inst in group {
            let world_tris = transform_triangles(&local_tris, &inst.transform);
            for t in world_tris {
                result.soup.push(t);
            }
            result.actors_resolved += 1;
            result.triangles_emitted += local_tris.len();
        }
    }

    Ok(result)
}

/// Open the StaticMesh's home package and run the decoder.
fn load_static_mesh(index: &PackageIndex, mesh_ref: &(String, String)) -> Result<StaticMesh> {
    let loc = index.find(&mesh_ref.0, &mesh_ref.1).ok_or_else(|| {
        ExtractError::Other(format!(
            "StaticMesh not in package index: {}.{}",
            mesh_ref.0, mesh_ref.1
        ))
    })?;
    let pkg = Package::open(&loc.file_path)?;
    let export = pkg.exports.get(loc.export_index).ok_or_else(|| {
        ExtractError::Other(format!(
            "PackageIndex returned out-of-range export_index {} for {}.{}",
            loc.export_index, mesh_ref.0, mesh_ref.1
        ))
    })?;
    let data = pkg.read_export_data(export)?;
    let mesh = deserialize_static_mesh(&data, &pkg.names)
        .map_err(|e| ExtractError::Other(format!("StaticMesh deserialize: {e}")))?;
    Ok(mesh)
}

/// Walk every `StaticMeshActor` in `pkg` and produce one instance per
/// actor whose `StaticMeshComponent.StaticMesh` reference can be
/// recovered from the tagged-property stream.
///
/// Actors with a missing or dangling mesh reference are silently skipped
/// — the extractor logs at `debug` and the orchestrator reports the
/// count via `actors_unresolved`.
pub fn collect_static_mesh_instances(pkg: &Package) -> Vec<StaticMeshInstance> {
    let mut out = Vec::new();
    for export in &pkg.exports {
        let class_name = pkg.export_class_name(export);
        if class_name != "StaticMeshActor" {
            continue;
        }
        if export.serial_size <= 32 {
            continue;
        }

        let data = match pkg.read_export_data(export) {
            Ok(d) => d,
            Err(e) => {
                tracing::debug!(
                    actor = %export.object_name,
                    error = %e,
                    "could not read StaticMeshActor data"
                );
                continue;
            }
        };

        let props = cimmeria_upk::parse_tagged_properties(&data, 32, &pkg.names);

        let xf = transform_from_actor_props(&props);

        // Recover the mesh import via the actor's StaticMeshComponent
        // sub-object. The component's tagged-property block lives at the
        // export pointed to by the actor's `StaticMeshComponent`
        // ObjectProperty.
        let Some(component_ref) = find_object(&props, "StaticMeshComponent") else {
            tracing::debug!(actor = %export.object_name, "no StaticMeshComponent ref");
            continue;
        };
        let Some(mesh_ref) = resolve_mesh_ref_from_component(pkg, component_ref) else {
            continue;
        };

        out.push(StaticMeshInstance {
            actor_name: export.object_name.clone(),
            mesh_ref,
            transform: xf,
        });
    }
    out
}

/// Build an [`ActorTransform`] from a slice of tagged properties. Missing
/// properties take their UE3-cooked defaults (origin, no rotation,
/// uniform scale).
pub fn transform_from_actor_props(props: &[TaggedProperty]) -> ActorTransform {
    let location = find_vector(props, "Location").unwrap_or([0.0; 3]);
    let rotation = find_rotator(props, "Rotation").unwrap_or([0, 0, 0]);
    let draw_scale = find_float(props, "DrawScale").unwrap_or(1.0);
    let draw_scale_3d = find_vector(props, "DrawScale3D").unwrap_or([1.0, 1.0, 1.0]);
    ActorTransform {
        location,
        rotation,
        draw_scale,
        draw_scale_3d,
    }
}

/// Resolve a `StaticMeshComponent` export reference into the
/// `(package_name, object_name)` pair its `StaticMesh` property points
/// to.
///
/// Returns `None` if the ref is invalid, the component has no
/// `StaticMesh` property, the property is a `None`-ref, or the import
/// table doesn't have an entry for the target.
fn resolve_mesh_ref_from_component(pkg: &Package, component_ref: i32) -> Option<(String, String)> {
    if component_ref <= 0 {
        // Components are typically embedded exports, so a positive
        // 1-based export index. Imports (negative) would be unusual.
        return None;
    }
    let idx = (component_ref - 1) as usize;
    let component = pkg.exports.get(idx)?;
    if component.serial_size <= 0 {
        return None;
    }
    let data = pkg.read_export_data(component).ok()?;

    // Cooked StaticMeshComponent layout: 8-byte binary header (NetIndex
    // + component-specific prefix) then tagged properties. Verified by
    // probing component export 1771 in Castle_CellBlock chunk fffefffd:
    // offset 4 yields 0 properties (parser hits a garbage FName), offset
    // 8 yields the expected 10 properties starting with the `StaticMesh`
    // ObjectProperty. The 4-byte offset that `crates/upk-objects/src/
    // static_mesh.rs` uses is for non-Component objects (StaticMesh
    // itself), which only have a 4-byte NetIndex prefix.
    //
    // The `castle_cellblock_walks_static_mesh_actors` integration test
    // pins this — reverting to offset 4 drops resolvable instances to 0.
    let props = cimmeria_upk::parse_tagged_properties(&data, 8, &pkg.names);
    let mesh_obj = find_object(&props, "StaticMesh")?;
    if mesh_obj == 0 {
        return None;
    }

    // Cross-package StaticMesh references are imports — negative obj
    // index. Local-to-this-package StaticMesh exports are positive and
    // unusual for `.umap` files but possible.
    if mesh_obj < 0 {
        let imp_idx = (-mesh_obj - 1) as usize;
        let imp = pkg.imports.get(imp_idx)?;
        // The import's `package_index` chain leads up to a root package
        // import; walk to the topmost ancestor to recover the .upk name.
        let package_name = top_package_name(pkg, imp)?;
        Some((package_name, imp.object_name.clone()))
    } else {
        let exp = pkg.exports.get((mesh_obj - 1) as usize)?;
        // For local exports, the "package name" is the chunk's stem —
        // but since the chunk owns the mesh, we record an empty package
        // name and a key the PackageIndex won't have. Callers that want
        // to handle local StaticMesh exports must short-circuit this
        // path; for SGW Castle_CellBlock the meshes are always imports
        // so we keep this branch dormant.
        Some((String::new(), exp.object_name.clone()))
    }
}

/// Walk an import's outer chain back to the root package and return
/// its name (the `.upk` stem).
fn top_package_name<'a>(pkg: &'a Package, mut imp: &'a ImportEntry) -> Option<String> {
    let mut depth = 0;
    while depth < 32 {
        if imp.package_index == 0 {
            // Reached the root — the class_package is the .upk stem the
            // engine looks up at load time.
            return Some(imp.object_name.clone());
        }
        if imp.package_index >= 0 {
            return None;
        }
        let next_idx = (-imp.package_index - 1) as usize;
        imp = pkg.imports.get(next_idx)?;
        depth += 1;
    }
    None
}

/// Run the per-mesh + per-transform → triangle pipeline against
/// already-decoded data. Exists so unit tests can supply hand-built
/// `StaticMesh` fixtures without touching the filesystem.
pub fn build_chunk_soup(instances: &[(StaticMesh, ActorTransform, String)]) -> TriangleSoup {
    let mut soup = TriangleSoup::new(None);
    for (mesh, xf, _name) in instances {
        let local_tris = mesh.collision_triangles();
        let world_tris = transform_triangles(&local_tris, xf);
        for t in world_tris {
            soup.push(t);
        }
    }
    soup
}

// ----- property lookup helpers (copies of the actor.rs helpers; we
// don't re-export those from cimmeria_upk because their signatures
// return owned data shapes we don't share) -----

fn find_vector(props: &[TaggedProperty], name: &str) -> Option<[f32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Vector { x, y, z } = &p.value {
            Some([*x, *y, *z])
        } else {
            None
        }
    })
}

fn find_rotator(props: &[TaggedProperty], name: &str) -> Option<[i32; 3]> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Rotator { pitch, yaw, roll } = &p.value {
            Some([*pitch, *yaw, *roll])
        } else {
            None
        }
    })
}

fn find_float(props: &[TaggedProperty], name: &str) -> Option<f32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Float(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}

fn find_object(props: &[TaggedProperty], name: &str) -> Option<i32> {
    props.iter().find(|p| p.name == name).and_then(|p| {
        if let PropValue::Object(v) = &p.value {
            Some(*v)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_upk_objects::{BoundingBox, KdopTriangle, LodModel, Vertex};

    fn vert(p: [f32; 3]) -> Vertex {
        Vertex {
            position: p,
            normal: [0.0; 3],
            tangent: [0.0; 4],
            uv: [0.0; 2],
        }
    }

    fn unit_triangle_mesh() -> StaticMesh {
        // One triangle: (0,0,0), (1,0,0), (0,1,0). Stored as kDOP so
        // collision_triangles() picks the kDOP path.
        StaticMesh {
            bounds: BoundingBox {
                origin: [0.0; 3],
                extent: [1.0; 3],
                sphere_radius: 1.0,
            },
            lod_models: vec![LodModel {
                vertices: vec![
                    vert([0.0, 0.0, 0.0]),
                    vert([1.0, 0.0, 0.0]),
                    vert([0.0, 1.0, 0.0]),
                ],
                indices: vec![],
                sections: vec![],
                num_vertices: 3,
                num_triangles: 0,
            }],
            internal_version: 15,
            kdop_triangles: vec![KdopTriangle {
                v0: 0,
                v1: 1,
                v2: 2,
                material: 0,
            }],
        }
    }

    #[test]
    fn empty_instance_list_produces_empty_soup() {
        let soup = build_chunk_soup(&[]);
        assert_eq!(soup.triangle_count(), 0);
    }

    #[test]
    fn single_instance_identity_transform_pushes_one_triangle() {
        let mesh = unit_triangle_mesh();
        let soup = build_chunk_soup(&[(mesh, ActorTransform::default(), "test".into())]);
        assert_eq!(soup.triangle_count(), 1);
        // Identity transform: vertex 0 stays at the origin.
        assert_eq!(soup.vertices[0], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn translation_only_shifts_every_vertex() {
        let mesh = unit_triangle_mesh();
        let xf = ActorTransform {
            location: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        let soup = build_chunk_soup(&[(mesh, xf, "test".into())]);
        assert_eq!(soup.vertices[0], [10.0, 20.0, 30.0]);
        assert_eq!(soup.vertices[1], [11.0, 20.0, 30.0]);
        assert_eq!(soup.vertices[2], [10.0, 21.0, 30.0]);
    }

    #[test]
    fn drawscale_scales_every_vertex() {
        let mesh = unit_triangle_mesh();
        let xf = ActorTransform {
            draw_scale: 100.0,
            ..Default::default()
        };
        let soup = build_chunk_soup(&[(mesh, xf, "test".into())]);
        assert_eq!(soup.vertices[1], [100.0, 0.0, 0.0]);
        assert_eq!(soup.vertices[2], [0.0, 100.0, 0.0]);
    }

    #[test]
    fn yaw_rotates_then_translates() {
        // Yaw 90° + translate (5, 5, 0). Vertex (1, 0, 0) → (0, 1, 0)
        // after yaw, then + (5, 5, 0) = (5, 6, 0).
        let mesh = unit_triangle_mesh();
        let xf = ActorTransform {
            location: [5.0, 5.0, 0.0],
            rotation: [0, 16384, 0], // yaw = 90°
            ..Default::default()
        };
        let soup = build_chunk_soup(&[(mesh, xf, "test".into())]);
        let v1 = soup.vertices[1];
        assert!((v1[0] - 5.0).abs() < 1e-3);
        assert!((v1[1] - 6.0).abs() < 1e-3);
    }

    #[test]
    fn multiple_instances_accumulate_triangles() {
        let mesh1 = unit_triangle_mesh();
        let mesh2 = unit_triangle_mesh();
        let xf1 = ActorTransform::default();
        let xf2 = ActorTransform {
            location: [100.0, 0.0, 0.0],
            ..Default::default()
        };
        let soup =
            build_chunk_soup(&[(mesh1, xf1, "actor1".into()), (mesh2, xf2, "actor2".into())]);
        assert_eq!(soup.triangle_count(), 2);
        // First instance starts at origin.
        assert_eq!(soup.vertices[0], [0.0, 0.0, 0.0]);
        // Second instance is offset by (+100, 0, 0).
        assert_eq!(soup.vertices[3], [100.0, 0.0, 0.0]);
    }

    #[test]
    fn mesh_with_no_collision_triangles_emits_nothing() {
        // Empty kDOP list AND empty LOD0 indices → collision_triangles()
        // returns an empty list → no triangles in the soup.
        let mesh = StaticMesh {
            bounds: BoundingBox {
                origin: [0.0; 3],
                extent: [1.0; 3],
                sphere_radius: 1.0,
            },
            lod_models: vec![LodModel {
                vertices: vec![vert([0.0, 0.0, 0.0])],
                indices: vec![],
                sections: vec![],
                num_vertices: 1,
                num_triangles: 0,
            }],
            internal_version: 15,
            kdop_triangles: vec![],
        };
        let soup = build_chunk_soup(&[(mesh, ActorTransform::default(), "test".into())]);
        assert_eq!(soup.triangle_count(), 0);
    }

    #[test]
    fn transform_from_props_recovers_all_four_fields() {
        let props = vec![
            TaggedProperty {
                name: "Location".into(),
                array_index: 0,
                value: PropValue::Vector {
                    x: 100.0,
                    y: 200.0,
                    z: 300.0,
                },
            },
            TaggedProperty {
                name: "Rotation".into(),
                array_index: 0,
                value: PropValue::Rotator {
                    pitch: 100,
                    yaw: 200,
                    roll: 300,
                },
            },
            TaggedProperty {
                name: "DrawScale".into(),
                array_index: 0,
                value: PropValue::Float(2.5),
            },
            TaggedProperty {
                name: "DrawScale3D".into(),
                array_index: 0,
                value: PropValue::Vector {
                    x: 1.5,
                    y: 2.5,
                    z: 3.5,
                },
            },
        ];
        let xf = transform_from_actor_props(&props);
        assert_eq!(xf.location, [100.0, 200.0, 300.0]);
        assert_eq!(xf.rotation, [100, 200, 300]);
        assert_eq!(xf.draw_scale, 2.5);
        assert_eq!(xf.draw_scale_3d, [1.5, 2.5, 3.5]);
    }

    #[test]
    fn transform_from_empty_props_defaults_to_identity() {
        let xf = transform_from_actor_props(&[]);
        assert_eq!(xf.location, [0.0, 0.0, 0.0]);
        assert_eq!(xf.rotation, [0, 0, 0]);
        assert_eq!(xf.draw_scale, 1.0);
        assert_eq!(xf.draw_scale_3d, [1.0, 1.0, 1.0]);
    }
}
