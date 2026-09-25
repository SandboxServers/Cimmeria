//! Phase 1.2 — StaticMesh + StaticMeshActor extraction.
//!
//! For each [`MESH_ACTOR_CLASSES`] export in a chunk `.umap` (the
//! `StaticMeshActor`-shaped family: `StaticMeshActor` itself, plus
//! NA36's `InterpActor` / `KActor` / `FracturedStaticMeshActor`):
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
//! the filesystem. Reference resolution lives in [`mesh_ref`].
//!
//! # Cross-package resolution
//!
//! Resolving an import to a mesh requires a [`PackageIndex`] keyed by
//! `(package_name, object_name)`. Building one walks all `~5000` packages
//! under `CookedPC/` and takes about 45 seconds; the orchestrator caches
//! the result via `PackageIndex::save` / `::load`. When the index is not
//! available (e.g. in CI without the asset bundle), the per-chunk
//! extractor logs the number of actors it could not resolve and returns
//! an empty soup — that's the path the integration test takes when the
//! `Stargate Worlds-QA` directory is missing.
//!
//! # Archetype-based actors
//!
//! In SGW cooked chunks, most `StaticMeshActor` exports carry the
//! `StaticMesh` reference directly on their cooked
//! `StaticMeshComponent`. The rest were instanced from a **prefab
//! archetype** (an import into e.g. `Em-Props.upk:EM-WallLight02_Pf0`);
//! their cooked component is a stub holding only per-instance overrides
//! (`CullDistance`, `IrrelevantLights`, …) and the mesh reference lives
//! on the archetype. [`archetype`] follows that chain; the walker only
//! tallies [`SkipReason::ArchetypeStubComponent`] when no
//! [`PackageIndex`] was supplied to follow it with. On Castle this is
//! 961 of 6,430 actors — see the crate README.

pub mod archetype;
pub mod mesh_ref;

use std::collections::HashMap;
use std::path::Path;

use cimmeria_upk::Package;
use cimmeria_upk_objects::{deserialize_static_mesh, PackageIndex, StaticMesh};

use crate::coverage::{SkipReason, SkipTally};
use crate::geometry::TriangleSoup;
use crate::transform::{transform_triangles, ActorTransform};
use crate::{ExtractError, Result};

pub use archetype::ArchetypeCache;
pub use mesh_ref::transform_from_actor_props;

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
    /// `true` when the actor's export-table `Archetype` field is
    /// non-zero — i.e. it was instanced from a prefab template. Carried
    /// through so the coverage report can say how many *archetype*
    /// actors resolved anyway.
    pub from_archetype: bool,
    /// `true` when the mesh reference came from the archetype chain
    /// rather than from the instance's own `StaticMesh` property.
    ///
    /// Distinct from `from_archetype`: a prefab-instanced actor whose
    /// cooked component *does* carry its own `StaticMesh` (the
    /// instance-overrides-archetype case) is `from_archetype = true`,
    /// `via_archetype = false`.
    pub via_archetype: bool,
}

/// Result of walking a package's `StaticMeshActor` exports.
#[derive(Debug, Default)]
pub struct ActorWalk {
    /// Actors whose mesh reference was recovered.
    pub instances: Vec<StaticMeshInstance>,
    /// Every `StaticMeshActor` export seen, resolvable or not.
    pub actors_total: u64,
    /// Actors that fell out before a mesh reference could be formed.
    pub skips: SkipTally,
    /// Actors with a non-zero export-table `Archetype`.
    pub archetype_actors: u64,
    /// Actors whose `Outer` chain passes through a `PrefabInstance`.
    pub prefab_outer_actors: u64,
    /// Prefab packages this chunk had to open to follow archetypes.
    pub prefab_packages_opened: u64,
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
    /// Always equal to `skips.total()`.
    pub actors_unresolved: usize,
    /// Per-reason breakdown of `actors_unresolved`.
    pub skips: SkipTally,
    /// Total collision triangles emitted across all resolved instances.
    pub triangles_emitted: usize,
    /// Actors with a non-zero export-table `Archetype`.
    pub archetype_actors: u64,
    /// ...of which produced triangles anyway.
    pub archetype_actors_resolved: u64,
    /// Actors whose `Outer` chain passes through a `PrefabInstance`.
    pub prefab_outer_actors: u64,
    /// Actors whose mesh reference came from the archetype chain rather
    /// than their own component, **and** that produced triangles.
    pub actors_resolved_via_archetype: u64,
    /// Triangles contributed by those actors.
    pub triangles_via_archetype: usize,
    /// Prefab packages opened while walking this chunk's archetypes.
    pub prefab_packages_opened: u64,
}

/// Walk every `StaticMeshActor` in a chunk and produce its triangle soup.
///
/// `index` may be `None`, in which case the extractor logs how many
/// actors it would have processed and returns an empty soup. This is the
/// degraded mode used in CI when the cooked asset bundle isn't on the
/// runner.
pub fn extract_chunk(chunk_path: &Path, index: Option<&PackageIndex>) -> Result<ChunkExtraction> {
    let pkg = Package::open(chunk_path)?;
    let mut cache = ArchetypeCache::default();
    Ok(extract_chunk_from_package(&pkg, index, &mut cache))
}

/// [`extract_chunk`] against an already-open package.
///
/// The orchestrator opens each chunk once — for the export-class census
/// *and* the geometry walk — so it calls this rather than paying the LZO
/// decompression twice.
///
/// `cache` memoises archetype-path resolution **across** chunks: Castle's
/// 961 stub actors share only 86 distinct archetype paths, so reusing one
/// cache for the whole map turns 961 prefab-package opens into 86. Pass a
/// fresh [`ArchetypeCache`] if you want per-chunk isolation; the result
/// is identical, just slower.
pub fn extract_chunk_from_package(
    pkg: &Package,
    index: Option<&PackageIndex>,
    cache: &mut ArchetypeCache,
) -> ChunkExtraction {
    let walk = collect_static_mesh_instances(pkg, index, cache);

    let mut result = ChunkExtraction {
        actors_total: walk.actors_total as usize,
        skips: walk.skips,
        archetype_actors: walk.archetype_actors,
        prefab_outer_actors: walk.prefab_outer_actors,
        prefab_packages_opened: walk.prefab_packages_opened,
        ..Default::default()
    };

    // Group instances by mesh reference so we only decode each unique
    // mesh once per chunk — a chunk often has dozens of instances of the
    // same archway / floor tile / wall section.
    let mut by_mesh: HashMap<(String, String), Vec<StaticMeshInstance>> = HashMap::new();
    for inst in walk.instances {
        by_mesh.entry(inst.mesh_ref.clone()).or_default().push(inst);
    }

    // A mesh that is an export of *this* package needs no index: it is
    // in the bytes we already have open. Resolving those first is not
    // an optimisation — routing them through `PackageIndex::find` with
    // an empty package name returns `MeshNotInIndex` and drops the
    // collision geometry on the floor.
    let mut cross_package: Vec<((String, String), Vec<StaticMeshInstance>)> = Vec::new();
    for (mesh_ref, group) in by_mesh {
        if !mesh_ref::is_local(&mesh_ref) {
            cross_package.push((mesh_ref, group));
            continue;
        }
        match load_local_static_mesh(pkg, &mesh_ref.1) {
            Ok(mesh) => emit_group(&mut result, &mesh_ref, &mesh, group),
            Err((e, reason)) => {
                tracing::debug!(
                    object = %mesh_ref.1,
                    error = %e,
                    "could not load package-local StaticMesh"
                );
                result.skips.add_n(reason, group.len() as u64);
            }
        }
    }

    let Some(index) = index else {
        let unresolvable: u64 = cross_package.iter().map(|(_, g)| g.len() as u64).sum();
        tracing::warn!(
            actors_total = walk.actors_total,
            cross_package_instances = unresolvable,
            local_instances = result.actors_resolved,
            "no PackageIndex available; only package-local meshes emitted"
        );
        result.skips.add_n(SkipReason::NoPackageIndex, unresolvable);
        result.actors_unresolved = result.skips.total() as usize;
        return result;
    };

    for (mesh_ref, group) in cross_package {
        let mesh = match load_static_mesh(index, &mesh_ref) {
            Ok(m) => m,
            Err((e, reason)) => {
                tracing::debug!(
                    package = %mesh_ref.0,
                    object = %mesh_ref.1,
                    error = %e,
                    "could not load StaticMesh"
                );
                result.skips.add_n(reason, group.len() as u64);
                continue;
            }
        };
        emit_group(&mut result, &mesh_ref, &mesh, group);
    }

    result.actors_unresolved = result.skips.total() as usize;
    result
}

/// Transform one decoded mesh into world space once per instance and
/// tally the result.
fn emit_group(
    result: &mut ChunkExtraction,
    mesh_ref: &(String, String),
    mesh: &StaticMesh,
    group: Vec<StaticMeshInstance>,
) {
    let local_tris = mesh.collision_triangles();
    if local_tris.is_empty() {
        tracing::debug!(
            package = %mesh_ref.0,
            object = %mesh_ref.1,
            "StaticMesh has no collision triangles"
        );
        result
            .skips
            .add_n(SkipReason::MeshNoCollision, group.len() as u64);
        return;
    }

    for inst in group {
        // Transform mesh-local triangles into world space and push
        // each one directly into the soup. The previous shape allocated
        // an intermediate `Vec` per instance via
        // `transform_triangles(...).collect()` — at ~375 actors per
        // dense chunk that's 375 redundant heap allocations on the
        // hot path.
        for t in &local_tris {
            // `apply_triangle`, not three `apply` calls: a mirrored
            // instance must keep its facing or its floor reads as a
            // ceiling.
            result.soup.push(inst.transform.apply_triangle(*t));
        }
        result.actors_resolved += 1;
        if inst.from_archetype {
            result.archetype_actors_resolved += 1;
        }
        if inst.via_archetype {
            result.actors_resolved_via_archetype += 1;
            result.triangles_via_archetype += local_tris.len();
        }
        result.triangles_emitted += local_tris.len();
    }
}

/// Decode a `StaticMesh` export of the package the reference came from.
///
/// Looked up by object name among the exports whose class is
/// `StaticMesh`: the reference shape
/// ([`mesh_ref::MeshRefResult`]) carries a name, not an export index,
/// and a chunk holds a handful of them at most.
fn load_local_static_mesh(
    pkg: &Package,
    object_name: &str,
) -> std::result::Result<StaticMesh, (ExtractError, SkipReason)> {
    let Some(export) = pkg
        .exports
        .iter()
        .find(|e| e.object_name == object_name && pkg.export_class_name(e) == "StaticMesh")
    else {
        return Err((
            ExtractError::Other(format!(
                "no package-local StaticMesh export named {object_name}"
            )),
            SkipReason::UnresolvableMeshRef,
        ));
    };
    let decode_failed = |e: ExtractError| (e, SkipReason::MeshDecodeFailed);
    let data = pkg
        .read_export_data(export)
        .map_err(|e| decode_failed(e.into()))?;
    deserialize_static_mesh(&data, &pkg.names)
        .map_err(|e| decode_failed(ExtractError::Other(format!("StaticMesh deserialize: {e}"))))
}

/// Open the StaticMesh's home package and run the decoder.
///
/// The error carries the [`SkipReason`] alongside the message so the
/// caller doesn't have to re-classify by string matching.
fn load_static_mesh(
    index: &PackageIndex,
    mesh_ref: &(String, String),
) -> std::result::Result<StaticMesh, (ExtractError, SkipReason)> {
    let Some(loc) = index.find(&mesh_ref.0, &mesh_ref.1) else {
        return Err((
            ExtractError::Other(format!(
                "StaticMesh not in package index: {}.{}",
                mesh_ref.0, mesh_ref.1
            )),
            SkipReason::MeshNotInIndex,
        ));
    };
    let decode_failed = |e: ExtractError| (e, SkipReason::MeshDecodeFailed);
    let pkg = Package::open(&loc.file_path).map_err(|e| decode_failed(e.into()))?;
    let Some(export) = pkg.exports.get(loc.export_index) else {
        return Err((
            ExtractError::Other(format!(
                "PackageIndex returned out-of-range export_index {} for {}.{}",
                loc.export_index, mesh_ref.0, mesh_ref.1
            )),
            SkipReason::MeshNotInIndex,
        ));
    };
    let data = pkg
        .read_export_data(export)
        .map_err(|e| decode_failed(e.into()))?;
    deserialize_static_mesh(&data, &pkg.names)
        .map_err(|e| decode_failed(ExtractError::Other(format!("StaticMesh deserialize: {e}"))))
}

/// Export classes the walker treats as `StaticMeshActor`-shaped: a
/// cooked actor whose tagged-property block carries `Location` /
/// `Rotation` / `DrawScale` / `DrawScale3D` and a `StaticMeshComponent`
/// object reference, optionally gated by `bCollideActors`.
///
/// UE3's `AInterpActor`, `AKActor` and `AFracturedStaticMeshActor` all
/// derive from `AStaticMeshActor` and add no new placement or mesh-ref
/// properties of their own (Matinee-driven movers, rigid-body physics
/// props, and destructible meshes respectively) — so the same resolver
/// walks them unchanged. NA36 (Harset raised-platform navmesh gap):
/// `Harset-*` chunks carry 31 `InterpActor` exports with real collision
/// meshes that a class filter of `StaticMeshActor` alone drops on the
/// floor entirely (not even into `SkipReason` — the walker's `for`
/// loop never visits them), leaving zero geometry under platforms built
/// from movers. `StaticMeshCollectionActor` is a deliberate exclusion:
/// it owns an *array* of `StaticMeshComponent`s rather than one, so it
/// needs its own walk and is tracked as a remaining gap in
/// `coverage::COLLISION_BEARING_CLASSES`.
pub const MESH_ACTOR_CLASSES: &[&str] = &[
    "StaticMeshActor",
    "InterpActor",
    "KActor",
    "FracturedStaticMeshActor",
];

/// Walk every [`MESH_ACTOR_CLASSES`] export in `pkg` and produce one
/// instance per actor whose `StaticMeshComponent.StaticMesh` reference
/// can be recovered from the tagged-property stream.
///
/// Actors with a missing or dangling mesh reference are tallied by
/// reason into [`ActorWalk::skips`]; the sum of `instances.len()` and
/// `skips.total()` always equals `actors_total`.
///
/// `index` is needed only to follow prefab archetypes; with `None` a
/// stub component stays a [`SkipReason::ArchetypeStubComponent`] skip,
/// which is what the degraded CI mode reports.
pub fn collect_static_mesh_instances(
    pkg: &Package,
    index: Option<&PackageIndex>,
    cache: &mut ArchetypeCache,
) -> ActorWalk {
    let mut walk = ActorWalk::default();
    // Prefab packages opened for *this* chunk only. Dropped on return,
    // so peak memory is one chunk's prefab working set rather than every
    // prefab package the map touches; `cache` is what stops that from
    // costing repeat opens.
    let mut open = archetype::OpenPrefabs::default();

    for export in &pkg.exports {
        if !MESH_ACTOR_CLASSES.contains(&pkg.export_class_name(export)) {
            continue;
        }
        walk.actors_total += 1;

        let from_archetype = mesh_ref::archetype_label(pkg, export.archetype).is_some();
        if from_archetype {
            walk.archetype_actors += 1;
        }
        if mesh_ref::has_prefab_instance_outer(pkg, export.package_index) {
            walk.prefab_outer_actors += 1;
        }

        // A cooked actor body shorter than the 32-byte binary prefix has
        // no tagged-property block to read at all.
        if export.serial_size <= 32 {
            walk.skips.add(SkipReason::NoComponentRef);
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
                walk.skips.add(SkipReason::NoComponentRef);
                continue;
            }
        };

        let props = cimmeria_upk::parse_tagged_properties(&data, 32, &pkg.names);

        // The *actor* archetype chain (distinct from the component's —
        // see `archetype`'s module doc) supplies `bCollideActors` and
        // any rotation/scale the instance omits. Castle: 26 of 86
        // archetype actors set `bCollideActors = false`, 17 of them
        // `Group = PrecipPlanes` weather cards sitting in doorways with
        // real kDOP collision. Emitting those splits the exterior
        // navmesh, so this gate runs before anything else.
        let resolution = match index {
            Some(index) if export.archetype != 0 => {
                archetype::resolve_actor_archetype(pkg, export.archetype, index, cache, &mut open)
            }
            // Degraded mode has no index to follow *any* chain with, and
            // emits nothing cross-package anyway; the actor's own
            // properties are all there is to go on.
            _ => archetype::ActorArchetypeResolution::NotInstanced,
        };
        let Some(arch) = resolution.props() else {
            // The chain that says whether this actor collides could not
            // be read. Emitting it would assume UE3's `true` default for
            // a template that may have said `false` — the component
            // chain can still resolve a mesh, so the actor would sail
            // through and put a solid obstacle in the navmesh.
            tracing::debug!(
                actor = %export.object_name,
                reason = ?resolution.skip_reason(),
                "actor archetype unreadable; skipping rather than assuming it collides"
            );
            walk.skips.add(SkipReason::ActorArchetypeUnreadable);
            continue;
        };
        if !arch.collides(&props) {
            walk.skips.add(SkipReason::CollisionDisabled);
            continue;
        }
        let xf = arch.merge_transform(&props);

        // Recover the mesh import via the actor's StaticMeshComponent
        // sub-object. The component's tagged-property block lives at the
        // export pointed to by the actor's `StaticMeshComponent`
        // ObjectProperty.
        let Some(component_ref) = mesh_ref::find_object(&props, "StaticMeshComponent") else {
            tracing::debug!(actor = %export.object_name, "no StaticMeshComponent ref");
            walk.skips.add(SkipReason::NoComponentRef);
            continue;
        };
        let mut via_archetype = false;
        let resolved = match mesh_ref::resolve_mesh_ref_from_component(pkg, component_ref) {
            // The cooked component is a stub: its `StaticMesh` lives on
            // the prefab archetype. Follow the chain when we have an
            // index to locate the archetype's package with.
            Err(SkipReason::ArchetypeStubComponent) => {
                match (index, pkg.exports.get((component_ref - 1) as usize)) {
                    (Some(index), Some(component)) => {
                        via_archetype = true;
                        archetype::resolve_via_archetype(pkg, component, index, cache, &mut open)
                    }
                    _ => Err(SkipReason::ArchetypeStubComponent),
                }
            }
            other => other,
        };
        match resolved {
            Ok(mesh_ref) => walk.instances.push(StaticMeshInstance {
                actor_name: export.object_name.clone(),
                mesh_ref,
                transform: xf,
                from_archetype,
                via_archetype,
            }),
            Err(reason) => walk.skips.add(reason),
        }
    }

    walk.prefab_packages_opened = open.opened() as u64;
    walk
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

#[cfg(test)]
mod archetype_walk_tests;
#[cfg(test)]
mod mesh_actor_class_tests;
#[cfg(test)]
mod tests;
